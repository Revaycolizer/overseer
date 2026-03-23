use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow, PartialEq)]
pub struct AgentInfo {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,
    pub instructions: String,
    pub model: String,
    pub domain: String,
    pub version: String,
    pub code: Option<String>,
    pub source_files: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow, PartialEq)]
pub struct ToolInfo {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub code: String,
    pub language: String,
    pub script_path: Option<String>,
    pub required_intent: Option<String>,
    pub safety_level: String,
    pub version: String,
}

pub async fn get_agents(db: &crate::db::Database) -> Vec<AgentInfo> {
    crate::backend::get_agents(db).await.unwrap_or_default()
}

pub async fn get_tools(db: &crate::db::Database) -> Vec<ToolInfo> {
    crate::backend::get_tools(db).await.unwrap_or_default()
}

pub fn project_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn app_root_candidates() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(current_dir) = std::env::current_dir() {
        roots.push(current_dir);
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            let exe_dir = exe_dir.to_path_buf();
            if !roots.contains(&exe_dir) {
                roots.push(exe_dir);
            }
        }
    }

    roots
}

pub fn find_existing_relative_path(relative: &str) -> Option<PathBuf> {
    let relative = PathBuf::from(relative);
    if relative.is_absolute() {
        return relative.exists().then_some(relative);
    }

    app_root_candidates()
        .into_iter()
        .map(|root| root.join(&relative))
        .find(|candidate| candidate.exists())
}

pub fn config_agents_dir() -> Option<PathBuf> {
    find_existing_relative_path("config/agents").filter(|path| path.is_dir())
}

pub fn logical_script_root(path: &Path) -> &'static str {
    let normalized = path.to_string_lossy().replace('\\', "/");
    if normalized.ends_with("/tool_scripts") || normalized == "tool_scripts" {
        "tool_scripts"
    } else if normalized.ends_with("/src/engine/scripts") || normalized == "src/engine/scripts" {
        "src/engine/scripts"
    } else if normalized.ends_with("/agents-scripts") || normalized == "agents-scripts" {
        "agents-scripts"
    } else {
        "tool_scripts"
    }
}

pub fn candidate_tool_script_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    for relative in ["tool_scripts", "src/engine/scripts", "agents-scripts"] {
        if let Some(path) = find_existing_relative_path(relative) {
            if path.is_dir() && !roots.contains(&path) {
                roots.push(path);
            }
        }
    }

    roots
}

pub fn runtime_dir() -> PathBuf {
    std::env::temp_dir().join("overseer_runtime")
}

fn resolve_script_path(raw_path: &str) -> Option<PathBuf> {
    let normalized = raw_path.replace('\\', "/");
    let path = PathBuf::from(&normalized);
    if path.is_absolute() {
        return path.exists().then_some(path);
    }

    let mut candidates = Vec::new();
    if let Some(rest) = normalized
        .strip_prefix("agents-scripts/")
        .or_else(|| normalized.strip_prefix("src/engine/scripts/"))
        .or_else(|| normalized.strip_prefix("tool_scripts/"))
    {
        for root in candidate_tool_script_roots() {
            candidates.push(root.join(rest));
        }
    }

    for root in app_root_candidates() {
        candidates.push(root.join(&normalized));
    }

    candidates.into_iter().find(|candidate| candidate.exists())
}

fn normalize_script_path(raw_path: &str) -> PathBuf {
    resolve_script_path(raw_path).unwrap_or_else(|| {
        let normalized = raw_path.replace('\\', "/");
        let path = PathBuf::from(&normalized);
        if path.is_absolute() {
            path
        } else {
            project_root().join(path)
        }
    })
}

fn python_command_candidates() -> Vec<Vec<String>> {
    let mut candidates = Vec::new();

    if let Ok(explicit) = std::env::var("OVERSEER_PYTHON") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            candidates.push(vec![trimmed.to_string()]);
        }
    }

    if cfg!(windows) {
        candidates.push(vec!["python".to_string()]);
        candidates.push(vec!["py".to_string(), "-3".to_string()]);
        candidates.push(vec!["py".to_string()]);
        candidates.push(vec!["python3".to_string()]);
    } else {
        candidates.push(vec!["python3".to_string()]);
        candidates.push(vec!["python".to_string()]);
    }

    candidates
}

pub fn detect_python_command() -> Option<Vec<String>> {
    for candidate in python_command_candidates() {
        let mut probe = Command::new(&candidate[0]);
        if candidate.len() > 1 {
            probe.args(&candidate[1..]);
        }
        probe.arg("--version");

        if let Ok(output) = probe.output() {
            if output.status.success() {
                return Some(candidate);
            }
        }
    }

    None
}

pub fn python_command_label() -> String {
    detect_python_command()
        .map(|parts| parts.join(" "))
        .unwrap_or_else(|| "python (not found)".to_string())
}

pub fn build_python_command() -> Result<Vec<String>, String> {
    detect_python_command().ok_or_else(|| {
        "No working Python interpreter found. Set OVERSEER_PYTHON or install Python 3.".to_string()
    })
}

/// Prepares a dedicated runtime directory with all DB-backed Python tools.
/// This allows Python scripts to reference sibling files relatively.
pub async fn prepare_runtime_env(db: &crate::db::Database) -> anyhow::Result<String> {
    let runtime_dir = runtime_dir();
    std::fs::create_dir_all(&runtime_dir)?;
    stage_requests_compat_module(&runtime_dir)?;
    stage_bs4_compat_module(&runtime_dir)?;

    let tools = get_tools(db).await;
    for tool in tools {
        if tool.language != "python" {
            continue;
        }

        let file_path = runtime_dir.join(format!("{}{}", tool.name, tool_extension(&tool.name)));

        let source = if !tool.code.trim().is_empty() {
            tool.code.clone()
        } else if let Some(script_path) = tool.script_path.as_deref() {
            let resolved = normalize_script_path(script_path);
            if resolved.exists() {
                std::fs::read_to_string(resolved).unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if source.is_empty() {
            println!(
                "⚠️  Skipping runtime staging for '{}' because no Python source was available.",
                tool.name
            );
            continue;
        }

        if let Err(e) = std::fs::write(&file_path, source) {
            println!("⚠️  Failed to stage tool '{}' to runtime: {}", tool.name, e);
        }
    }

    Ok(runtime_dir.to_string_lossy().to_string())
}

fn stage_requests_compat_module(runtime_dir: &Path) -> anyhow::Result<()> {
    let shim_path = runtime_dir.join("requests.py");
    let shim = r#"import json
import urllib.parse
import urllib.request
from urllib.error import HTTPError, URLError


class RequestException(Exception):
    pass


class HTTPStatusError(RequestException):
    pass


class Response:
    def __init__(self, body, status_code, headers, url):
        self._body = body
        self.status_code = status_code
        self.headers = headers
        self.url = url
        self.text = body.decode("utf-8", errors="replace")
        self.content = body

    def json(self):
        return json.loads(self.text)

    def raise_for_status(self):
        if self.status_code >= 400:
            raise HTTPStatusError(f"HTTP {self.status_code} for {self.url}")


def _encode_data(data):
    if data is None:
        return None
    if isinstance(data, (bytes, bytearray)):
        return data
    if isinstance(data, str):
        return data.encode("utf-8")
    if isinstance(data, dict):
        return urllib.parse.urlencode(data).encode("utf-8")
    return str(data).encode("utf-8")


def request(method, url, data=None, headers=None, timeout=30):
    req = urllib.request.Request(
        url,
        data=_encode_data(data),
        headers=headers or {},
        method=method.upper(),
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return Response(resp.read(), getattr(resp, "status", resp.getcode()), dict(resp.headers), resp.geturl())
    except HTTPError as exc:
        body = exc.read()
        return Response(body, exc.code, dict(exc.headers or {}), exc.geturl())
    except URLError as exc:
        raise RequestException(str(exc)) from exc


def get(url, headers=None, timeout=30):
    return request("GET", url, headers=headers, timeout=timeout)


def post(url, data=None, headers=None, timeout=30):
    return request("POST", url, data=data, headers=headers, timeout=timeout)
"#;

    std::fs::write(shim_path, shim)?;
    Ok(())
}

fn stage_bs4_compat_module(runtime_dir: &Path) -> anyhow::Result<()> {
    let package_dir = runtime_dir.join("bs4");
    std::fs::create_dir_all(&package_dir)?;
    let shim_path = package_dir.join("__init__.py");
    let shim = r#"from html.parser import HTMLParser


class _Node:
    def __init__(self, name=None, attrs=None, parent=None, is_text=False, text=""):
        self.name = name
        self.attrs = dict(attrs or [])
        self.parent = parent
        self.children = []
        self.is_text = is_text
        self.text = text
        self.removed = False

    def append(self, child):
        self.children.append(child)

    def get(self, key, default=None):
        return self.attrs.get(key, default)

    def _iter_descendants(self):
        for child in self.children:
            if child.removed:
                continue
            yield child
            yield from child._iter_descendants()

    def _class_list(self):
        classes = self.attrs.get("class", "")
        if isinstance(classes, str):
            return [part for part in classes.split() if part]
        return list(classes)

    def matches(self, selector):
        if self.removed or self.is_text:
            return False
        if selector.startswith("."):
            return selector[1:] in self._class_list()
        return self.name == selector

    def find_all(self, selector):
        selectors = selector if isinstance(selector, (list, tuple, set)) else [selector]
        matches = []
        for node in self._iter_descendants():
            if any(node.matches(item) for item in selectors):
                matches.append(Tag(node))
        return matches

    def select(self, selector):
        return self.find_all(selector)

    def select_one(self, selector):
        found = self.select(selector)
        return found[0] if found else None

    def get_text(self, separator="", strip=False):
        parts = []
        for child in self.children:
            if child.removed:
                continue
            if child.is_text:
                parts.append(child.text)
            else:
                nested = child.get_text(separator=separator, strip=strip)
                if nested:
                    parts.append(nested)
        text = separator.join(part for part in parts if part)
        return text.strip() if strip else text

    def decompose(self):
        self.removed = True
        self.children = []


class Tag:
    def __init__(self, node):
        self._node = node

    def __call__(self, selector):
        return self._node.find_all(selector)

    def select(self, selector):
        return self._node.select(selector)

    def select_one(self, selector):
        return self._node.select_one(selector)

    def get_text(self, separator="", strip=False):
        return self._node.get_text(separator=separator, strip=strip)

    def get(self, key, default=None):
        return self._node.get(key, default)

    def decompose(self):
        self._node.decompose()


class _SoupParser(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.root = _Node(name="[document]")
        self.stack = [self.root]

    def handle_starttag(self, tag, attrs):
        node = _Node(name=tag, attrs=attrs, parent=self.stack[-1])
        self.stack[-1].append(node)
        self.stack.append(node)

    def handle_endtag(self, tag):
        while len(self.stack) > 1:
            node = self.stack.pop()
            if node.name == tag:
                break

    def handle_startendtag(self, tag, attrs):
        node = _Node(name=tag, attrs=attrs, parent=self.stack[-1])
        self.stack[-1].append(node)

    def handle_data(self, data):
        if not data:
            return
        self.stack[-1].append(_Node(parent=self.stack[-1], is_text=True, text=data))


class BeautifulSoup(Tag):
    def __init__(self, markup, parser="html.parser"):
        parser_impl = _SoupParser()
        parser_impl.feed(markup or "")
        parser_impl.close()
        super().__init__(parser_impl.root)
"#;

    std::fs::write(shim_path, shim)?;
    Ok(())
}

fn tool_extension(tool_name: &str) -> &'static str {
    if tool_name.contains('.') {
        ""
    } else {
        ".py"
    }
}

pub fn resolve_tool_script_path(tool: &ToolInfo) -> Option<PathBuf> {
    if tool.language != "python" {
        return tool.script_path.as_deref().map(normalize_script_path);
    }

    let staged = runtime_dir().join(format!("{}{}", tool.name, tool_extension(&tool.name)));
    if staged.exists() {
        return Some(staged);
    }

    tool.script_path
        .as_deref()
        .map(normalize_script_path)
        .filter(|path| path.exists())
}

/// Helper to get a path to an executable script for a tool.
pub fn get_tool_executable_path(tool: &ToolInfo) -> Option<String> {
    resolve_tool_script_path(tool).map(|path| path.to_string_lossy().to_string())
}

pub async fn print_system_info(db: &crate::db::Database) {
    let agents = get_agents(db).await;
    let tools = get_tools(db).await;

    println!("\n🚀 RIG AGENT SYSTEM CONFIGURATION");
    println!("=================================");
    println!("Total Agents: {}", agents.len());
    println!("Total Tools:  {}", tools.len());
    println!("Runtime Dir:  {}", runtime_dir().display());
    println!("Runtime:      native core tools");

    println!("\n🤖 AVAILABLE AGENTS");
    println!("--------------------");
    for agent in &agents {
        println!("● {:<20} | {}", agent.name, agent.description);
        if !agent.tools.is_empty() {
            println!("  └── Equipped Tools: {}", agent.tools.join(", "));
        }
        println!();
    }

    println!("🛠️  SYSTEM TOOLS & USAGE");
    println!("-----------------------");

    let mut tool_to_agents: HashMap<String, Vec<String>> = HashMap::new();
    for agent in &agents {
        for tool in &agent.tools {
            tool_to_agents
                .entry(tool.clone())
                .or_insert_with(Vec::new)
                .push(agent.name.clone());
        }
    }

    for tool in &tools {
        println!("⚙️  {:<24}", tool.name);
        println!("   Description: {}", tool.description);
        println!("   Safety Level: {}", tool.safety_level);
        if let Some(path) = resolve_tool_script_path(tool) {
            println!("   Runtime Path: {}", path.display());
        }
        if let Some(users) = tool_to_agents.get(&tool.name) {
            println!("   Utilized By: {}", users.join(", "));
        } else {
            println!("   Utilized By: (Infrastructure / Internal)");
        }
        println!();
    }
}
