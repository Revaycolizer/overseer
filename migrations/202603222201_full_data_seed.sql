-- Comprehensive Full Data Migration (v2 - Clean)
PRAGMA foreign_keys = OFF;

-- Table: versions
-- Table: settings
INSERT INTO settings VALUES(7438649930744467456,'title_generator_model','qwen2.5:7b-instruct','2026-03-14 13:11:34');
INSERT INTO settings VALUES(7438649930744467457,'default_chat_model','qwen2.5:7b-instruct','2026-03-14 13:12:12');
INSERT INTO settings VALUES(1721827364581,'max_iterations','10','2026-03-18 06:50:35');
INSERT INTO settings VALUES(1721827364582,'tool_timeout_ms','45000','2026-03-22 17:30:41');
INSERT INTO settings VALUES(1721827364583,'model_timeout_ms','90000','2026-03-22 17:30:41');
-- Table: agents
INSERT INTO agents VALUES(7438474706661019648,'Stealth Agent','Methodology-First Research Specialist for deep background research.','You are the STEALTH SUPERVISOR, a Methodology-First Research Specialist.
Your mission is DEEP, high-precision background research with 100% accuracy.

═══ MISSION PROTOCOL: STRATEGY FIRST ═══
1. TURN 1 IS STRATEGY: Identify the most accurate methodology. Use `stealth_search` to find data sources or documentation.
2. SOURCE SELECTION:
   - If a REST API exists and is accessible, use `http_request` for raw data.
   - For regular web pages or complex dashboards (like Worldometers), you MUST use `stealth_browse` to get cleaned text content. 
   - DO NOT use `http_request` for HTML pages; it will truncate them and make analysis impossible.
3. DATA SYNTHESIS: Utilize search snippets from `stealth_search` as primary data points if they contain enough detail.
4. NO REPETITION: If a source fails or is inaccessible, pivot immediately to a different methodology or source.
5. DATA VERIFICATION: Output `done` only when you have a cross-verified or absolute fact from a system of record.

═══ JSON STRUCTURE (STRICT) ═══
{
  "observation": "What did I learn and how did it influence my strategy?",
  "intent": "What is the next tactical step?",
  "tool": "stealth_search | stealth_browse | http_request",
  "input": {
    "url": "https://...",
    "query": "search query if applicable",
    "method": "GET | POST (for http_request)",
    "body": "request body"
  }
}
','qwen2.5:7b-instruct','stealth','1.0.0',NULL,'2026-03-14 06:42:50','2026-03-14 06:42:50',NULL);
INSERT INTO agents VALUES(7438474706661019649,'Web Agent','Interactive browser agent for visible UI actions.','You are the WEB SUPERVISOR. Your ONLY goal is to manage EXECUTION in a BROWSER environment.

═══ RULE 0: NO CONVERSATION (CRITICAL) ═══
NEVER output any status report, explanation, plans, or conversational text.
Your response MUST be a SINGLE block of valid JSON or the word `done`.
ANY OTHER TEXT IS A CRITICAL SYSTEM FAILURE.

═══ JSON STRUCTURE (STRICT) ═══
Every tool call MUST include these three fields:
{
  "observation": "What do I see relevant to the goal? CRITICAL: ONLY mention elements visible in the CURRENT --- WEB DOM TREE ---.",
  "intent": "What am I about to do and WHY?",
  "tool": "click",
  "input": {
    "target": "GitHub - Revaycolizer"
  }
}

═══ GROUNDING RULES (CRITICAL) ═══
1. NO HALLUCINATIONS: Do NOT invent links or buttons. If it''s not in the `--- WEB DOM TREE ---`, it does not exist.
2. MEMORY VS REALITY: Ignore memories of previous search results if they are not in the current DOM.
3. ABOUT:BLANK: If the URL is `about:blank`, you MUST perform a `search` or `navigate` first.

═══ SEARCH VS CLICK (CRITICAL) ═══
- `tool: "search"` uses DuckDuckGo to search the web.
- NEVER use `tool: "navigate"` to go to a search engine. ALWAYS use `tool: "search"` instead.
- CRITICAL: If the goal explicitly says "Search for [X]", your VERY FIRST action MUST be `{"tool": "search", "input": {"text": "X"}}`.
- If on a target page, NEVER use `tool: "search"`. Use site-search via `type` instead.

═══ WEB RULES (STRICT) ═══
1. NAVIGATION VERIFICATION: A tool result saying "Clicked" is a promise, not proof. Verify the current URL or DOM.
   - If already on the target page, output `{"tool": "done", "input": {}}` immediately.
   - If the goal is JUST to "Search" and results are visible, output `{"tool": "done", "input": {}}` immediately.
2. DUCKDUCKGO & GITHUB: Prefer clicking BOLD header text "GitHub - [Repo Name]". AVOID DISTRACTIONS.
3. SUCCESS CHECK: If last action returned "Error:", "timeout", etc., do NOT output "done". RE-TRY.
4. TOOL USAGE: Use `tool: "click"` with plain text target. XPath/CSS will fail.
   - For OCR-only items, use coordinates: `{"tool": "click", "input": {"x": 100, "y": 200}}`.

═══ RESPONSIBILITIES ═══
1. EXECUTION: Use `browser_automation` for navigation, clicking, and web-search.
2. GROUNDING: Strictly follow the `--- WEB DOM TREE ---`.
3. COMPLETION: Output {"tool": "done", "input": {}} ONLY when the browser is on the target page.','qwen2.5:7b-instruct','web_supervisor','1.2.4',NULL,'2026-03-14 06:42:50','2026-03-14 06:42:50',NULL);
INSERT INTO agents VALUES(7438474706665213952,'Master Orchestrator','Primary system planner','You are the MASTER ORCHESTRATOR. Breakdown the user request into a JSON array of MacroTasks.
Each MacroTask MUST have a "domain" and a "task" string.

═══ VALID DOMAINS ═══
- chat: For general conversation, data synthesis, and creating charts using tools.
- web: For interactive web browsing and research.
- stealth: For background research and API calls (non-interactive).
- desktop: For UI automation and local app control.
- neontune: For Neontune-specific music automation.
- office: For document generation (Word, Excel, PDF).

═══ ROUTING RULES ═══
1. Research: Use "stealth" for background lookup, "web" for interactive browsing.
2. Charts: ALWAYS use "chat" for any chart, graph, or visualization.
3. Synthesis + Chart = ONE single "chat" task. NEVER split them into two tasks.
4. Automation: Use "desktop" for app control, "neontune" for music.

═══ CRITICAL CHART RULE ═══
When the goal requires research AND a chart, create EXACTLY 2 tasks total:
- Task 1: stealth research
- Task 2: ONE chat task that synthesizes AND charts in the same step

WRONG — never create 3 tasks for a chart request:
[
  {"domain": "stealth", "task": "Research data..."},
  {"domain": "chat", "task": "Synthesize findings..."},
  {"domain": "chat", "task": "Create the chart..."}
]

CORRECT — always combine synthesize and chart into one task:
[
  {"domain": "stealth", "task": "Research data..."},
  {"domain": "chat", "task": "Synthesize the research findings and call the charting tool to create the chart."}
]

═══ EXAMPLES ═══

Input: "Research Next.js growth and create a bar chart"
Output: [
  {"domain": "stealth", "task": "Research Next.js growth metrics and adoption rates from 2024 to 2026."},
  {"domain": "chat", "task": "Synthesize the research findings and call the charting tool to create a bar chart of Next.js growth from 2024 to 2026."}
]

Input: "Create a pie chart of Next.js npm usage from 2024 to 2026"
Output: [
  {"domain": "stealth", "task": "Research Next.js npm download statistics and usage percentages for 2024, 2025, and 2026."},
  {"domain": "chat", "task": "Synthesize the research findings and call the charting tool to create a pie chart of Next.js npm usage from 2024 to 2026."}
]

Input: "Create a line chart of Next.js growth from 2024 to 2026"
Output: [
  {"domain": "stealth", "task": "Research Next.js npm download numbers for 2024, 2025, and 2026."},
  {"domain": "chat", "task": "Synthesize the research findings and call the charting tool to create a line chart of Next.js growth from 2024 to 2026."}
]

Input: "Show me all three chart types for Next.js npm growth"
Output: [
  {"domain": "stealth", "task": "Research Next.js npm download statistics for 2024, 2025, and 2026."},
  {"domain": "chat", "task": "Synthesize the research findings and call the charting tool three times to create a bar chart, a line chart, and a pie chart of Next.js npm growth from 2024 to 2026."}
]

Input: "Open youtube and search for cats"
Output: [
  {"domain": "web", "task": "Navigate to youtube.com and search for cats."}
]

Input: "What is the capital of France?"
Output: [
  {"domain": "chat", "task": "Answer the question: what is the capital of France."}
]

═══ FORMAT ENFORCEMENT ═══
- NO MARKDOWN. NO CODE BLOCKS. NO EXPLANATIONS. JUST THE JSON ARRAY.
- Maximum 3 tasks per plan unless absolutely necessary.
- Never create more than 1 chat task for any chart-related request.','qwen2.5:7b-instruct','orchestrator','1.0.0',NULL,'2026-03-14 06:42:50','2026-03-21 08:05:36',NULL);
INSERT INTO agents VALUES(7438474706665213954,'Chat Assistant','General chat and help','You are a helpful Chat Assistant. You can create beautiful charts (bar, line, pie) using the charting tool.

CRITICAL: When you have data to visualize, you MUST call the ''charting'' tool with a JSON payload.

JSON SCHEMA:
{ "tool": "charting", "input": { "chart_type": "bar|line|pie", "title": "Chart Title", "labels": ["label1", "label2"], "data": [10, 20] } }

Respond ONLY with the tool call JSON to generate the chart.','qwen3:latest','chat','1.0.0',NULL,'2026-03-14 06:42:50','2026-03-14 19:40:26',NULL);
INSERT INTO agents VALUES(7438474706665213955,'Vision Assistant','Image analysis specialist','You are a Vision and General Reasoning Assistant.

Your capabilities:
1. Analyze images (vision_analysis).
2. Answer general questions and reason about complex topics.

When asked about an image, ALWAYS use the `vision_analysis` tool.
For general questions, use your internal knowledge.','qwen3-vl:2b','vision','1.0.0',NULL,'2026-03-14 06:42:50','2026-03-14 20:37:21',NULL);
INSERT INTO agents VALUES(7438649930757050370,'Text Writer','Reads screen content and writes text files','You are the TextWriterSupervisor, an elite specialized AI that reads screen content and writes text files.

═══ RULE 0: NO CONVERSATION (CRITICAL) ═══
NEVER output a status report or explanation.
Your response MUST be a SINGLE block of valid JSON or the word `done`.

═══ JSON STRUCTURE (STRICT) ═══
Every tool call MUST include these three fields:
{
  "observation": "What text do I see in CURRENT GROUNDING relevant to the goal?",
  "intent": "What am I about to write to the file?",
  "tool": "code_executor",
  "input": {
    "language": "bash",
    "command": "cat << ''EOF'' > /tmp/summary.txt\\nThis is the summary text.\\nEOF"
  }
}

═══ TEXT WRITER RULES (STRICT) ═══
1. READ: Look at the `CURRENT GROUNDING` to find the information requested by your goal. For GitHub repos, prioritize the README or file list.
2. SYNTHESIZE: Write a comprehensive summary based ONLY on the grounding data.
3. WRITE: Use `code_executor` with `"language": "bash"` and a heredoc.
4. COMPLETION: If the code_executor shows SUCCESS, output `{"tool": "done", "input": {}}` on the next turn.

CRITICAL:
- ONLY output exactly ONE JSON tool call. No conversational text.','qwen2.5:7b-instruct','text_writer','1.0.0',NULL,'2026-03-14 10:40:44','2026-03-14 10:40:44',NULL);
INSERT INTO agents VALUES(7438649930757050371,'Overseer','High-precision verification agent for UI automation','You are the Overseer, a high-precision verification agent for UI automation.
YOUR JOB: Determine if a specific step was ACTUALLY successfully completed based on the Final Grounding and the Plan & History provided.

INPUT:
1. Step Description: The goal the agent was trying to achieve.
2. Final Grounding: The current state of the screen (Vision, UI Tree, OCR).
3. Plan & History: The log of what the agent has done and what the tools returned.

RULES:
- Be skeptical. If the evidence is ambiguous, mark as NOT VERIFIED.
- For ''Open Document'' / ''LibreOffice'' steps: Verify that ''LibreOffice'' or the expected filename appears in the App Hierarchy / UI Tree.
- For ''Save''/''Export'' steps: Verify the file save dialog is gone and the window title is updated or a success message appeared.
- HISTORICAL AWARENESS (CRITICAL): Check the ''Plan & History''. If the last action resulted in an ''Error:'', ''failed:'', or ''could not be loaded'' message, the step is NOT verified.
- DESTINATION RULE (CRITICAL): If the step says ''Click on X'' and the FINAL GROUNDING shows the agent is ALREADY at the destination of X, output `verified: true`.
- Ignore terminal messages or ''intent'' text. Trust ONLY Vision, UI Tree, OCR, and Tool Results.

OUTPUT FORMAT:
You MUST output valid JSON only:
{
  "verified": bool,
  "reason": "A short explanation of why it is or isn''t verified."
}','qwen2.5:7b-instruct','overseer','1.0.0',NULL,'2026-03-14 10:40:44','2026-03-14 10:40:44',NULL);
INSERT INTO agents VALUES(7438900000000000001,'Neontune Agent','Native execution agent for Neontune music player','You are the Neontune Agent. Your job is to execute UI automation commands for the Neontune music player.','llama3:latest','neontune','1.0.0',NULL,'2026-03-15 15:23:27','2026-03-15 15:23:27',NULL);
INSERT INTO agents VALUES(7439134615694086144,'New Agent','A new custom agent','','qwen2.5:7b-instruct','core','1.0.0',NULL,'2026-03-16 02:25:04','2026-03-16 02:25:23','');
INSERT INTO agents VALUES(7439135332081209344,'Synthesizer','Synthesizes research findings into clean user-friendly responses','You are a helpful assistant. Your job is to take research findings and present them clearly and concisely to the user in plain Markdown.

RULES:
- If the research findings indicate a simple successful command OR automation (e.g., ''done'', ''SUCCESS''), AND there are no subsequent errors, provide an extremely concise response (e.g., ''Tali Rush - U is now playing.'').
- If the findings contain a ''GROUNDING ERROR'' or any ''Error:'' message from a tool or supervisor, you MUST report the failure honestly. Do NOT claim success if any step failed.
- If the last step in the findings is an error, the entire response should reflect that failure.
- For complex research or long findings, structure your response with headers and bullet points.
- Never output raw JSON arrays or technical plan data.
- Do NOT provide generic steps or helpful alternatives if the automation failed; instead, report the specific failure.

VISUALIZATION RULE: If the research findings contain an HTML or SVG document (e.g., a chart), you MUST include that raw HTML/SVG document at the end of your response without any markdown wrapping. Do not strip or summarize the technical HTML code if it is intended for visualization.','qwen2.5:7b-instruct','synthesizer','1.0.0',NULL,'2026-03-16 02:27:55','2026-03-16 02:27:55',NULL);
INSERT INTO agents VALUES(7439380445306949632,'Grounding Agent','UI verification and state grounding assistant','You are the ''Additional Eye'' for UI automation. 
Your role is to analyze a list of visible UI text and determine if a specific goal or state has been reached.

RULES:
- Be critical and objective.
- If you see partial matches or ambiguous text, flag it.
- Your output should be a concise assessment (e.g., ''STATE VERIFIED: Tali Rush is playing'' or ''STATE MISMATCH: Search results visible but song not playing'').
- Only report what you see in the provided text list.','qwen2.5:7b-instruct','grounding','1.0.0',NULL,'2026-03-16 18:26:46','2026-03-16 18:26:46',NULL);
INSERT INTO agents VALUES(7438649930752856068,'Desktop Supervisor','Controls native desktop applications','You are the Desktop Agent. Your mission is to automate desktop applications using OCR and UI trees.','qwen2.5:7b-instruct','desktop','1.0.0',NULL,'2026-03-14 06:42:50','2026-03-14 06:42:50',NULL);
-- Table: tools
INSERT INTO tools VALUES(7438474706681991168, 'browser_server', 'browser_server tool for web domain', '#!/usr/bin/env python3
"""
browser_server.py — Persistent Playwright browser daemon for the rig agent.

Keeps ONE browser window alive across all agent calls.
Run once in the background: python3 browser_server.py &
Then browser_automation.py sends HTTP commands to this server.

Port: 9228
"""

import asyncio
import json
import os
import shutil
import sys
import traceback
from http.server import BaseHTTPRequestHandler
from socketserver import TCPServer
import threading
import subprocess

PWA_FLAG = f"--rig-browser-session-{os.getpid()}"

def get_browser_pid():
    try:
        output = subprocess.check_output(["ps", "-e", "-o", "pid,command"], text=True)
        for line in output.splitlines():
            if PWA_FLAG in line:
                parts = line.strip().split(maxsplit=1)
                if len(parts) == 2 and not parts[1].startswith("python"):
                    return int(parts[0])
    except Exception:
        pass
    return None

try:
    from playwright.async_api import async_playwright, TimeoutError as PWTimeout
except ImportError:
    print("ERROR: playwright not installed", flush=True)
    sys.exit(1)

PORT = 9223
DUCKDUCKGO_SEARCH = "https://duckduckgo.com/?q={}"

# Global browser state — shared across all requests
_playwright = None
_browser = None
_page = None
_ctx = None
_loop = None
_lock = threading.Lock()


STEALTH_SCRIPT = """
    Object.defineProperty(navigator, ''webdriver'', {get: () => undefined});
    window.chrome = { runtime: {} };
    Object.defineProperty(navigator, ''languages'', {get: () => [''en-US'', ''en'']});
    Object.defineProperty(navigator, ''plugins'', {get: () => [1,2,3,4,5]});
"""

def find_chrome_binary():
    for name in ["google-chrome-stable", "google-chrome", "chromium-browser", "chromium"]:
        path = shutil.which(name)
        if path:
            return path
    for path in ["/usr/bin/google-chrome", "/usr/bin/chromium", "/usr/local/bin/chromium"]:
        if os.path.exists(path):
            return path
    return None


async def _init_browser():
    global _playwright, _browser, _ctx, _page
    # Use a specific user data dir to maintain state if needed, or stick to temp for isolation
    # For now, stick to standard launch
    _playwright = await async_playwright().start()

    chrome_path = find_chrome_binary()
    launch_kwargs = dict(
        headless=False,
        args=[
            "--start-maximized", "--no-sandbox", "--disable-dev-shm-usage",
            "--disable-blink-features=AutomationControlled",
            "--disable-infobars",
            PWA_FLAG,
        ],
    )
    if chrome_path:
        launch_kwargs["executable_path"] = chrome_path
        print(f"🌍 Using system Chrome: {chrome_path}", flush=True)
    else:
        print("🌍 Using Playwright bundled Chromium", flush=True)

    _browser = await _playwright.chromium.launch(**launch_kwargs)
    _ctx = await _browser.new_context(
        no_viewport=True,
        user_agent=(
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
            "(KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36"
        ),
    )
    await _ctx.add_init_script(STEALTH_SCRIPT)
    _page = await _ctx.new_page()
    print(f"🚀 Browser daemon started. Listening on port {PORT}", flush=True)


async def init_browser():
    global _browser, _ctx, _page, _playwright
    if _playwright:
        await _playwright.stop()
    _playwright = await async_playwright().start()
    _browser = await _playwright.chromium.launch(
        headless=False,
        args=[
            "--disable-blink-features=AutomationControlled",
            "--no-sandbox",
            "--disable-setuid-sandbox",
            "--window-size=1280,720",
        ],
    )
    _ctx = await _browser.new_context(
        viewport={"width": 1280, "height": 720},
        user_agent="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
    )
    await _ctx.add_init_script(STEALTH_SCRIPT)
    _page = await _ctx.new_page()
    return _page

async def ensure_browser():
    global _browser, _ctx, _page, _playwright
    try:
        if not _page:
            return await init_browser()
        
        # Test if the current page/context is alive
        await _page.title()
        return _page
    except Exception as e:
        print(f"⚠️ ensure_browser detected dead context: {e}. Restarting...", file=sys.stderr)
        return await init_browser()

async def run_action(data: dict) -> dict:
    global _page, _browser, _ctx
    action = data.get("action", "")
    text   = data.get("text", "")
    url    = data.get("url", "")
    direction = data.get("direction", "down")
    amount    = data.get("amount", 500)
    result = {"status": "error", "message": "Unknown action"}

    try:
        _page = await ensure_browser()

        # ALWAYS bring window to front
        try:
            await _page.bring_to_front()
        except Exception:
            _page = await init_browser()
            await _page.bring_to_front()

        if action == "navigate":
            if not url:
                return {"status": "error", "message": "Navigation failed: empty URL provided"}
            if not url.startswith("http"):
                url = "https://" + url
            await _page.goto(url, wait_until="domcontentloaded", timeout=30000)
            title = await _page.title()
            result = {"status": "loaded", "title": title, "url": _page.url}

        elif action == "search":
            # Navigate to DuckDuckGo homepage first to establish session cookies
            current_url = _page.url
            needs_ddg_home = "duckduckgo.com" not in current_url
            if needs_ddg_home:
                try:
                    await _page.goto("https://duckduckgo.com", wait_until="domcontentloaded", timeout=15000)
                    await _page.wait_for_timeout(1000)
                except Exception:
                    pass

            # Use real search input on DuckDuckGo homepage
            found_search = False
            for sel in ["input[name=''q'']", "input[type=''search'']", "textarea[name=''q'']"]:
                try:
                    el = await _page.wait_for_selector(sel, timeout=4000, state="visible")
                    if el:
                        await el.click()
                        await el.fill(text)
                        await _page.wait_for_timeout(300)
                        await el.press("Enter")
                        await _page.wait_for_load_state("domcontentloaded", timeout=20000)
                        title = await _page.title()
                        result = {"status": "search_submitted", "title": title, "url": _page.url, "query": text}
                        found_search = True
                        break
                except Exception:
                    continue

            if not found_search:
                print(f"Fallback: Searching via direct URL for ''{text}''", file=sys.stderr)
                try:
                    await _page.goto(
                        f"https://duckduckgo.com/?q={text.replace('' '', ''+'')}",
                        wait_until="domcontentloaded", timeout=20000
                    )
                    title = await _page.title()
                    result = {"status": "search_via_ddg", "title": title, "url": _page.url, "query": text}
                except Exception as e:
                    result = {"status": "error", "message": f"Search fallback failed: {str(e) or type(e).__name__}"}

        elif action == "click":
            clicked = False
            current_url = _page.url
            is_search_engine = any(s in current_url for s in ["google.com", "duckduckgo.com", "bing.com"])
            selector = data.get("selector", "")

            # Step 1: Click by CSS Selector (Most precise)
            if not clicked and selector:
                print(f"DEBUG: Attempting selector click: ''{selector}''", file=sys.stderr)
                try:
                    # Check existence first for better debugging
                    el = await _page.query_selector(selector)
                    if el:
                        print(f"DEBUG: Found element for selector: {selector}", file=sys.stderr)
                        await el.click()
                        clicked = True
                    else:
                        print(f"DEBUG: No element found for selector: {selector}", file=sys.stderr)
                except Exception as e:
                    print(f"DEBUG: Selector click exception: {e}", file=sys.stderr)

            # Step 1.5: Click by X/Y coordinates (from OCR)
            x = data.get("x")
            y = data.get("y")
            if not clicked and x is not None and y is not None:
                print(f"DEBUG: Attempting coordinate click at: ({x}, {y})", file=sys.stderr)
                try:
                    await _page.mouse.click(float(x), float(y))
                    clicked = True
                except Exception as e:
                    print(f"DEBUG: Coordinate click exception: {e}", file=sys.stderr)

            # Step 2: Click by exact URL if provided (Deterministic)
            target_url = data.get("url", "")
            if not clicked and target_url:
                print(f"Deterministic URL click: {target_url}", file=sys.stderr)
                await _page.goto(target_url, wait_until="domcontentloaded", timeout=15000)
                clicked = True

            # Step 3: Click by exact element text (Standard elements)
            if not clicked and text and not is_search_engine:
                safe_text = json.dumps(text.lower().strip())
                clicked = await _page.evaluate(f"""
                    () => {{
                        const targets = Array.from(document.querySelectorAll(''button, a, input, [role="button"], [role="link"]''));
                        const t = {safe_text};
                        const exact = targets.find(el => (el.textContent || el.innerText || el.value || '''').toLowerCase().trim() === t);
                        if (exact) {{
                            exact.click();
                            return true;
                        }}
                        return false;
                    }}
                """)

            # Step 3: Fuzzy Link Selection (Title/Domain Match)
            if not clicked and text:
                keywords = [kw.lower() for kw in text.split() if len(kw) > 2]
                all_links = await _page.evaluate("""
                    () => Array.from(document.querySelectorAll(''a[href]''))
                        .filter(a => a.offsetParent !== null)
                        .map(a => ({
                            text: a.textContent.trim().toLowerCase().substring(0, 100),
                            href: a.href
                        }))
                        .filter(l => l.href.startsWith(''http'') && l.text.length > 0)
                        .slice(0, 300)
                """)

                def score(link):
                    l_text = link["text"].lower()
                    l_href = link["href"].lower()
                    t_text = text.lower().strip()
                    
                    # Domain detection
                    current_domain = current_url.split(''/'')[2] if ''/'' in current_url else ""
                    link_domain = l_href.split(''/'')[2] if ''/'' in l_href else ""
                    
                    is_internal = current_domain and (current_domain in link_domain or link_domain in current_domain)
                    is_major_platform = any(p in t_text for p in ["github", "twitter", "youtube", "linkedin", "instagram"])

                    total = 0
                    if t_text in l_text: total += 1000
                    
                    # Platform boost
                    platform_boost = 0
                    for platform in ["github", "twitter", "youtube", "linkedin", "duckduckgo", "instagram"]:
                        if platform in l_href:
                            # Higher boost if the keyword is also in the request text
                            if platform in t_text:
                                platform_boost = 4000
                            break
                    
                    matched_kws = sum(1 for kw in keywords if kw in l_text or kw in l_href)
                    if len(keywords) <= 2 and matched_kws < len(keywords): return 0
                    
                    total += matched_kws + platform_boost
                    
                    # NAVIGATION HARDENING: 
                    # If we are on a search engine, favor external domains.
                    if is_search_engine:
                        if is_internal:
                            # Penalty for refinements/internal navigation
                            total -= 2000 
                        else:
                            # Boost for external results
                            total += 1000 
                        
                    return total

                matches = sorted([(score(l), l) for l in all_links if score(l) > -5000], key=lambda x: x[0], reverse=True)
                if matches and matches[0][0] > 0:
                    best = matches[0][1]
                    print(f"Link score match: ''{best[''text''][:50]}'' (Score: {matches[0][0]}) -> {best[''href'']}", file=sys.stderr)
                    await _page.goto(best["href"], wait_until="domcontentloaded", timeout=15000)
                    clicked = True

            # Step 4: Final Fuzzy Text Match (Any visible element)
            if not clicked and text:
                safe_text = json.dumps(text.lower().strip() if text else "")
                clicked = await _page.evaluate(f"""
                    () => {{
                        const targets = Array.from(document.querySelectorAll(''*''));
                        const t = {safe_text};
                        const match = targets.find(el => {{
                            if (el.children.length > 0) return false; // leaf nodes only
                            return (el.innerText || el.value || '''').toLowerCase().includes(t);
                        }});
                        if (match) {{
                            match.click();
                            return true;
                        }}
                        return false;
                    }}
                """)

            if clicked:
                await _page.wait_for_load_state("domcontentloaded", timeout=5000)
                title = await _page.title()
                result = {"status": "clicked", "title": title, "url": _page.url}
            else:
                result = {"status": "not_found", "message": f"No element or link matching ''{text}''"}

        elif action == "type":
            submit = data.get("submit", False)
            safe_text = json.dumps(text.lower() if text else "")
            typed = await _page.evaluate(f"""
                async () => {{
                    const t = {safe_text};
                    const inputs = Array.from(document.querySelectorAll(''input, textarea, [contenteditable="true"], [role="textbox"]''))
                        .filter(el => {{
                            const style = window.getComputedStyle(el);
                            return style.display !== ''none'' && style.visibility !== ''hidden'' && el.offsetParent !== null;
                        }});
                    
                    // Try to find by placeholder, aria-label, or value
                    let target = inputs.find(el => 
                        (el.placeholder || "").toLowerCase().includes(t) || 
                        (el.getAttribute("aria-label") || "").toLowerCase().includes(t) ||
                        (el.value || "").toLowerCase().includes(t)
                    );

                    // Fallback: If no direct match, take the first visible input if it''s a search-like page
                    if (!target && inputs.length > 0) {{
                        target = inputs[0];
                    }}

                    if (target) {{
                        target.focus();
                        target.value = ""; // Clear first
                        return true;
                    }}
                    return false;
                }}
            """)

            if typed:
                await _page.keyboard.type(text if text else "")
                if submit:
                    await _page.keyboard.press("Enter")
                    await _page.wait_for_load_state("domcontentloaded", timeout=10000)
                
                title = await _page.title()
                result = {"status": "typed", "title": title, "url": _page.url}
            else:
                result = {"status": "error", "message": f"No input field found to type ''{text}''"}

        elif action == "get_links":
            links = await _page.evaluate("""
                (filterText) => Array.from(document.querySelectorAll(''a[href]''))
                    .filter(a => a.offsetParent !== null)
                    .map(a => ({text: a.textContent.trim().substring(0,100), href: a.href}))
                    .filter(l => l.text.length > 0 && l.href.startsWith(''http''))
                    .filter(l => !filterText ||
                        l.text.toLowerCase().includes(filterText.toLowerCase()) ||
                        l.href.toLowerCase().includes(filterText.toLowerCase()))
                    .slice(0, 20)
            """, text or "")
            if data.get("auto_navigate") and links:
                target = links[0]["href"]
                print(f"Auto-navigating to: {target}", file=sys.stderr)
                await _page.goto(target, wait_until="commit", timeout=12000)
                await _page.wait_for_load_state("domcontentloaded", timeout=15000)
                title = await _page.title()
                result = {"status": "navigated", "title": title, "url": _page.url, "links": links}
            else:
                result = {"status": "ok", "links": links, "count": len(links)}

        elif action == "get_dom_tree":
            dom_tree = await _page.evaluate("""
                () => {
                    function isVisible(e) {
                        return !!(e.offsetWidth || e.offsetHeight || e.getClientRects().length) && window.getComputedStyle(e).visibility !== ''hidden'';
                    }
                    const elements = document.querySelectorAll(''a, button, input, select, textarea'');
                    let res = [];
                    for (let el of elements) {
                        if (!isVisible(el)) continue;
                        let tag = el.tagName.toLowerCase();
                        let text = (el.innerText || el.value || el.placeholder || '''').trim().replace(/\\\\n/g, '' '');
                        if (!text) text = el.getAttribute(''aria-label'') || el.getAttribute(''title'') || '''';
                        if (!text) continue;
                        let info = tag;
                        if (tag === ''a'') {
                            let href = el.getAttribute(''href'') || '''';
                            if (href.startsWith(''/'')) href = window.location.origin + href;
                            if (href.length < 80) info += ` href="${href}"`;
                        }
                        res.push(`[${info}] ${text.substring(0, 100)}`);
                    }
                    return res.slice(0, 800).join(''\\\\n'');
                }
            """)
            result = {"status": "ok", "dom_tree": dom_tree}

        elif action == "get_title":
            title = await _page.title()
            result = {"status": "ok", "title": title, "url": _page.url}

        elif action == "get_url":
            result = {"status": "ok", "url": _page.url}

        elif action == "scroll":
            delta = amount if direction == "down" else -amount
            await _page.evaluate(f"window.scrollBy(0, {delta})")
            await _page.wait_for_timeout(300)
            scroll_y = await _page.evaluate("window.scrollY")
            result = {"status": "scrolled", "scroll_y": scroll_y}

        elif action == "wait_for_text":
            if not text:
                result = {"status": "error", "message": "wait_for_text requires ''text'' parameter"}
            else:
                try:
                    safe_text = json.dumps(text.lower())
                    await _page.wait_for_function(
                        f"() => document.body.innerText.toLowerCase().includes({safe_text})",
                        timeout=timeout
                    )
                    result = {"status": "found", "text": text}
                except PWTimeout:
                    result = {"status": "timeout", "text": text}

        elif action == "screenshot":
            path = "/tmp/rig_browser_debug.png"
            await _page.screenshot(path=path)
            result = {"status": "ok", "path": path}

        elif action == "ping":
            title = await _page.title()
            result = {"status": "alive", "url": _page.url, "title": title}

        else:
            result = {"status": "error", "message": f"Unknown action: {action}"}

    except Exception as e:
        msg = str(e) or f"Unknown {type(e).__name__} error during {action}"
        result = {"status": "error", "message": msg, "trace": traceback.format_exc(limit=3)}

    # Inject browser PID for precision window targeting
    pid = get_browser_pid()
    if pid:
        result["browser_pid"] = pid
    
    return result


class BrowserHandler(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        pass  # Silence access logs

    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(length)
        try:
            data = json.loads(body)
        except Exception:
            data = {}

        # Run the async action in the event loop
        future = asyncio.run_coroutine_threadsafe(run_action(data), _loop)
        try:
            result = future.result(timeout=30)
        except Exception as e:
            msg = str(e) or f"Async operation failed for {data.get(''action'')}"
            result = {"status": "error", "message": msg}

        resp = json.dumps(result).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", len(resp))
        self.end_headers()
        self.wfile.write(resp)

    def do_GET(self):
        # Health check
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b''{"status":"alive"}'')


def run_event_loop(loop):
    asyncio.set_event_loop(loop)
    loop.run_forever()


def main():
    global _loop
    
    # Create asyncio loop in background thread
    _loop = asyncio.new_event_loop()
    t = threading.Thread(target=run_event_loop, args=(_loop,), daemon=True)
    t.start()

    # Initialize browser in the background loop
    future = asyncio.run_coroutine_threadsafe(init_browser(), _loop)
    future.result(timeout=30)

    # Start HTTP server
    with TCPServer(("127.0.0.1", PORT), BrowserHandler) as server:
        print(f"✅ Browser server ready on http://127.0.0.1:{PORT}", flush=True)
        server.serve_forever()


if __name__ == "__main__":
    main()
', 'python', '1.0.0', NULL, '2026-03-14 06:42:50', '2026-03-14 09:42:18', 'agents-scripts/web/browser_server.py', NULL, 'standard');



INSERT INTO tools VALUES(7438474706690379776, 'browser_automation', 'browser_automation tool for web domain', '#!/usr/bin/env python3
"""
browser_automation.py — Thin client for the persistent browser daemon.

Sends a single HTTP request to browser_server.py (which keeps the browser alive),
gets back a JSON result, and prints it.

Usage (same as before):
    python3 browser_automation.py --action search --text "Revaycolizer"
    python3 browser_automation.py --action click --text "GitHub"
    python3 browser_automation.py --action navigate --url "https://github.com"
    python3 browser_automation.py --action get_links --text "github"
    python3 browser_automation.py --action get_title
"""

import argparse
import json
import os
import subprocess
import sys
import time
import urllib.request
import urllib.error

SERVER_URL = "http://127.0.0.1:9223"
SERVER_SCRIPT = os.path.join(os.path.dirname(__file__), "browser_server.py")


def is_server_alive() -> bool:
    try:
        with urllib.request.urlopen(f"{SERVER_URL}/", timeout=2) as r:
            return r.status == 200
    except Exception:
        return False


def start_server():
    """Start the browser daemon in the background if not already running."""
    print("🚀 Starting browser daemon...", file=sys.stderr)
    proc = subprocess.Popen(
        [sys.executable, SERVER_SCRIPT],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )
    # Wait up to 20 seconds for the server to be ready with backoff
    for i in range(40):
        time.sleep(0.5)
        if i > 10 and i % 5 == 0:
            print("⏳ Waiting for browser daemon to initialize...", file=sys.stderr)
        if is_server_alive():
            print("✅ Browser daemon is ready.", file=sys.stderr)
            return True
    print("❌ Browser daemon failed to start.", file=sys.stderr)
    return False


def send_action(payload: dict) -> dict:
    """Send an action to the browser daemon and return its JSON response."""
    body = json.dumps(payload).encode()
    req = urllib.request.Request(
        f"{SERVER_URL}/",
        data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            return json.loads(resp.read())
    except urllib.error.URLError as e:
        return {"status": "error", "message": f"Server connection failed: {e}"}
    except Exception as e:
        return {"status": "error", "message": str(e)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--action", required=True)
    parser.add_argument("--url", default="")
    parser.add_argument("--text", default="")
    parser.add_argument("--selector", default="")
    parser.add_argument("--submit", action="store_true")
    parser.add_argument("--direction", default="down")
    parser.add_argument("--amount", type=int, default=500)
    parser.add_argument("--timeout", type=int, default=15000)
    parser.add_argument("--navigate", action="store_true") # For get_links auto-navigation
    parser.add_argument("--x", type=float, default=None)
    parser.add_argument("--y", type=float, default=None)
    args = parser.parse_args()

    # Ensure daemon is running
    if not is_server_alive():
        if not start_server():
            print(json.dumps({"status": "error", "message": "Could not start browser daemon"}))
            sys.exit(1)

    payload = {
        "action": args.action,
        "url": args.url,
        "text": args.text,
        "selector": args.selector,
        "submit": args.submit,
        "direction": args.direction,
        "amount": args.amount,
        "timeout": args.timeout,
        "auto_navigate": args.navigate,
        "x": args.x,
        "y": args.y,
    }

    result = send_action(payload)

    # Human-readable status line to stdout (for the agent log)
    status = result.get("status", "?")
    title  = result.get("title", "")
    url    = result.get("url", "")

    if args.action == "search":
        if status == "error":
            print(f"❌ Search failed: {result.get(''message'', ''Unknown error'')}")
        else:
            print(f"✅ Search complete: ''{title}'' @ {url}")
    elif args.action == "click":
        if status == "clicked":
            print(f"✅ Clicked. Now at: ''{title}'' @ {url}")
        else:
            print(f"⚠️ Click result: {result.get(''message'', status)}")
    elif args.action == "navigate":
        print(f"✅ Navigated to: ''{title}'' @ {url}")
    elif args.action == "get_links":
        if status == "navigated":
            print(f"✅ Found matches and Navigated to: ''{title}'' @ {url}")
        else:
            links = result.get("links", [])
            print(f"✅ Found {len(links)} links:")
            for l in links[:10]:
                print(f"  [{l.get(''text'','''')[:60]}] -> {l.get(''href'','''')[:80]}")
    elif args.action == "get_title":
        print(f"📄 ''{title}'' @ {url}")
    else:
        print(f"✅ {args.action}: {status}")

    # Always also print the full JSON for the Rust side to parse
    print(json.dumps(result))


if __name__ == "__main__":
    main()
', 'python', '1.0.0', NULL, '2026-03-14 06:42:50', '2026-03-14 06:42:50', 'agents-scripts/web/browser_automation.py', NULL, 'standard');



INSERT INTO tools VALUES(7438474706690379777, 'stealth_search', 'stealth_search: Accept a raw search query string as input. Return search results with URLs and snippets.', '#!/usr/bin/env python3
import sys
import requests
import json
from bs4 import BeautifulSoup

def search_ddg_stealth(query):
    # Use DuckDuckGo HTML version for scraping (lite/html)
    url = "https://html.duckduckgo.com/html/"
    headers = {
        "User-Agent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
    }
    
    try:
        response = requests.post(url, data={"q": query}, headers=headers, timeout=10)
        response.raise_for_status()
        
        soup = BeautifulSoup(response.text, "html.parser")
        results = []
        
        for item in soup.select(".result"):
            title_el = item.select_one(".result__a")
            snippet_el = item.select_one(".result__snippet")
            
            if title_el:
                title = title_el.get_text(strip=True)
                href = title_el.get("href")
                snippet = snippet_el.get_text(strip=True) if snippet_el else ""
                
                results.append({
                    "title": title,
                    "url": href,
                    "snippet": snippet
                })
        
        return results[:5]
    except Exception as e:
        return {"error": str(e)}

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(json.dumps({"error": "No query provided"}))
        sys.exit(1)
        
    query = " ".join(sys.argv[1:])
    results = search_ddg_stealth(query)
    print(json.dumps(results, indent=2))
', 'python', '1.0.0', NULL, '2026-03-14 06:42:50', '2026-03-14 06:42:50', 'agents-scripts/stealth/stealth_search.py', NULL, 'standard');



INSERT INTO tools VALUES(7438474706690379778, 'stealth_browse', 'stealth_browse: Accept a raw URL string as input (e.g. "https://nextjs.org"). Fetch and return cleaned text content.', '#!/usr/bin/env python3
import sys
import requests
import json
from bs4 import BeautifulSoup
import re

def get_page_text(url):
    headers = {
        "User-Agent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
    }
    
    try:
        response = requests.get(url, headers=headers, timeout=15)
        response.raise_for_status()
        
        soup = BeautifulSoup(response.text, "html.parser")
        
        # Remove script and style elements
        for script_or_style in soup(["script", "style", "header", "footer", "nav"]):
            script_or_style.decompose()
            
        # Get text
        text = soup.get_text(separator='' '')
        
        # Clean up whitespace
        lines = (line.strip() for line in text.splitlines())
        chunks = (phrase.strip() for line in lines for phrase in line.split("  "))
        text = ''\\n''.join(chunk for chunk in chunks if chunk)
        
        # Limit total length to fit in Rig outputs sensibly, but much more than 10KB raw HTML
        # We can afford more text if it''s clean text.
        return text[:15000] 
    except Exception as e:
        return f"Error fetching {url}: {str(e)}"

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: stealth_browse.py <url>")
        sys.exit(1)
        
    url = sys.argv[1]
    result = get_page_text(url)
    print(result)
', 'python', '1.0.0', NULL, '2026-03-14 06:42:50', '2026-03-14 06:42:50', 'agents-scripts/stealth/stealth_browse.py', NULL, 'standard');



INSERT INTO tools VALUES(7438474706694574080, 'calibrate', 'calibrate tool for desktop domain', 'import pyautogui
import time
import json
import sys
import os

CONFIG_FILE = os.path.expanduser("~/.neontune_config.json")

def calibrate():
    print("🎯 Neontune Calibration Tool")
    print("--------------------------------")
    print("We need to teach the agent where the UI elements are.")
    print("Move your mouse to the target and press ENTER in this terminal.")
    
    try:
        # 1. Search Bar
        input("\\n1. Hover over the **Search Bar** (text input) and press ENTER...")
        sx, sy = pyautogui.position()
        print(f"   Captured: {sx}, {sy}")
        
        # 2. First Result
        input("\\n2. Type a search, wait for results, then hover over the **First Song Row** and press ENTER...")
        rx, ry = pyautogui.position()
        print(f"   Captured: {rx}, {ry}")
        
        config = {
            "search": [sx, sy],
            "result": [rx, ry]
        }
        
        with open(CONFIG_FILE, "w") as f:
            json.dump(config, f)
            
        print(f"\\n✅ Configuration saved to {CONFIG_FILE}")
        print("Automation attempts will now use these exact coordinates automatically.")
        
    except KeyboardInterrupt:
        print("\\nCalibration cancelled.")
        sys.exit(1)

if __name__ == "__main__":
    calibrate()
', 'python', '1.0.0', NULL, '2026-03-14 06:42:50', '2026-03-14 06:42:50', 'agents-scripts/desktop/calibrate.py', NULL, 'standard');



INSERT INTO tools VALUES(7438474706694574081, 'diagnose_ui', 'diagnose_ui tool for desktop domain', 'import subprocess
import time
import os

def run_diagnosis():
    # 1. Launch/Check
    print("🚀 Launching/Checking Neontune...")
    subprocess.Popen(["neontune"])
    time.sleep(4)
    
    window_name = "tao window"
    
    # 2. Debug Layout
    print("📋 Current Window List (wmctrl -lG):")
    subprocess.run(["wmctrl", "-lG"])
    
    # 3. Force Focus (Aggressive)
    print(f"🎯 Focusing {window_name}...")
    subprocess.run(["wmctrl", "-a", window_name], check=False)
    time.sleep(0.5)
    
    # xdotool activate
    try:
        wid = subprocess.check_output([''xdotool'', ''search'', ''--name'', ''--limit'', ''1'', window_name]).decode().strip()
        print(f"✅ Found Window ID: {wid}")
        subprocess.run([''xdotool'', ''windowactivate'', ''--sync'', wid])
    except Exception as e:
        print(f"⚠️ xdotool failed: {e}")

    # 4. Reset Layout (Force 0,0) after focus
    print(f"📏 Resetting layout...")
    subprocess.run(["wmctrl", "-r", window_name, "-e", "0,0,0,1280,800"], check=False)
    time.sleep(1)
    
    # 5. Screenshot
    print("📸 Taking screenshot...")
    subprocess.run(["import", "-window", "root", "ui_debug.png"])
    
    # 6. OCR
    print("🔍 Running OCR...")
    subprocess.run(["tesseract", "ui_debug.png", "ui_debug_ocr", "hocr"])
    
    # 7. Parse
    print("📊 Extracting word coordinates...")
    subprocess.run(["python3", "scripts/ocr_dump.py"])

if __name__ == "__main__":
    run_diagnosis()
', 'python', '1.0.0', NULL, '2026-03-14 06:42:50', '2026-03-14 06:42:50', 'agents-scripts/desktop/diagnose_ui.py', NULL, 'standard');



INSERT INTO tools VALUES(7438474706694574082, 'configure_app', 'configure_app tool for desktop domain', '#!/usr/bin/env python3
import pyautogui
import json
import os
import argparse
import time

CONFIG_PATH = os.path.expanduser("~/.config/rig/apps.json")

def load_config():
    if os.path.exists(CONFIG_PATH):
        try:
            with open(CONFIG_PATH, ''r'') as f:
                return json.load(f)
        except:
            pass
    return {}

def save_config(config):
    os.makedirs(os.path.dirname(CONFIG_PATH), exist_ok=True)
    with open(CONFIG_PATH, ''w'') as f:
        json.dump(config, f, indent=4)
    print(f"✅ Configuration saved to {CONFIG_PATH}")

def configure(app_name):
    print(f"\\n🔧 Configuring ''{app_name}'' automation profile.")
    print("------------------------------------------------")
    print("We need to teach the agent where the Search Bar and Results are.")
    print("Launch the app manually and arrange it how you like (or let the agent reset it later).")
    print("NOTE: It is best if the app is maximized or in a standard position.\\n")

    input(f"1. Open ''{app_name}'' and Hover mouse over the **SEARCH BAR**. Press ENTER when ready...")
    sx, sy = pyautogui.position()
    print(f"   📍 Captured Search: {sx}, {sy}")
    
    time.sleep(1)
    
    input(f"2. Type a search (e.g. ''Test'') and Hover over the **FIRST RESULT**. Press ENTER when ready...")
    rx, ry = pyautogui.position()
    print(f"   📍 Captured Result: {rx}, {ry}")

    # Load existing
    config = load_config()
    profile = config.get(app_name.lower(), {})
    
    # Update profile
    profile["window_title"] = app_name # User can edit json later if complex
    profile["launch_cmd"] = [app_name.lower()]
    profile["search_pos"] = [sx, sy]
    profile["result_pos"] = [rx, ry]
    # Default to NO reset if user is calibrating manually, 
    # OR we assume they want the reset behavior? 
    # Let''s ask? No, keep it simple. Enable reset for consistency.
    profile["reset_layout"] = True 

    config[app_name.lower()] = profile
    save_config(config)
    print(f"\\n✨ Setup Complete for {app_name}!")
    print(f"You can now run: cargo run -- \\"play ''Song'' in {app_name}\\"")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--app", required=True, help="App name to configure")
    args = parser.parse_args()
    
    configure(args.app)
', 'python', '1.0.0', NULL, '2026-03-14 06:42:50', '2026-03-14 06:42:50', 'agents-scripts/desktop/configure_app.py', NULL, 'standard');



INSERT INTO tools VALUES(7438474706694574083, 'paddle_ui_automation', 'paddle_ui_automation tool for desktop domain', X'23212f7573722f62696e2f656e7620707974686f6e330a2222220a506164646c654f43522d6261736564205549204175746f6d6174696f6e2073637269707420666f722074657374696e672e0a5265706c6163657320546573736572616374204f4352207769746820506164646c654f435220746f20636f6d7061726520706572666f726d616e636520616e642061636375726163792e0a2222220a0a696d706f7274206f730a232053455420464c414753204245464f524520414e5920494d504f5254530a6f732e656e7669726f6e5b22504144444c455f5044585f44495341424c455f4d4f44454c5f534f555243455f434845434b225d203d202254727565220a6f732e656e7669726f6e5b22464c4147535f7573655f6f6e65646e6e225d203d202230220a6f732e656e7669726f6e5b22464c4147535f7573655f6d6b6c646e6e225d203d202230220a6f732e656e7669726f6e5b22464c4147535f656e61626c655f7069725f617069225d203d202230220a6f732e656e7669726f6e5b22464c4147535f656e61626c655f6578656375746f725f7632225d203d202230220a6f732e656e7669726f6e5b22504144444c455f5245434f474e495a45525f42415443485f53495a45225d203d202231220a6f732e656e7669726f6e5b22464c4147535f616c6c6f6361746f725f7374726174656779225d203d20226e616976655f626573745f666974220a0a696d706f7274207379730a696d706f72742074696d650a696d706f72742073756270726f636573730a696d706f72742070796175746f6775690a696d706f72742061726770617273650a696d706f7274206a736f6e0a696d706f72742074656d7066696c650a696d706f727420706164646c650a66726f6d20706164646c656f637220696d706f727420506164646c654f43520a0a232044697361626c652070796175746f677569206661696c7361666520666f72206175746f6d6174696f6e0a70796175746f6775692e4641494c53414645203d2046616c73650a0a2320476c6f62616c20666f72206c617a79206c6f6164696e670a5f6f63725f696e7374616e6365203d204e6f6e650a0a646566206765745f6f637228293a0a202020202222224c617a696c7920696e697469616c697a657320616e642072657475726e732074686520506164646c654f435220696e7374616e63652077697468206c6f67732064697361626c65642e2222220a20202020676c6f62616c205f6f63725f696e7374616e63650a202020206966205f6f63725f696e7374616e6365206973204e6f6e653a0a20202020202020207072696e742822f09f93a520496e697469616c697a696e6720506164646c654f435220286c617a79292e2e2e222c2066696c653d7379732e737464657272290a20202020202020207472793a0a2020202020202020202020202320537472617465677920313a205374616e6461726420696e697469616c697a6174696f6e2077697468206c6f6767696e672064697361626c65640a2020202020202020202020205f6f63725f696e7374616e6365203d20506164646c654f4352286c616e673d27656e272c2073686f775f6c6f673d46616c73652c207573655f616e676c655f636c733d46616c73652c207573655f6770753d46616c7365290a202020202020202065786365707420457863657074696f6e3a0a2020202020202020202020207472793a0a202020202020202020202020202020202320537472617465677920323a204162736f6c7574652062617265626f6e65730a202020202020202020202020202020205f6f63725f696e7374616e6365203d20506164646c654f4352286c616e673d27656e272c2073686f775f6c6f673d46616c7365290a20202020202020202020202065786365707420457863657074696f6e20617320653a0a202020202020202020202020202020207072696e74286622e29d8c204661696c656420746f20696e697469616c697a6520506164646c654f43523a207b657d222c2066696c653d7379732e737464657272290a202020202020202020202020202020207379732e657869742831290a2020202072657475726e205f6f63725f696e7374616e63650a0a646566206765745f77696e646f775f67656f6d6574727928776964293a0a2020202022222252657475726e732028782c20792c2077696474682c2068656967687429206f66207468652077696e646f772e2222220a202020207472793a0a20202020202020206f7574203d2073756270726f636573732e636865636b5f6f7574707574285b2778646f746f6f6c272c202767657477696e646f7767656f6d65747279272c2073747228776964295d292e6465636f646528290a2020202020202020706f735f6c696e65203d205b6c20666f72206c20696e206f75742e73706c697428275c6e27292069662027506f736974696f6e3a2720696e206c5d5b305d0a202020202020202067656f5f6c696e65203d205b6c20666f72206c20696e206f75742e73706c697428275c6e2729206966202747656f6d657472793a2720696e206c5d5b305d0a20202020202020200a2020202020202020232055736520726567657820746f2066696e642067656f6d657472790a2020202020202020696d706f72742072650a2020202020202020782c2079203d206d617028696e742c2072652e736561726368287227506f736974696f6e3a20285c642b292c285c642b29272c20706f735f6c696e65292e67726f7570732829290a2020202020202020772c2068203d206d617028696e742c2072652e73656172636828722747656f6d657472793a20285c642b2978285c642b29272c2067656f5f6c696e65292e67726f7570732829290a202020202020202072657475726e20782c20792c20772c20680a202020206578636570743a0a202020202020202072657475726e20302c20302c20302c20300a0a646566206765745f77696e646f775f69642877696e646f775f7469746c65293a0a2020202022222246696e64732061207669657761626c652077696e646f77204944206d61746368696e6720746865207469746c65206f7220636c6173732c2066696c746572696e67206f75742074696e79207574696c6974792077696e646f77732e2222220a202020207472793a0a20202020202020202320536561726368206e616d652c20636c6173732c20616e6420636c6173736e616d65202878646f746f6f6c2064656661756c74290a20202020202020202320576520757365202d2d616e7920746f206d6174636820696620616e7920637269746572696f6e206d6174636865730a20202020202020206f7574203d2073756270726f636573732e636865636b5f6f7574707574285b2778646f746f6f6c272c2027736561726368272c20272d2d616e79272c20272d2d6e616d65272c20272d2d636c617373272c20272d2d636c6173736e616d65272c2077696e646f775f7469746c655d2c207374646572723d73756270726f636573732e5354444f5554290a202020202020202077696473203d206f75742e6465636f646528292e737472697028292e73706c697428275c6e27290a20202020202020200a202020202020202063616e64696461746573203d205b5d0a2020202020202020666f722077696420696e20776964733a0a202020202020202020202020776964203d207769642e737472697028290a2020202020202020202020206966206e6f74207769643a20636f6e74696e75650a2020202020202020202020207472793a0a202020202020202020202020202020202320557365207877696e696e666f20746f20636865636b2069662069742773207669657761626c6520616e64206765742067656f6d657472790a20202020202020202020202020202020696e666f203d2073756270726f636573732e636865636b5f6f7574707574285b277877696e696e666f272c20272d6964272c207769645d2c207374646572723d73756270726f636573732e5354444f5554292e6465636f646528290a20202020202020202020202020202020696620224d61702053746174653a2049735669657761626c6522206e6f7420696e20696e666f3a0a2020202020202020202020202020202020202020636f6e74696e75650a202020202020202020202020202020200a202020202020202020202020202020202320457874726163742067656f6d657472790a20202020202020202020202020202020696d706f72742072650a20202020202020202020202020202020775f6d61746368203d2072652e73656172636828722757696474683a20285c642b29272c20696e666f290a20202020202020202020202020202020685f6d61746368203d2072652e7365617263682872274865696768743a20285c642b29272c20696e666f290a202020202020202020202020202020206966206e6f7420775f6d61746368206f72206e6f7420685f6d617463683a20636f6e74696e75650a202020202020202020202020202020200a20202020202020202020202020202020772c2068203d20696e7428775f6d617463682e67726f7570283129292c20696e7428685f6d617463682e67726f7570283129290a202020202020202020202020202020200a20202020202020202020202020202020232046696c746572206f75742074696e792077696e646f77732028746f6f6c746970732c207472616e7369656e74732c206574632e290a2020202020202020202020202020202069662077203e2031303020616e642068203e203130303a0a202020202020202020202020202020202020202063616e646964617465732e617070656e6428287769642c2077202a206829290a20202020202020202020202065786365707420457863657074696f6e3a0a20202020202020202020202020202020636f6e74696e75650a20202020202020200a202020202020202069662063616e646964617465733a0a202020202020202020202020232052657475726e20746865206c617267657374207669657761626c652077696e646f770a20202020202020202020202063616e646964617465732e736f7274286b65793d6c616d62646120783a20785b315d2c20726576657273653d54727565290a20202020202020202020202072657475726e2063616e646964617465735b305d5b305d0a2020202020202020202020200a2020202020202020232046616c6c6261636b20746f206c617374206d61746368206966206e6f20226c61726765207669657761626c65222077696e646f7720666f756e640a202020202020202072657475726e20776964735b2d315d206966207769647320656c7365204e6f6e650a2020202065786365707420457863657074696f6e20617320653a0a20202020202020207072696e74286622e29aa0efb88f206765745f77696e646f775f6964206572726f723a207b657d222c2066696c653d7379732e737464657272290a202020202020202072657475726e204e6f6e650a0a646566206765745f73637265656e5f74657874286170705f6e616d652c20677265705f7374723d4e6f6e65293a0a2020202022222243617074757265732077696e646f7720616e642072657475726e73204a534f4e206c697374206f6620616c6c2074657874207769746820636f6f7264732e2222220a202020207072696e74286622f09f93962052656164696e672073637265656e2074657874207573696e6720506164646c654f435220666f72207b6170705f6e616d657d2e2e2e222c2066696c653d7379732e737464657272290a202020200a20202020776964203d204e6f6e650a202020206966206170705f6e616d652e6c6f7765722829203d3d2022616374697665223a0a20202020202020207472793a0a202020202020202020202020776964203d2073756270726f636573732e636865636b5f6f7574707574285b2778646f746f6f6c272c202767657461637469766577696e646f77275d292e6465636f646528292e737472697028290a20202020202020206578636570743a20706173730a20202020656c73653a0a202020202020202073756270726f636573732e72756e285b27776d6374726c272c20272d61272c206170705f6e616d655d2c20636865636b3d46616c7365290a2020202020202020776964203d206765745f77696e646f775f6964286170705f6e616d65290a0a202020206966206e6f74207769643a0a202020202020202072657475726e206a736f6e2e64756d7073287b226572726f72223a20662257696e646f77206e6f7420666f756e6420666f7220277b6170705f6e616d657d27227d290a20202020202020200a2020202074696d652e736c65657028302e35290a202020200a202020207472793a0a2020202020202020746d705f696d67203d20222f746d702f7269675f706164646c655f636170747572652e706e67220a202020202020202077696e5f782c2077696e5f792c2077696e5f772c2077696e5f68203d206765745f77696e646f775f67656f6d6574727928776964290a20202020202020200a20202020202020202320526f627573742063617074757265207573696e672070796175746f6775692069662067656f6d657472792069732076616c69640a202020202020202069662077696e5f77203e203020616e642077696e5f68203e20303a0a2020202020202020202020207472793a0a202020202020202020202020202020202320726567696f6e20697320286c6566742c20746f702c2077696474682c20686569676874290a2020202020202020202020202020202073637265656e73686f74203d2070796175746f6775692e73637265656e73686f7428726567696f6e3d2877696e5f782c2077696e5f792c2077696e5f772c2077696e5f6829290a2020202020202020202020202020202073637265656e73686f742e7361766528746d705f696d67290a20202020202020202020202065786365707420457863657074696f6e20617320653a0a202020202020202020202020202020207072696e74286622e29aa0efb88f202070796175746f6775692e73637265656e73686f7428726567696f6e29206661696c65643a207b657d2c20747279696e672066756c6c2073637265656e73686f74202b2063726f70222c2066696c653d7379732e737464657272290a202020202020202020202020202020207472793a0a20202020202020202020202020202020202020202320436170747572652066756c6c2073637265656e2066697273742c207468656e2063726f7020746f2061766f696420726567696f6e206572726f72730a202020202020202020202020202020202020202066756c6c5f73637265656e73686f74203d2070796175746f6775692e73637265656e73686f7428290a2020202020202020202020202020202020202020232077696e5f782f79206d69676874206265206e65676174697665206f72206f7574206f6620626f756e64732069662077696e646f77206973207061727469616c6c79206f666673637265656e0a20202020202020202020202020202020202020202320576520656e737572652063726f702073746179732077697468696e20696d61676520626f756e64730a2020202020202020202020202020202020202020696d675f772c20696d675f68203d2066756c6c5f73637265656e73686f742e73697a650a20202020202020202020202020202020202020206c656674203d206d617828302c2077696e5f78290a2020202020202020202020202020202020202020746f70203d206d617828302c2077696e5f79290a20202020202020202020202020202020202020207269676874203d206d696e28696d675f772c2077696e5f78202b2077696e5f77290a2020202020202020202020202020202020202020626f74746f6d203d206d696e28696d675f682c2077696e5f79202b2077696e5f68290a20202020202020202020202020202020202020200a20202020202020202020202020202020202020206966207269676874203e206c65667420616e6420626f74746f6d203e20746f703a0a20202020202020202020202020202020202020202020202066756c6c5f73637265656e73686f742e63726f7028286c6566742c20746f702c2072696768742c20626f74746f6d29292e7361766528746d705f696d67290a2020202020202020202020202020202020202020656c73653a0a20202020202020202020202020202020202020202020202072616973652056616c75654572726f72286622496e76616c69642063726f7020617265613a207b6c6566747d2c7b746f707d2c7b72696768747d2c7b626f74746f6d7d22290a2020202020202020202020202020202065786365707420457863657074696f6e2061732065323a0a20202020202020202020202020202020202020207072696e74286622e29aa0efb88f202043726f702066616c6c6261636b206661696c65643a207b65327d2c20747279696e67207363726f74222c2066696c653d7379732e737464657272290a20202020202020202020202020202020202020202320557365207363726f7420696e7374656164206f6620496d6167654d616769636b27732027696d706f72742720e28094206d6f72652072656c6961626c65207769746820636f6d706f7369746f72730a2020202020202020202020202020202020202020726573756c74203d2073756270726f636573732e72756e285b277363726f74272c20272d2d77696e646f77272c2073747228776964292c20746d705f696d675d2c20636170747572655f6f75747075743d54727565290a2020202020202020202020202020202020202020696620726573756c742e72657475726e636f646520213d20303a0a20202020202020202020202020202020202020202020202023205472756c79206c617374207265736f72743a20696d706f72740a2020202020202020202020202020202020202020202020207072696e74286622e29aa0efb88f20207363726f74206661696c65642c2066616c6c696e67206261636b20746f20696d706f7274222c2066696c653d7379732e737464657272290a20202020202020202020202020202020202020202020202073756270726f636573732e72756e285b27696d706f7274272c20272d77696e646f77272c207769642c20746d705f696d675d2c20636865636b3d54727565290a2020202020202020656c73653a0a202020202020202020202020232046616c6c6261636b2069662067656f6d6574727920636f756c64206e6f742062652064657465726d696e65640a202020202020202020202020726573756c74203d2073756270726f636573732e72756e285b277363726f74272c20272d2d77696e646f77272c2073747228776964292c20746d705f696d675d2c20636170747572655f6f75747075743d54727565290a202020202020202020202020696620726573756c742e72657475726e636f646520213d20303a0a2020202020202020202020202020202073756270726f636573732e72756e285b27696d706f7274272c20272d77696e646f77272c207769642c20746d705f696d675d2c20636865636b3d54727565290a20202020202020200a202020202020202073746172745f74696d65203d2074696d652e74696d6528290a202020202020202023204c415a59204c4f414420484552450a20202020202020206f63725f656e67696e65203d206765745f6f637228290a2020202020202020726573756c74203d206f63725f656e67696e652e6f637228746d705f696d67290a2020202020202020656c6170736564203d2074696d652e74696d652829202d2073746172745f74696d650a20202020202020207072696e74286622e29aa1204f435220746f6f6b207b656c61707365643a2e32667d207365636f6e6473222c2066696c653d7379732e737464657272290a0a202020202020202066696e616c5f726573756c7473203d205b5d0a2020202020202020696620726573756c7420616e6420726573756c745b305d3a0a202020202020202020202020666f72206c696e6520696e20726573756c745b305d3a0a202020202020202020202020202020206966206e6f74206c696e65206f72206c656e286c696e6529203c20323a20636f6e74696e75650a20202020202020202020202020202020706f696e7473203d206c696e655b305d2023205b5b78312c2079315d2c205b78322c2079315d2c205b78322c2079325d2c205b78312c2079325d5d0a2020202020202020202020202020202074657874203d206c696e655b315d5b305d0a20202020202020202020202020202020636f6e66203d206c696e655b315d5b315d0a202020202020202020202020202020200a20202020202020202020202020202020696620677265705f73747220616e6420677265705f7374722e6c6f7765722829206e6f7420696e20746578742e6c6f77657228293a0a2020202020202020202020202020202020202020636f6e74696e75650a202020202020202020202020202020200a202020202020202020202020202020202320426f756e64696e6720626f780a202020202020202020202020202020207830203d206d696e28705b305d20666f72207020696e20706f696e7473290a202020202020202020202020202020207930203d206d696e28705b315d20666f72207020696e20706f696e7473290a202020202020202020202020202020207831203d206d617828705b305d20666f72207020696e20706f696e7473290a202020202020202020202020202020207931203d206d617828705b315d20666f72207020696e20706f696e7473290a202020202020202020202020202020200a202020202020202020202020202020207461726765745f78203d2077696e5f78202b20287830202b20783129202f2f20320a202020202020202020202020202020207461726765745f79203d2077696e5f79202b20287930202b20793129202f2f20320a202020202020202020202020202020200a2020202020202020202020202020202066696e616c5f726573756c74732e617070656e64287b0a20202020202020202020202020202020202020202274657874223a20746578742c0a20202020202020202020202020202020202020202278223a20696e74287461726765745f78292c0a20202020202020202020202020202020202020202279223a20696e74287461726765745f79292c0a2020202020202020202020202020202020202020227769647468223a20696e74287831202d207830292c0a202020202020202020202020202020202020202022686569676874223a20696e74287931202d207930292c0a202020202020202020202020202020202020202022636f6e666964656e6365223a20666c6f617428636f6e66290a202020202020202020202020202020207d290a20202020202020200a202020202020202072657475726e206a736f6e2e64756d70732866696e616c5f726573756c7473290a2020202065786365707420457863657074696f6e20617320653a0a2020202020202020696d706f72742074726163656261636b0a202020202020202072657475726e206a736f6e2e64756d7073287b226572726f72223a207374722865292c20227472616365223a2074726163656261636b2e666f726d61745f65786328297d290a0a646566206c61756e63685f6170705f73616665286170705f6e616d65293a0a202020202222224c61756e6368657320746865206170706c69636174696f6e207573696e6720612062696e617279207365617263682e2222220a202020207072696e74286622f09f9a8020417474656d7074696e6720746f206c61756e636820277b6170705f6e616d657d272e2e2e222c2066696c653d7379732e737464657272290a202020207472793a0a20202020202020202320312e205472792077686963680a202020202020202070617468203d2073756270726f636573732e636865636b5f6f7574707574285b277768696368272c206170705f6e616d652e6c6f77657228295d292e6465636f646528292e737472697028290a2020202020202020696620706174683a0a20202020202020202020202073756270726f636573732e506f70656e285b706174685d290a20202020202020202020202072657475726e20547275650a202020206578636570743a20706173730a202020200a202020202320322e2054727920636f6d6d6f6e206c6f636174696f6e730a20202020666f72206c6f6320696e205b66222f7573722f62696e2f7b6170705f6e616d652e6c6f77657228297d222c2066222f7573722f6c6f63616c2f62696e2f7b6170705f6e616d652e6c6f77657228297d225d3a0a20202020202020206966206f732e706174682e657869737473286c6f63293a0a20202020202020202020202073756270726f636573732e506f70656e285b6c6f635d290a20202020202020202020202072657475726e20547275650a2020202020202020202020200a202020202320332e204c6173742064697463683a2074727920746f2072756e20617320636f6d6d616e640a202020207472793a0a202020202020202073756270726f636573732e506f70656e285b6170705f6e616d652e6c6f77657228295d290a202020202020202072657475726e20547275650a202020206578636570743a0a202020202020202072657475726e2046616c73650a0a646566206d61696e28293a0a202020202320466c657869626c6520434c493a2068616e646c6520636173657320776865726520636f6d6d616e642069732070617373656420617320612073696e676c6520737472696e67202873797374656d2063616c6c290a2020202023206f72206173207365706172617465206261736820617267756d656e74732e0a202020200a202020207261775f61726773203d207379732e617267765b313a5d0a202020200a20202020232049662077652067657420612073696e676c6520737472696e672077697468207370616365732c2073706c69742069740a202020206966206c656e287261775f6172677329203d3d203120616e642022202220696e207261775f617267735b305d3a0a20202020202020207261775f61726773203d207261775f617267735b305d2e73706c697428290a20202020202020200a20202020616374696f6e203d204e6f6e650a20202020617070203d2022616374697665220a2020202074657874203d204e6f6e650a202020200a202020206966206c656e287261775f6172677329203e3d20313a0a20202020202020202320436865636b20696620666972737420776f72642069732061206b6e6f776e20616374696f6e0a202020202020202066697273745f776f7264203d207261775f617267735b305d2e6c6f77657228290a202020202020202069662066697273745f776f726420696e205b226c61756e6368222c2022736561726368222c2022636c69636b222c20226765745f74657874222c20226765745f7374617465222c202277616974225d3a0a202020202020202020202020616374696f6e203d2066697273745f776f72640a2020202020202020202020206966206c656e287261775f6172677329203e20313a0a20202020202020202020202020202020696620616374696f6e203d3d20226c61756e6368223a0a2020202020202020202020202020202020202020617070203d207261775f617267735b315d0a20202020202020202020202020202020656c73653a0a20202020202020202020202020202020202020202320436f6c6c6563742065766572797468696e6720616674657220616374696f6e20746861742069736e2774206120666c61670a2020202020202020202020202020202020202020746578745f7061727473203d205b5d0a202020202020202020202020202020202020202069203d20310a20202020202020202020202020202020202020207768696c652069203c206c656e287261775f61726773293a0a2020202020202020202020202020202020202020202020206966207261775f617267735b695d203d3d20222d2d6170702220616e642069202b2031203c206c656e287261775f61726773293a0a20202020202020202020202020202020202020202020202020202020617070203d207261775f617267735b692b315d0a2020202020202020202020202020202020202020202020202020202069202b3d20320a202020202020202020202020202020202020202020202020656c6966207261775f617267735b695d2e7374617274737769746828222d2d22293a0a202020202020202020202020202020202020202020202020202020202320536b6970206f7468657220706f74656e7469616c20666c61677320616e642074686569722076616c7565730a2020202020202020202020202020202020202020202020202020202069202b3d20320a202020202020202020202020202020202020202020202020656c73653a0a20202020202020202020202020202020202020202020202020202020746578745f70617274732e617070656e64287261775f617267735b695d290a2020202020202020202020202020202020202020202020202020202069202b3d20310a202020202020202020202020202020202020202074657874203d202220222e6a6f696e28746578745f7061727473290a2020202020202020656c73653a0a2020202020202020202020202320547261646974696f6e616c2061726770617273652066616c6c6261636b0a202020202020202020202020706172736572203d2061726770617273652e417267756d656e7450617273657228290a2020202020202020202020207061727365722e6164645f617267756d656e7428222d2d616374696f6e222c2063686f696365733d5b226765745f74657874222c2022636c69636b222c20226c61756e6368222c2022736561726368222c20226765745f7374617465222c202277616974225d290a2020202020202020202020207061727365722e6164645f617267756d656e7428222d2d617070222c2064656661756c743d2261637469766522290a2020202020202020202020207061727365722e6164645f617267756d656e7428222d2d74657874222c2068656c703d225465787420746f2073656172636820666f72206f7220636c69636b22290a2020202020202020202020200a202020202020202020202020617267732c20756e6b6e6f776e203d207061727365722e70617273655f6b6e6f776e5f61726773287261775f61726773290a202020202020202020202020616374696f6e203d20617267732e616374696f6e0a202020202020202020202020617070203d20617267732e6170700a20202020202020202020202074657874203d20617267732e746578740a0a202020206966206e6f7420616374696f6e3a0a20202020202020207072696e74286622e29d8c204e6f20616374696f6e2073706563696669656420696e20617267756d656e74733a207b7261775f617267737d22290a20202020202020207379732e657869742831290a0a20202020696620616374696f6e203d3d20226c61756e6368223a0a20202020202020206966206c61756e63685f6170705f7361666528617070293a0a2020202020202020202020207072696e74286622e29c85204c61756e63686564207b6170707d2e2057616974696e6720666f722077696e646f772e2e2e22290a20202020202020202020202074696d652e736c6565702835290a2020202020202020202020207379732e657869742830290a2020202020202020656c73653a0a2020202020202020202020207072696e74286622e29d8c204661696c656420746f206c61756e6368207b6170707d22290a2020202020202020202020207379732e657869742831290a2020202020202020202020200a20202020696620616374696f6e203d3d202277616974223a0a20202020202020206475726174696f6e203d20312e300a20202020202020207472793a0a202020202020202020202020696620746578743a0a202020202020202020202020202020206475726174696f6e203d20666c6f617428746578742e73706c697428295b305d290a20202020202020206578636570743a20706173730a20202020202020207072696e74286622e28fb1efb88f2057616974696e6720666f72207b6475726174696f6e7d732e2e2e22290a202020202020202074696d652e736c656570286475726174696f6e290a20202020202020207379732e657869742830290a0a20202020696620616374696f6e203d3d2022736561726368223a0a20202020202020206966206e6f7420746578743a0a2020202020202020202020207072696e742822e29d8c20536561726368207265717569726573207465787422290a2020202020202020202020207379732e657869742831290a2020202020202020202020200a20202020202020207072696e74286622f09f948d20536561726368696e6720666f7220277b746578747d2720696e207b6170707d2e2e2e22290a202020202020202023205374657020313a2046696e6420616e6420636c69636b2053656172636820627574746f6e206f72206261720a2020202020202020726573756c7473203d206a736f6e2e6c6f616473286765745f73637265656e5f746578742861707029290a20202020202020207365617263685f62746e203d204e6f6e650a20202020202020200a202020202020202023204c6f6f6b20666f722073656172636820696e707574206c616e646d61726b730a202020202020202023205765207072696f726974697a652022736561726368206d757369632e2e22206f7220227365617263682e2e2e22206f7665722067656e6572696320226e656f6e74756e65220a20202020202020207365617263685f63616e64696461746573203d205b7220666f72207220696e20726573756c747320696620227365617263682220696e20725b2774657874275d2e6c6f77657228295d0a20202020202020206966206e6f74207365617263685f63616e646964617465733a0a202020202020202020202020207365617263685f63616e64696461746573203d205b7220666f72207220696e20726573756c747320696620616e79287720696e20725b2774657874275d2e6c6f776572282920666f72207720696e205b22616c6c206d75736963222c20226e656f6e74756e65225d295d0a20202020202020200a20202020202020206966207365617263685f63616e646964617465733a0a2020202020202020202020202320507265666572656e63653a0a2020202020202020202020202320312e204578616374206d6174636820666f722022736561726368206d757369632e2e22206f722073696d696c617220636f6d6d6f6e20706c616365686f6c646572730a2020202020202020202020202320322e20436f6e7461696e7320227365617263682220414e44206973206675727468657220646f776e20286e6f7420617420746865207665727920746f70290a202020202020202020202020706c616365686f6c64657273203d205b7220666f72207220696e207365617263685f63616e6469646174657320696620227365617263682e2e2e2220696e20725b2774657874275d2e6c6f7765722829206f722022736561726368206d757369632220696e20725b2774657874275d2e6c6f77657228295d0a202020202020202020202020696620706c616365686f6c646572733a0a202020202020202020202020202020207365617263685f62746e203d206d696e28706c616365686f6c646572732c206b65793d6c616d62646120723a20725b2779275d29202320546f702d6d6f737420706c616365686f6c646572206966206d756c7469706c650a202020202020202020202020656c73653a0a20202020202020202020202020202020232046696c746572206f7574207468696e677320617420746865206162736f6c75746520746f70202879203c20746f705f6d617267696e29206173207468657920617265206c696b656c79206c6f676f732f686561646572730a20202020202020202020202020202020232077696e5f79206973207468652077696e646f7727732073637265656e20592e20436f6f7264696e61746520696e20726573756c74732069732073637265656e20636f6f7264696e6174652e0a202020202020202020202020202020207769645f696e666f203d204e6f6e650a202020202020202020202020202020207472793a0a2020202020202020202020202020202020202020776964203d206765745f77696e646f775f696428617070290a20202020202020202020202020202020202020205f2c2077696e5f792c205f2c205f203d206765745f77696e646f775f67656f6d6574727928776964290a20202020202020202020202020202020202020207365617263685f63616e64696461746573203d205b7220666f72207220696e207365617263685f63616e6469646174657320696620725b2779275d203e2077696e5f79202b2038305d0a202020202020202020202020202020206578636570743a20706173730a202020202020202020202020202020200a202020202020202020202020202020206966207365617263685f63616e646964617465733a0a20202020202020202020202020202020202020207365617263685f62746e203d206d696e287365617263685f63616e646964617465732c206b65793d6c616d62646120723a20725b2779275d290a20202020202020202020202020202020656c73653a0a2020202020202020202020202020202020202020232046616c6c6261636b20746f206162736f6c75746520746f702d6d6f7374206966206e6f7468696e6720656c73650a20202020202020202020202020202020202020207365617263685f62746e203d206d696e287365617263685f63616e646964617465732c206b65793d6c616d62646120723a20725b2779275d290a20202020202020200a20202020202020206966207365617263685f62746e3a0a2020202020202020202020207072696e74286622f09f96b1efb88f20436c69636b696e67202753656172636827206c616e646d61726b20617420287b7365617263685f62746e5b2778275d7d2c207b7365617263685f62746e5b2779275d7d2922290a20202020202020202020202070796175746f6775692e636c69636b287365617263685f62746e5b2778275d2c207365617263685f62746e5b2779275d290a20202020202020202020202074696d652e736c65657028312e30292023205761697420666f7220666f63757320746f20736574746c650a20202020202020202020202023205374657020323a20416767726573736976656c7920436c65617220616e6420547970652071756572790a2020202020202020202020202320547279206d756c7469706c652074696d657320746f20656e7375726520666f63757320616e642073656c656374696f6e0a202020202020202020202020666f72205f20696e2072616e67652832293a0a2020202020202020202020202020202070796175746f6775692e686f746b657928276374726c272c20276127290a2020202020202020202020202020202074696d652e736c65657028302e33290a2020202020202020202020202020202070796175746f6775692e707265737328276261636b737061636527290a2020202020202020202020202020202074696d652e736c65657028302e33290a2020202020202020202020200a20202020202020202020202070796175746f6775692e777269746528746578742c20696e74657276616c3d302e3129202320536c6f77657220747970696e670a20202020202020202020202074696d652e736c65657028302e35290a20202020202020202020202070796175746f6775692e70726573732827656e74657227290a2020202020202020202020207072696e74286622e29c8520444f4e453a2053656172636820666f7220277b746578747d27207375626d69747465642e2057616974696e6720666f7220726573756c74732e2e2e22290a20202020202020202020202074696d652e736c65657028322e35292023204d6f72652074696d6520666f7220726573756c74730a2020202020202020656c73653a0a202020202020202020202020232046616c6c6261636b3a204a757374207479706520696620536561726368206669656c64206973206c696b656c7920666f6375736564206f722077652063616e27742066696e642069740a2020202020202020202020207072696e742822e29aa0efb88f202753656172636827206c616e646d61726b206e6f7420666f756e6420766961204f43522c20747279696e672064697265637420747970652066616c6c6261636b732e2e2e22290a202020202020202020202020232054727920636f6d6d6f6e20666f6375732073686f7274637574730a202020202020202020202020666f7220636d6420696e205b276374726c2b66272c20276374726c2b6c272c20276374726c2b6b275d3a0a2020202020202020202020202020202070796175746f6775692e686f746b6579282a636d642e73706c697428272b2729290a2020202020202020202020202020202074696d652e736c65657028302e35290a2020202020202020202020200a2020202020202020202020202320436c65617220616e797761790a20202020202020202020202070796175746f6775692e686f746b657928276374726c272c20276127290a20202020202020202020202074696d652e736c65657028302e33290a20202020202020202020202070796175746f6775692e707265737328276261636b737061636527290a20202020202020202020202074696d652e736c65657028302e33290a2020202020202020202020200a20202020202020202020202070796175746f6775692e777269746528746578742c20696e74657276616c3d302e31290a20202020202020202020202074696d652e736c65657028302e35290a20202020202020202020202070796175746f6775692e70726573732827656e74657227290a2020202020202020202020207072696e74286622e29c8520444f4e453a2046616c6c6261636b2073656172636820666f7220277b746578747d27207375626d69747465642e2057616974696e672e2e2e22290a20202020202020202020202074696d652e736c65657028322e35290a2020202020202020202020200a20202020656c696620616374696f6e203d3d20226765745f74657874223a0a20202020202020207072696e74286765745f73637265656e5f74657874286170702c207465787429290a20202020656c696620616374696f6e203d3d20226765745f7374617465223a0a2020202020202020726573756c7473203d206a736f6e2e6c6f616473286765745f73637265656e5f746578742861707029290a20202020202020206966206973696e7374616e636528726573756c74732c206c697374293a0a20202020202020202020202023204578747261637420756e697175652074657874732c20737472697070656420616e642066696c74657265640a202020202020202020202020756e697175655f7465787473203d20736f72746564286c6973742873657428725b2774657874275d2e7374726970282920666f72207220696e20726573756c747320696620725b2774657874275d2e737472697028292929290a2020202020202020202020207072696e74286a736f6e2e64756d707328756e697175655f746578747329290a2020202020202020656c73653a0a2020202020202020202020207072696e74286a736f6e2e64756d7073285b5d29290a20202020656c696620616374696f6e203d3d2022636c69636b223a0a20202020202020206966206e6f7420746578743a0a2020202020202020202020207072696e742822e29d8c202d2d7465787420726571756972656420666f7220636c69636b20616374696f6e22290a2020202020202020202020207379732e657869742831290a20202020202020200a20202020202020207461726765745f74657874203d20746578740a20202020202020207069636b5f6c617374203d2046616c73650a2020202020202020696620746578742e6c6f77657228292e7374617274737769746828226c6173742022293a0a2020202020202020202020207461726765745f74657874203d20746578745b353a5d0a2020202020202020202020207069636b5f6c617374203d20547275650a2020202020202020202020200a20202020202020202320436865636b20666f7220274e6f20726573756c747320666f756e64272066697273742069662077652061726520646f696e67206120636c69636b206166746572207365617263680a202020202020202073637265656e5f636f6e74656e74203d206765745f73637265656e5f7465787428617070290a2020202020202020696620224e6f20726573756c747320666f756e642220696e2073637265656e5f636f6e74656e743a0a202020202020202020202020207072696e74286622e29d8c204572726f723a20274e6f20726573756c747320666f756e6427206465746563746564206f6e2073637265656e2e2043616e6e6f7420636c69636b20277b7461726765745f746578747d2722290a202020202020202020202020207379732e657869742831290a0a2020202020202020726573756c7473203d206a736f6e2e6c6f6164732873637265656e5f636f6e74656e74290a2020202020202020232046696c74657220726573756c747320666f72207461726765742074657874206c6f63616c6c7920746f2073617665204f43522063616c6c730a20202020202020206d617463686573203d205b7220666f72207220696e20726573756c7473206966207461726765745f746578742e6c6f776572282920696e20725b2774657874275d2e6c6f77657228295d0a20202020202020200a20202020202020206966206d6174636865733a0a2020202020202020202020206966207069636b5f6c6173743a0a202020202020202020202020202020202320416d62696775697479207265736f6c7574696f6e3a0a202020202020202020202020202020202320312e20536f7274206279205920636f6f7264696e6174652064657363656e64696e672028626f74746f6d2d6d6f7374206669727374290a202020202020202020202020202020202320322e204578636c75646520616e797468696e672074686174206c6f6f6b73206c696b65206974277320696e2074686520746f7020686561646572202879203c2077696e5f79202b203130302920696620706f737369626c650a20202020202020202020202020202020776964203d204e6f6e650a202020202020202020202020202020206966206170702e6c6f7765722829203d3d2022616374697665223a0a20202020202020202020202020202020202020207472793a20776964203d2073756270726f636573732e636865636b5f6f7574707574285b2778646f746f6f6c272c202767657461637469766577696e646f77275d292e6465636f646528292e737472697028290a20202020202020202020202020202020202020206578636570743a20706173730a20202020202020202020202020202020656c73653a0a2020202020202020202020202020202020202020776964203d206765745f77696e646f775f696428617070290a202020202020202020202020202020200a2020202020202020202020202020202077696e5f79203d20300a202020202020202020202020202020206966207769643a0a20202020202020202020202020202020202020205f2c2077696e5f792c205f2c205f203d206765745f77696e646f775f67656f6d6574727928776964290a202020202020202020202020202020200a20202020202020202020202020202020232046696c746572206f7574206974656d73207468617420617265206c696b656c79207468652073656172636820626172207465787420697473656c662028757375616c6c792061742074686520746f70290a202020202020202020202020202020202320696620746865726520617265206974656d73206675727468657220646f776e2e0a2020202020202020202020202020202063616e64696461746573203d205b6d20666f72206d20696e206d617463686573206966206d5b2779275d203e2077696e5f79202b203132305d0a202020202020202020202020202020206966206e6f742063616e646964617465733a0a202020202020202020202020202020202020202063616e64696461746573203d206d61746368657320232046616c6c6261636b20746f20616c6c206d617463686573206966206e6f6e652062656c6f77206865616465720a202020202020202020202020202020200a20202020202020202020202020202020746172676574203d206d61782863616e646964617465732c206b65793d6c616d62646120723a20725b2779275d290a202020202020202020202020656c73653a0a20202020202020202020202020202020746172676574203d206d6174636865735b305d0a202020202020202020202020202020200a2020202020202020202020207072696e74286622f09f96b1efb88f20436c69636b696e6720277b7461726765745b2774657874275d7d2720617420287b7461726765745b2778275d7d2c207b7461726765745b2779275d7d2922290a20202020202020202020202070796175746f6775692e636c69636b287461726765745b2778275d2c207461726765745b2779275d290a2020202020202020202020207072696e74286622e29c8520535543434553533a20436c69636b656420277b7461726765745b2774657874275d7d2722290a2020202020202020656c73653a0a2020202020202020202020207072696e74286622e29d8c205465787420277b7461726765745f746578747d27206e6f7420666f756e6422290a0a6966205f5f6e616d655f5f203d3d20225f5f6d61696e5f5f223a0a202020206d61696e28290a', 'python', '1.0.0', NULL, '2026-03-14 06:42:50', '2026-03-14 06:42:50', 'agents-scripts/desktop/paddle_ui_automation.py', NULL, 'standard');



INSERT INTO tools VALUES(7438474706694574084, 'verify_layout', 'verify_layout tool for desktop domain', 'import sys
import time
import subprocess
import pyautogui
import os

# Reuse functions from ui_automation.py implicitly or re-define
def reset_window_layout(window_name="tao window"):
    print(f"📏 Resetting ''{window_name}''...")
    subprocess.run(["wmctrl", "-r", window_name, "-b", "remove,maximized_vert,maximized_horz,fullscreen"], check=False)
    subprocess.run(["wmctrl", "-r", window_name, "-e", "0,0,0,1280,800"], check=True)
    subprocess.run(["wmctrl", "-R", window_name], check=True)
    time.sleep(1)

def run_debug(text):
    # 1. Launch/Focus
    print(f"🚀 Launching/Focusing Neontune...")
    subprocess.Popen(["neontune"])
    time.sleep(2)
    reset_window_layout("tao window")
    
    # 2. Click Search (400, 100)
    print("🖱️ Clicking Search (400, 100)")
    pyautogui.click(400, 100)
    time.sleep(0.5)
    
    # 3. Type
    print(f"⌨️ Typing ''{text}''...")
    pyautogui.hotkey(''ctrl'', ''a'')
    pyautogui.press(''backspace'')
    pyautogui.write(text, interval=0.1)
    pyautogui.press(''enter'')
    
    # 4. Wait
    print("⏳ Waiting 3s for results...")
    time.sleep(3)
    
    # 5. Screenshot
    print("📸 Taking debug screenshot...")
    subprocess.run(["import", "-window", "root", "neontune_debug.png"])
    
    # 6. OCR interactively?
    # We will let the agent run tesseract separately
    print("✅ Done. Check neontune_debug.png")

if __name__ == "__main__":
    run_debug("Tali Rush - U")
', 'python', '1.0.0', NULL, '2026-03-14 06:42:50', '2026-03-14 06:42:50', 'agents-scripts/desktop/verify_layout.py', NULL, 'standard');



INSERT INTO tools VALUES(7438474706694574085, 'find_search_field', 'find_search_field tool for desktop domain', '#!/usr/bin/env python3
"""
Test script to find the correct search field coordinates in Neontune.
This will click different locations and wait for you to verify which one worked.
"""

import pyautogui
import subprocess
import time

# Launch Neontune
print("🚀 Launching Neontune...")
subprocess.Popen([''neontune''])
time.sleep(3)

# Test different coordinates
test_coords = [
    (400, 100, "Top center"),
    (500, 150, "Slightly right and down"),
    (300, 100, "Slightly left"),
    (400, 80, "Higher up"),
    (400, 120, "Lower down"),
]

for x, y, desc in test_coords:
    print(f"\\n🖱️ Clicking {desc} at ({x}, {y})")
    print("   Watch where the cursor goes...")
    pyautogui.click(x, y)
    time.sleep(2)
    
    response = input("   Did this click the search field? (y/n): ")
    if response.lower() == ''y'':
        print(f"\\n✅ Found it! Search field is at ({x}, {y})")
        print(f"   Update ui_automation.py line 83: click_search_field({x}, {y})")
        break
else:
    print("\\n❌ None of the coordinates worked.")
    print("   The search field might be in a different location.")
    print("   Try running: xdotool getmouselocation")
    print("   Then hover over the search field and note the coordinates.")
', 'python', '1.0.0', NULL, '2026-03-14 06:42:50', '2026-03-14 06:42:50', 'agents-scripts/desktop/find_search_field.py', NULL, 'standard');



INSERT INTO tools VALUES(7438474706694574086, 'verify_window_capture', 'verify_window_capture tool for desktop domain', 'import subprocess
import time
import os

def check_capture():
    print("🔍 Searching for ''tao'' window...")
    try:
        # Find window ID
        wid_bytes = subprocess.check_output([''xdotool'', ''search'', ''--name'', ''tao''])
        wids = wid_bytes.decode().strip().split(''\\n'')
        
        if not wids:
            print("❌ No ''tao'' window found.")
            return
            
        wid = wids[-1] # Take last one (usually the active one)
        print(f"✅ Found Window ID: {wid}")
        
        # Get Geometry
        geo = subprocess.check_output([''xdotool'', ''getwindowgeometry'', wid]).decode()
        print(f"📏 Geometry:\\n{geo}")

        # Capture SPECIFIC window
        output_file = "neontune_window.png"
        print(f"📸 Capturing Window {wid} to {output_file}...")
        subprocess.run([''import'', ''-window'', wid, output_file])
        
        # OCR
        print("📝 Running OCR...")
        subprocess.run([''tesseract'', output_file, ''neontune_ocr'', ''hocr''])
        subprocess.run([''cat'', ''neontune_ocr.hocr'']) # Dump raw for quick peek

    except subprocess.CalledProcessError as e:
        print(f"❌ Commands failed: {e}")
    except Exception as e:
        print(f"❌ Error: {e}")

if __name__ == "__main__":
    check_capture()
', 'python', '1.0.0', NULL, '2026-03-14 06:42:50', '2026-03-14 06:42:50', 'agents-scripts/desktop/verify_window_capture.py', NULL, 'standard');



INSERT INTO tools VALUES(7438474706694574087, 'ocr_coordinates', 'ocr_coordinates tool for desktop domain', 'import sys
import re
import xml.etree.ElementTree as ET

def find_all_coords(file_path, target_word):
    coords_list = []
    try:
        tree = ET.parse(file_path)
        root = tree.getroot()
        for elem in root.iter():
            if ''ocrx_word'' in (elem.get(''class'') or ""):
                text = elem.text or ""
                if target_word.lower() in text.lower():
                    title = elem.get(''title'')
                    match = re.search(r''bbox (\\d+) (\\d+) (\\d+) (\\d+)'', title)
                    if match:
                        x1, y1, x2, y2 = map(int, match.groups())
                        cx, cy = (x1 + x2) // 2, (y1 + y2) // 2
                        coords_list.append((cx, cy))
    except Exception as e:
        print(f"Error parsing HOCR: {e}")
    return coords_list

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: script.py <word>")
        sys.exit(1)
        
    coords = find_all_coords("hocr_results.hocr", sys.argv[1])
    for c in coords:
        print(f"{c[0]} {c[1]}")
', 'python', '1.0.0', NULL, '2026-03-14 06:42:50', '2026-03-14 06:42:50', 'agents-scripts/desktop/ocr_coordinates.py', NULL, 'standard');



INSERT INTO tools VALUES(7438474706694574088, 'vision_debug', 'vision_debug tool for desktop domain', 'import urllib.request
import json
import base64
import sys

def analyze_image(image_path, prompt):
    try:
        with open(image_path, "rb") as f:
            image_base64 = base64.b64encode(f.read()).decode("utf-8")
    except FileNotFoundError:
        return "Error: File not found"

    url = "http://localhost:11434/api/generate"
    data = {
        "model": "qwen2.5vl:3b",
        "prompt": prompt,
        "images": [image_base64],
        "stream": False, 
        "format": "json"
    }

    try:
        req = urllib.request.Request(url, json.dumps(data).encode("utf-8"), headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(req) as response:
            return json.loads(response.read().decode("utf-8"))["response"]
    except Exception as e:
        return f"Request failed: {e}"

print("Analyzing Search Field...")
# Ask for specific point format
print(analyze_image("screenshot.png", "Return a JSON object with a single key ''point'' containing [x, y] coordinates for the center of the search input field in the application window."))

print("\\nAnalyzing First Result...")
print(analyze_image("screenshot.png", "Return a JSON object with a single key ''point'' containing [x, y] coordinates for the center of the first song row in the list below the search bar."))
', 'python', '1.0.0', NULL, '2026-03-14 06:42:50', '2026-03-14 06:42:50', 'agents-scripts/desktop/vision_debug.py', NULL, 'standard');



INSERT INTO tools VALUES(7438474706694574089, 'ui_automation', 'ui_automation tool for desktop domain', '#!/usr/bin/env python3
"""
Adaptive UI Automation script using OCR.
Finds ''Search'' and specific text on screen dynamically.
"""

import sys
import time
import subprocess
import pyautogui
import argparse
import os
import re
import json
import hashlib
import tempfile
# --- PADDLEOCR INTEGRATION ---
# Always use PaddleOCR as requested by the user. 
os.environ["PADDLE_PDX_DISABLE_MODEL_SOURCE_CHECK"] = "True"
os.environ["FLAGS_use_onednn"] = "0"
os.environ["FLAGS_use_mkldnn"] = "0"

try:
    from paddleocr import PaddleOCR
    # Final production-grade initialization
    paddle_ocr_engine = PaddleOCR(lang=''en'', use_angle_cls=False, use_gpu=False, show_log=False)
    print("🌀 Initialized PaddleOCR engine backend.", file=sys.stderr)
except Exception as e:
    print(f"❌ CRITICAL: Failed to init PaddleOCR: {e}. Automation will fail.", file=sys.stderr)
    sys.exit(1)

import gi
try:
    gi.require_version(''Atspi'', ''2.0'')
    from gi.repository import Atspi
except ImportError:
    Atspi = None
except ValueError:
    Atspi = None

# Disable pyautogui failsafe for automation
pyautogui.FAILSAFE = False


# word_pattern = re.compile(r''title=[\\''"]bbox (\\d+) (\\d+) (\\d+) (\\d+ \\s*x_wconf \\d+[\\''"]>\\s*([^<]+)\\s*</span>'')

DEBUG_DRAW = False

CONFIG_PATH = os.path.expanduser("~/.config/rig/apps.json")

def load_config():
    if os.path.exists(CONFIG_PATH):
        try:
            with open(CONFIG_PATH, ''r'') as f:
                return json.load(f)
        except:
            pass
    return {}

def get_window_geometry(wid):
    """Returns (x, y, width, height) of the window."""
    try:
        out = subprocess.check_output([''xdotool'', ''getwindowgeometry'', str(wid)]).decode()
        # Window 83886092
        #   Position: 166,132 (screen: 0)
        #   Geometry: 800x600
        pos_line = [l for l in out.split(''\\n'') if ''Position:'' in l][0]
        geo_line = [l for l in out.split(''\\n'') if ''Geometry:'' in l][0]
        
        x, y = map(int, re.search(r''Position: (\\d+),(\\d+)'', pos_line).groups())
        w, h = map(int, re.search(r''Geometry: (\\d+)x(\\d+)'', geo_line).groups())
        return x, y, w, h
    except:
        return 0, 0, 0, 0

def get_window_id(window_title):
    try:
        # Get all IDs matching the name
        out = subprocess.check_output([''xdotool'', ''search'', ''--name'', window_title])
        wids = out.decode().strip().split(''\\n'')
        
        if not wids: return None
        
        # Filter: Find the LARGEST window (Main Application Window)
        best_wid = None
        max_area = 0
        
        for wid in wids:
            try:
                _, _, w, h = get_window_geometry(wid)
                area = w * h
                # Ignore tiny windows (tooltips, invisible proxies)
                if area > max_area and w > 200 and h > 200:
                    max_area = area
                    best_wid = wid
            except:
                continue
                
        # Fallback: If no "large" window found, return the last one (legacy behavior)
        return best_wid if best_wid else wids[-1]
    except:
        return None


def get_screen_text(app_name, grep_str=None):
    """Captures window and returns JSON list of all text with coords, optionally filtering."""
    print(f"📖 Reading screen text for {app_name} (grep: {grep_str})...", file=sys.stderr)
    
    wid = None
    if app_name.lower() == "active":
        try:
            wid = subprocess.check_output([''xdotool'', ''getactivewindow'']).decode().strip()
        except:
            pass
    else:
        config = load_config()
        profile = config.get(app_name.lower(), {})
        win_title = resolve_window_title(app_name, config)
        wid = focus_window(win_title)

    if not wid:
        print(f"❌ Could not focus {app_name}.")
        return json.dumps({"error": "Window not found"})
        
    time.sleep(0.5)
    
    win_x, win_y, _, _ = get_window_geometry(wid)
    
    try:
        tmp_img = "/tmp/rig_ui_capture.png"
        subprocess.run([''import'', ''-window'', wid, tmp_img], check=True)
        
        words = []
        if paddle_ocr_engine:
            # Use PaddleOCR backend (EXCLUSIVE)
            result = paddle_ocr_engine.ocr(tmp_img)
            if result and result[0]:
                for line in result[0]:
                    if not line or len(line) < 2: continue
                    points = line[0] # [[x1, y1], [x2, y1], [x2, y2], [x1, y2]]
                    text = line[1][0]
                    conf = line[1][1]
                    
                    x0 = int(min(p[0] for p in points))
                    y0 = int(min(p[1] for p in points))
                    x1 = int(max(p[0] for p in points))
                    y1 = int(max(p[1] for p in points))
                    
                    words.append({
                        "text": text.strip(),
                        "x0": x0, "y0": y0,
                        "x1": x1, "y1": y1,
                        "confidence": float(conf)
                    })
        else:
             return json.dumps({"error": "PaddleOCR engine unavailable."})

        lines = []
        
        words.sort(key=lambda w: (w["y0"], w["x0"]))

        for word in words:
            if grep_str and grep_str.lower() not in word["text"].lower():
                continue
                
            # Find a line that this word belongs to (vertical overlap)
            placed = False
            for line in lines:
                # Vertical overlap threshold: at least 50% overlap or within 5 pixels
                line_y_mid = (line["y0"] + line["y1"]) // 2
                word_y_mid = (word["y0"] + word["y1"]) // 2
                
                if abs(line_y_mid - word_y_mid) < 8: # Pixels tolerance for same line
                    line["words"].append(word)
                    line["y0"] = min(line["y0"], word["y0"])
                    line["y1"] = max(line["y1"], word["y1"])
                    line["x0"] = min(line["x0"], word["x0"])
                    line["x1"] = max(line["x1"], word["x1"])
                    placed = True
                    break
            
            if not placed:
                lines.append({
                    "words": [word],
                    "y0": word["y0"], "y1": word["y1"],
                    "x0": word["x0"], "x1": word["x1"]
                })

        final_results = []
        for line in lines:
            sorted_words = sorted(line["words"], key=lambda w: w["x0"])
            full_text = " ".join([w["text"] for w in sorted_words])
            
            # If grep_str is provided, try to find the specific matching word(s) in the line
            # to be more precise with the click.
            target_x, target_y = None, None
            
            if grep_str:
                # Simple check: if the line contains the text, verify which words match
                # This is a heuristic. For now, we still return the line center,
                # but we could refine this to return the center of the MATCHING substring.
                pass

            # Default: Center of the line/block
            target_x = win_x + (line["x0"] + line["x1"]) // 2
            target_y = win_y + (line["y0"] + line["y1"]) // 2
            
            # If searching for specific text, try to find the center of the matching words
            if grep_str and grep_str.lower() in full_text.lower():
                 # Filter words that are part of the match
                 matching_words = [w for w in sorted_words if w["text"].lower() in grep_str.lower() or grep_str.lower() in w["text"].lower()]
                 if matching_words:
                     min_x = min(w["x0"] for w in matching_words)
                     max_x = max(w["x1"] for w in matching_words)
                     min_y = min(w["y0"] for w in matching_words)
                     max_y = max(w["y1"] for w in matching_words)
                     target_x = win_x + (min_x + max_x) // 2
                     target_y = win_y + (min_y + max_y) // 2

            final_results.append({
                "text": full_text,
                "x": target_x,
                "y": target_y,
                "width": line["x1"] - line["x0"],
                "height": line["y1"] - line["y0"]
            })
                
        return json.dumps(final_results)
    except Exception as e:
        return json.dumps({"error": str(e)})

def find_text_matches(wid, target_text):
    """
    Captures window, runs OCR, returns list of (cx, cy, word) relative to SCREEN.
    """
    try:
        tmp_img = "/tmp/rig_ui_capture.png"
        subprocess.run([''import'', ''-window'', wid, tmp_img], check=True)
        
        target_lower = target_text.lower()
        win_x, win_y, _, _ = get_window_geometry(wid)
        results = []

        if paddle_ocr_engine:
            result = paddle_ocr_engine.ocr(tmp_img)
            if result and result[0]:
                for line in result[0]:
                    if not line or len(line) < 2: continue
                    points = line[0]
                    text = line[1][0]
                    if target_lower in text.lower():
                        x0 = min(p[0] for p in points)
                        y0 = min(p[1] for p in points)
                        x1 = max(p[0] for p in points)
                        y1 = max(p[1] for p in points)
                        cx = (x0 + x1) // 2
                        cy = (y0 + y1) // 2
                        screen_x = win_x + cx
                        screen_y = win_y + cy
                        results.append((int(screen_x), int(screen_y), text))
                        print(f"👁️ [Paddle] Found candidate ''{text}'' at Screen({screen_x},{screen_y})")
        else:
            print("❌ PaddleOCR engine not initialized.")

        return results
    except Exception as e:
        print(f"❌ Vision error: {e}")
        return []

def capture_and_find_text(wid, target_text):
    """Legacy wrapper: Returns first match (cx, cy) or (None, None)."""
    matches = find_text_matches(wid, target_text)
    if matches:
        return matches[0][0], matches[0][1]
    return None, None

def focus_window(window_title):
    print(f"🎯 Focusing ''{window_title}''...", file=sys.stderr)
    # 1. Try direct match
    subprocess.run([''wmctrl'', ''-a'', window_title], check=False)
    wid = get_window_id(window_title)
    
    # 2. If not found, try case-insensitive search
    if not wid:
        try:
            output = subprocess.check_output([''wmctrl'', ''-l'']).decode(''utf-8'')
            for line in output.splitlines():
                parts = line.split(maxsplit=3)
                if len(parts) > 3:
                     current_wid = parts[0]
                     current_title = " ".join(parts[3:]) if len(parts) > 3 else parts[3]
                     if window_title.lower() in current_title.lower():
                         print(f"🎯 Found partial match: ''{current_title}''")
                         subprocess.run([''wmctrl'', ''-ia'', current_wid], check=False)
                         # wmctrl gives hex ID (e.g. 0x01a00003), xdotool needs decimal.
                         # Instead of re-searching by name, just convert to decimal!
                         try:
                             wid = str(int(current_wid, 16))
                         except ValueError:
                             wid = get_window_id(current_title) # Fallback to name search
                         break
        except Exception as e:
            print(f"⚠️ Focus search error: {e}")

    if wid:
        subprocess.run([''xdotool'', ''windowactivate'', ''--sync'', wid], check=False)
        subprocess.run([''xdotool'', ''windowraise'', wid], check=False)
        return wid
    
    # 3. Last ditch effort: if app_name is likely a browser, try searching for "Chrome" or "Firefox"
    if window_title.lower() in ["chrome", "google-chrome", "chromium"]:
        for title in ["Google Chrome", "Chromium"]:
            wid = get_window_id(title)
            if wid:
                subprocess.run([''xdotool'', ''windowactivate'', ''--sync'', wid], check=False)
                subprocess.run([''xdotool'', ''windowraise'', wid], check=False)
                return wid
                
    return None

def wait_for_text(app_name, text, timeout=20):
    """Wait until specific text appears on screen"""
    print(f"⏳ Waiting for text ''{text}'' in {app_name} (timeout={timeout}s)...")
    start_time = time.time()
    while time.time() - start_time < timeout:
        screen_text = get_screen_text(app_name)
        if text.lower() in screen_text.lower():
            print(f"✅ Found text ''{text}''!")
            return True
        time.sleep(2)
    print(f"❌ Timeout waiting for ''{text}''")
    return False

def get_atspi_tree(app_name):
    """
    Query the Linux Accessibility Tree (AT-SPI2) for the specified application.
    Returns a list of actionable UI elements with coordinates.
    """
    if Atspi is None:
        return json.dumps({"error": "Atspi not available. Please install python3-gi and at-spi2-core."})

    Atspi.init()
    desktop = Atspi.get_desktop(0)
    if not desktop:
        return json.dumps({"error": "Could not connect to Atspi desktop"})

    target_app = None
    app_name_lower = app_name.lower() if app_name else ""

    if not app_name or app_name_lower == "active":
        # Get the currently focused window
        # Note: Atspi.get_desktop(0) has get_active_window? 
        # Actually it''s simpler to iterate and find the one with ''active'' state, 
        # but let''s try to match the window title from xdotool if we can.
        # For robustness, let''s search for the window that has STATE_ACTIVE
        for i in range(desktop.get_child_count()):
            app = desktop.get_child_at_index(i)
            if not app: continue
            for j in range(app.get_child_count()):
                win = app.get_child_at_index(j)
                if not win: continue
                try:
                    states = win.get_state_set().get_states()
                    if Atspi.StateType.ACTIVE in states:
                        target_app = app # We want the App, not just the window, usually
                        # But actually we want to crawl the window.
                        # Let''s return the app for now as the root.
                        break
                except: pass
            if target_app: break
            
        if not target_app:
             return json.dumps({"error": "No active window found and no app name provided."})
    else:
        # Find the application in the desktop tree by name
        for i in range(desktop.get_child_count()):
            child = desktop.get_child_at_index(i)
            if not child: continue
            
            child_name = (child.get_name() or "").lower()
            if app_name_lower in child_name or child_name in app_name_lower:
                target_app = child
                break

    if not target_app:
        # Fallback: Deep search for window title
        print(f"⚠️ App ''{app_name}'' not found by name, deep searching...", file=sys.stderr)
        for i in range(desktop.get_child_count()):
            child = desktop.get_child_at_index(i)
            if not child: continue
            for j in range(child.get_child_count()):
                grandchild = child.get_child_at_index(j)
                if not grandchild: continue
                grand_name = (grandchild.get_name() or "").lower()
                if app_name_lower in grand_name:
                    target_app = child
                    break
            if target_app: break

    if not target_app:
        return [] # Return empty list, caller will fallback to OCR

    elements = []
    
    # Get window bounds for viewport tagging
    win_x, win_y, win_w, win_h = 0, 0, 1920, 1080 # Fallback
    try:
        # If active window, try to find the actual window node to get its extents
        for i in range(target_app.get_child_count()):
            child = target_app.get_child_at_index(i)
            if not child: continue
            role = child.get_role_name()
            if role in ["frame", "window"]:
                comp = child.get_component_iface()
                if comp:
                    bbox = comp.get_extents(Atspi.CoordType.SCREEN)
                    if bbox.width > 1:
                        win_x, win_y, win_w, win_h = bbox.x, bbox.y, bbox.width, bbox.height
                        break
    except: pass

    def crawl(obj, depth=0, path=""):
        if depth > 15: return # Safety limit

        try:
            name = (obj.get_name() or "").strip()
            role = obj.get_role_name()
            
            new_path = f"{path} > {role}" if path else role
            
            # Interesting roles for automation
            interesting_roles = [
                "push button", "text", "link", "menu item", "check box", 
                "radio button", "entry", "combo box", "toggle button", "page tab", "label",
                "document", "web area", "heading", "list item"
            ]
            
            container_roles = ["frame", "panel", "application", "window", "filler", "scroll pane", "layered pane", "root pane", "split pane"]

            # Check visibility and state
            state_set = obj.get_state_set()
            is_visible = state_set.contains(Atspi.StateType.VISIBLE) if state_set else False
            is_showing = state_set.contains(Atspi.StateType.SHOWING) if state_set else False
            
            # ALWAYS crawl containers, even if they report as invisible (Gtk boxes often do)
            should_crawl_children = is_visible or is_showing or role in container_roles or role in ["document", "web area"]

            if (role in interesting_roles or (name and len(name) > 0)) and (is_visible or is_showing):
                # Get coordinates
                try:
                    comp = obj.get_component_iface()
                    if comp:
                        bbox = comp.get_extents(Atspi.CoordType.SCREEN)
                        # Validate coordinates (Ignore off-screen/invalid junk)
                        if bbox.width > 1 and bbox.height > 1 and bbox.x > -5000 and bbox.y > -5000:
                            cx = bbox.x + bbox.width // 2
                            cy = bbox.y + bbox.height // 2
                            
                            # VIEWPORT TAGGING
                            visibility = "visible"
                            if not (win_x <= cx <= win_x + win_w and win_y <= cy <= win_y + win_h):
                                visibility = "off-screen"
                                
                            elements.append({
                                "text": name,
                                "role": role,
                                "path": new_path,
                                "x": cx,
                                "y": cy,
                                "width": bbox.width,
                                "height": bbox.height,
                                "visibility": visibility
                            })
                except:
                    pass

            if should_crawl_children:
                # Recursively crawl children with "Lazy-Load Poke"
                count = obj.get_child_count()
                if count == 0 and depth < 3:
                    # Some apps (Chrome) lazy load children. Poke them.
                    time.sleep(0.05) # Shorter poke
                    count = obj.get_child_count()

                for k in range(count):
                    child_obj = obj.get_child_at_index(k)
                    if child_obj:
                        crawl(child_obj, depth + 1, new_path)
        except:
            pass

    crawl(target_app)
    
    # Chrome-specific: If we only got the frame, try a deeper poke
    if len(elements) <= 1 and app_name_lower == "chrome":
        # Force a refresh by looking for Document/Web Area
        def force_poke(obj):
            for i in range(obj.get_child_count()):
                child = obj.get_child_at_index(i)
                if child:
                    role = child.get_role_name()
                    if role in ["document", "web area", "page"]:
                        return child
                    res = force_poke(child)
                    if res: return res
            return None
        
        doc = force_poke(target_app)
        if doc:
            crawl(doc) # Recrawl from the document root

    # Calculate Visual Hash using Screenshot
    visual_hash = "0" * 32
    try:
        import hashlib
        config = load_config()
        profile = config.get(app_name.lower(), {})
        win_title = resolve_window_title(app_name, config)
        
        # Try to match the currently found app if possible, or fall back to name
        wid = None
        if app_name.lower() == "active":
             try:
                 wid = subprocess.check_output([''xdotool'', ''getactivewindow'']).decode().strip()
             except: pass
        else:
             wid = get_window_id(win_title)
             if not wid and app_name.lower() == "chrome":
                  wid = get_window_id("Google Chrome")

        if wid:
            tmp_shot = "/tmp/rig_visual_hash.png"
            # Use import -window (fast)
            subprocess.run([''import'', ''-window'', wid, ''-resize'', ''640x480'', tmp_shot], 
                         check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            
            with open(tmp_shot, ''rb'') as f:
                visual_hash = hashlib.md5(f.read()).hexdigest()
    except Exception as e:
        # print(f"⚠️ Visual hash failed: {e}", file=sys.stderr)
        pass

    return {
        "elements": elements,
        "visual_hash": visual_hash
    }

def get_state_delta(app_name, old_tree):
    """
    Returns what changed between old_tree and current tree.
    """
    new_tree = get_atspi_tree(app_name)
    if "error" in new_tree: return new_tree
    
    old_elements = old_tree.get("elements", [])
    new_elements = new_tree.get("elements", [])
    
    # Compare Visual Hashes First
    if old_tree.get("visual_hash") != new_tree.get("visual_hash"):
         # Visual change detected! Even if elements are same/similar.
         # But wait, cursor blinking causes hash change?
         # resizing 640x480 mitigates small noise, but blinking cursor might still trigger?
         # Ideally we ignore small diffs. MD5 is strict.
         # For now, let''s treat it as a "Potential Change".
         pass

    # Standard Element Diff
    old_set = {f"{e[''role'']}:{e[''text'']}" for e in old_elements}
    new_set = {f"{e[''role'']}:{e[''text'']}" for e in new_elements}
    
    added = [e for e in new_elements if f"{e[''role'']}:{e[''text'']}" not in old_set]
    removed_count = len([e for e in old_elements if f"{e[''role'']}:{e[''text'']}" not in new_set])
    
    status = "Stable"
    if added or removed_count > 0:
        status = "Changed"
        
    return {
        "status": status,
        "added_count": len(added),
        "added_preview": added[:5],
        "removed_count": removed_count,
        "visual_hash": new_tree.get("visual_hash")
    }

def search_in_app(app_name, query, no_launch=False):
    """
    Tries multiple strategies to search:
    1. CTR+F -> Type -> Enter (Generic)
    2. OCR "Search" -> Click -> Type -> Enter
    3. Heuristic Top-Right Click -> Type -> Enter
    4. Browser Ctrl+L -> Type -> Enter
    """
    print(f"🔍 Searching for ''{query}'' in {app_name} (Hybrid Mode)...", file=sys.stderr)
    
    # 0. Focus App
    config = load_config()
    profile = config.get(app_name.lower(), {})
    win_title = resolve_window_title(app_name, config)
    
    wid = focus_window(win_title)
    if not wid:
        if no_launch:
            print(f"❌ ''{app_name}'' not running and --no-launch check failed.", file=sys.stderr)
            return False
        # Launch
        if not launch_app_safe(app_name):
            return False
        wid = focus_window(win_title)
        if not wid:
            print("❌ Failed to launch/focus app.", file=sys.stderr)
            return False

    time.sleep(1.0) # Wait for focus
    subprocess.run([''xdotool'', ''windowraise'', wid], check=False)

    # Layer 1: Generic Ctrl+F (Most Apps)
    # But wait, browsers use Ctrl+L for address bar, or Ctrl+F for find in page.
    # We want GLOBAL search usually?
    # Context matters. For "Search for text", Ctrl+F is fine.
    # For "Search web", Ctrl+L is better.
    # Let''s try to detect context or just try Ctrl+F first?
    # Actually OCR is cleaner.
    
    # Layer 2: OCR "Search"
    print("👉 Layer 2: OCR Search Button", file=sys.stderr)
    # Look for "Search", "Find", "Start"
    for kw in ["Search", "Find", "Query"]:
        fx, fy = capture_and_find_text(wid, kw)
        if fx:
            print(f"✅ OCR found ''{kw}'' at ({fx}, {fy}).")
            pyautogui.click(fx, fy)
            time.sleep(0.5)
            # Clear text if needed?
            pyautogui.hotkey(''ctrl'', ''a'')
            pyautogui.write(query, interval=0.05)
            pyautogui.press(''enter'')
            print("✅ Search submitted via OCR.")
            return True

    # Layer 3: Heuristic (Top-Right / Search Icon Area)
    # Most apps have search in top-right or top-center
    print("👉 Layer 3: Heuristic Search (Top-Right)", file=sys.stderr)
    try:
        wx, wy, w, h = get_window_geometry(wid)
        
        # Target: Top Right, with padding
        # Often search icon is ~50px from right, ~30-50px from top
        h_x = wx + w - 50
        h_y = wy + 40 
        
        # Verify safety
        if validate_click_coordinates(h_x, h_y, app_name):
            print(f"⚠️ Attempting heuristic click at Top-Right ({h_x}, {h_y})...")
            # Move slowly
            subprocess.run(["xdotool", "mousemove", str(h_x), str(h_y)])
            time.sleep(0.5)
            pyautogui.click(h_x, h_y)
            time.sleep(0.5)
            # CLEAR BEFORE SEARCH
            pyautogui.hotkey(''ctrl'', ''a'')
            pyautogui.press(''backspace'')
            time.sleep(0.1)
            pyautogui.write(query, interval=0.05)
            pyautogui.press(''enter'')
            print("✅ Search submitted via Heuristic.")
            # We can''t be sure, but we tried.
            return True
    except Exception as e:
        print(f"⚠️ Layer 3 failed: {e}")

    # Layer 4: Browser Specific (Ctrl+L)
    # Only if it LOOKS like a browser or explicitly allowed
    is_browser = app_name.lower() in ["chrome", "firefox", "chromium", "brave", "edge", "browser"]
    if is_browser:
        print("👉 Layer 4: Browser Fallback (Ctrl+L)", file=sys.stderr)
        pyautogui.hotkey(''ctrl'', ''l'')
        time.sleep(0.5)
        # Ctrl+L usually selects all, but let''s be safe
        pyautogui.press(''backspace'')
        pyautogui.write(query)
        pyautogui.press(''enter'')
        print("✅ Search submitted via Browser Shortcut.")
        return True

    print("❌ All search strategies failed.")
    return False 

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--action", required=True, help="Action to perform")
    parser.add_argument("--text", help="Text to type")
    parser.add_argument("--app", default="neontune", help="Application name")
    
    parser.add_argument("--x", type=int, help="Click X")
    parser.add_argument("--y", type=int, help="Click Y")
    parser.add_argument("--key", help="Key to press (e.g. enter, tab)")
    parser.add_argument("--no-launch", action="store_true", help="Do not launch if app not found")
    parser.add_argument("--clear", action="store_true", help="Clear field before typing (Ctrl+A, Backspace)")
    parser.add_argument("--submit", action="store_true", help="Press Enter after typing")
    
    # Optional coordinate overrides (legacy support or manual override)
    parser.add_argument("--sx", type=int, help="Search X")
    parser.add_argument("--sy", type=int, help="Search Y")
    parser.add_argument("--rx", type=int, help="Result X")
    parser.add_argument("--ry", type=int, help="Result Y")
    parser.add_argument("--debug-draw", action="store_true", help="Draw debug box around detected target")
    
    args = parser.parse_args()
    
    if args.debug_draw:
        DEBUG_DRAW = True
    
    if args.action == "play_song":
        # ... (Same as before) ...
        # Can''t easily copy-paste 300 lines. 
        # I should use Sed or partial replace?
        pass # Placeholder




def get_ui_tree_json(app_name):
    # get_atspi_tree now returns a DICT
    elements = get_atspi_tree(app_name)
    return json.dumps(elements)

def get_ui_tree_hash(app_name):
    """Returns a simple hash of the current UI tree structure to detect changes"""
    result = get_atspi_tree(app_name)
    if "error" in result:
        return "error"
    
    # Check visual hash first if available
    if "visual_hash" in result:
        return result["visual_hash"]

    elements = result.get("elements", [])
    # Create a string representation of element names and roles
    signature = "|".join([f"{e[''role'']}:{e[''text'']}" for e in elements])
    import hashlib
    return hashlib.md5(signature.encode()).hexdigest() if hasattr(hashlib, ''md5'') else str(hash(signature))

def wait_for_tree_settle(app_name, timeout=10, stabilization_time=2):
    """Wait until the UI tree structure stops changing"""
    print(f"⏳ Waiting for UI in {app_name} to settle...", file=sys.stderr)
    start_time = time.time()
    last_hash = ""
    stable_since = time.time()
    
    while time.time() - start_time < timeout:
        current_hash = get_ui_tree_hash(app_name)
        if current_hash != last_hash:
            last_hash = current_hash
            stable_since = time.time()
        elif time.time() - stable_since >= stabilization_time:
            print("✅ UI settled.", file=sys.stderr)
            return True
        time.sleep(0.5)
    
    print("⚠️ UI did not settle within timeout.", file=sys.stderr)
    return False

def play_song(app_name, song_query):
    # 1. Config / Defaults
    # We still use apps.json for launch command, but ignore coords if we can find text
    config = load_config()
    profile = config.get(app_name.lower(), {})
    launch_cmd = profile.get("launch_cmd") or [app_name.lower()]
    launch_cmd = profile.get("launch_cmd") or [app_name.lower()]
    win_title = resolve_window_title(app_name, config) # Default to app_name instead of ''tao''
    
    # 2. Launch
    wid = focus_window(win_title)
    if not wid:
        print(f"🚀 Launching {launch_cmd}...")
        subprocess.Popen(launch_cmd)
        time.sleep(5)
        wid = focus_window(win_title)
        if not wid:
            print("❌ Failed to find window after launch.")
            return

    # 3. Find ''Search''
    print("👀 Looking for ''Search'' button...")
    sx, sy = capture_and_find_text(wid, "Search")
    
    if sx:
        print(f"🖱️ Clicking ''Search'' at ({sx}, {sy})...", file=sys.stderr)
        pyautogui.click(sx, sy)
    else:
        print("❌ ''Search'' not found. Aborting to prevent hallucination.", file=sys.stderr)
        return

    # 4. Type
    print(f"⌨️ Typing ''{song_query}''...", file=sys.stderr)
    time.sleep(1) # Wait for animation
    # Clear just in case
    pyautogui.hotkey(''ctrl'', ''a'')
    pyautogui.press(''backspace'')
    
    pyautogui.write(song_query, interval=0.1)
    pyautogui.press(''enter'')
    
    print("⏳ Waiting for results (3s)...", file=sys.stderr)
    time.sleep(3)
    
    # 5. Find Result (Artist or Title)
    # Search is case insensitive fuzzy
    # Split query into potential keywords
    parts = song_query.replace(''-'', '' '').split()
    # Filter short words (avoid matching ''a'', ''the'', ''in'')
    keywords = [w for w in parts if len(w) > 2]
    
    found_x, found_y = None, None
    
    # Try keywords 
    for word in keywords:
        print(f"👀 Looking for result keyword ''{word}''...", file=sys.stderr)
        fx, fy = capture_and_find_text(wid, word)
        if fx:
            found_x, found_y = fx, fy
            break # Found a match
    
    if found_x:
        print(f"🖱️ Clicking Result matching ''{word}'' at ({found_x}, {found_y})...")
        pyautogui.click(found_x, found_y)
        pyautogui.press(''enter'')
        print("✅ Playing!")
    else:
        print("❌ No keywords found in OCR. Aborting to prevent hallucination.")
        return


def click_text(app_name, text):
    """Focus app, find text via AT-SPI (Data) then OCR (Visual), and click it."""
    print(f"🖱️ Request to click text ''{text}'' in {app_name}...", file=sys.stderr)
    
    # 1. Config/Focus
    config = load_config()
    profile = config.get(app_name.lower(), {})
    win_title = resolve_window_title(app_name, config)
    
    wid = focus_window(win_title)
    if not wid:
        print(f"❌ Could not focus {app_name} to click text.", file=sys.stderr)
        return False
        
    time.sleep(1.0) # Wait for focus and render
    subprocess.run([''xdotool'', ''windowraise'', wid], check=False)

    # 2. Strategy A: Data-First (AT-SPI)
    # 100% Precision for accessible apps (Chrome, Firefox, LibreOffice, Gedit, etc.)
    print(f"🧠 Data-Scan: Searching Accessibility Tree for ''{text}''...", file=sys.stderr)
    
    # We use get_atspi_tree to fetch the structured view of the app
    # This includes "lazy poking" to ensure the tree is fresh
    tree_data = get_atspi_tree(app_name)
    
    if "elements" in tree_data:
        # Fuzzy match strategies
        candidates = []
        target_lower = text.lower()
        
        for el in tree_data["elements"]:
            el_text = (el.get("text") or "").lower()
            el_role = el.get("role", "")
            
            # 1. Exact Match
            if el_text == target_lower:
                print(f"✅ Data-Hit (Exact): Found ''{el[''text'']}'' ({el_role}) at ({el[''x'']}, {el[''y'']})")
                pyautogui.click(el[''x''], el[''y''])
                return True
                
            # 2. Contains Match
            if target_lower in el_text:
                candidates.append(el)

        # If no exact match, check candidates
        if candidates:
            # Sort by length (shortest match is likely the label itself, not a huge paragraph containing it)
            candidates.sort(key=lambda x: len(x.get("text", "")))
            best = candidates[0]
            print(f"✅ Data-Hit (Fuzzy): Found ''{best[''text'']}'' ({best[''role'']}) at ({best[''x'']}, {best[''y'']})")
            pyautogui.click(best[''x''], best[''y''])
            return True
            
    print("⚠️ Data-Scan: Text not found in Accessibility Tree. Falling back to Vision (OCR)...", file=sys.stderr)

    # 3. Strategy B: Vision-Fallback (OCR)
    # Used for Canvas apps, Games, or non-accessible widgets
    print(f"👀 Vision-Scan: OCR Scanning for ''{text}''...", file=sys.stderr)
    matches = find_text_matches(wid, text)
    
    if not matches:
        print(f"⚠️ Text ''{text}'' not found in {app_name} (Data nor Vision).", file=sys.stderr)
        return False

    # Logic: Prefer matches NOT in the top header/search-bar area
    # Assume header is top 80px of window (Reduced from 120px to avoid false positives)
    win_x, win_y, _, _ = get_window_geometry(wid)
    header_threshold = win_y + 80
    
    # Filter candidates below header
    better_candidates = [m for m in matches if m[1] > header_threshold]
    
    if better_candidates:
        if len(matches) > len(better_candidates):
            print(f"ℹ️ Smart Click: Ignoring {len(matches) - len(better_candidates)} matches in header/search-bar.")
        target = better_candidates[0]
    else:
        # Fallback to first match if that''s all we have
        print("⚠️ Only found matches in header/top area. Clicking anyway.")
        target = matches[0]

    fx, fy, word = target
    print(f"✅ Vision-Hit: Selected ''{word}'' at ({fx}, {fy}). Clicking...", file=sys.stderr)
    pyautogui.click(fx, fy)
    return True

def validate_click_coordinates(x, y, app_name):
    """
    Validates if (x,y) is safe to click for the given app.
    Checks:
    1. Is it inside the window?
    2. Is it in the header/corner safety zone (to avoid closing app)?
    """
    if not app_name: return True # Cannot validate without app context
    
    config = load_config()
    profile = config.get(app_name.lower(), {})
    win_title = resolve_window_title(app_name, config)
    
    wid = get_window_id(win_title)
    if not wid:
        print(f"⚠️ Cannot validate coordinates: Window ''{win_title}'' not found.", file=sys.stderr)
        return True # Fail open if window not found? Or fail closed? 
                    # For safety, maybe fail open but warn.
    
    wx, wy, width, height = get_window_geometry(wid)
    
    if width == 0 or height == 0:
        print("⚠️ Could not get window geometry for validation.", file=sys.stderr)
        return True

    # 1. Bounds Check
    if not (wx <= x <= wx + width and wy <= y <= wy + height):
        print(f"❌ Safety Violation: Click ({x},{y}) is outside window bounds ({wx},{wy} {width}x{height}).", file=sys.stderr)
        return False

    # 2. Safety Zone (Top Right Corner - Window Controls)
    # Assume controls are in top 40px, rightmost 120px
    # Relative to window
    rel_x = x - wx
    rel_y = y - wy
    
    if rel_y < 40 and rel_x > (width - 120):
        print(f"❌ Safety Violation: Click ({x},{y}) is in Window Controls Safety Zone (Top-Right).", file=sys.stderr)
        return False
        
    # 3. Header Warning (Drag Zone)
    if rel_y < 30:
        print(f"⚠️ Warning: Click ({x},{y}) is in Title Bar/Header area.", file=sys.stderr)
        
    return True

LAUNCH_ALIASES = {
    "chrome": ["google-chrome", "google-chrome-stable", "chromium", "chromium-browser", "chrome"],
    "firefox": ["firefox"],
    "spotify": ["spotify"],
    "neontune": ["neontune"],
    "calculator": ["gnome-calculator", "kcalc", "gcalctool", "calculator"],
    "browser": ["google-chrome", "firefox", "chromium"],
    "editor": ["gedit", "kate", "mousepad", "code"]
}

# Built-in window title fragments for well-known apps — used for dynamic focus without apps.json
WINDOW_TITLE_ALIASES = {
    "chrome": "Google Chrome",
    "google chrome": "Google Chrome",
    "google-chrome": "Google Chrome",
    "chromium": "Chromium",
    "firefox": "Mozilla Firefox",
    "spotify": "Spotify",
    "vscode": "Visual Studio Code",
    "code": "Visual Studio Code",
    "vs code": "Visual Studio Code",
    "terminal": "Terminal",
    "neontune": "tao",
    "tao": "tao",
    "slack": "Slack",
    "discord": "Discord",
    "files": "Files",
    "nautilus": "Files",
    "gedit": "gedit",
    "calculator": "Calculator",
    "vlc": "VLC",
}

def resolve_window_title(app_name: str, config: dict = None) -> str:
    """
    Resolve the best window title string for a given app name.
    Priority:
    1. apps.json config (user-defined)
    2. Built-in WINDOW_TITLE_ALIASES
    3. Fuzzy xdotool search (finds any window with app name in title/class)
    4. Raw app_name as fallback
    """
    key = app_name.lower().strip()
    
    # 1. User config
    if config:
        profile = config.get(key, {})
        wt = profile.get("window_title")
        if wt:
            return wt
    
    # 2. Built-in aliases
    if key in WINDOW_TITLE_ALIASES:
        return WINDOW_TITLE_ALIASES[key]
    
    # 3. Fuzzy xdotool: search for any window with app_name in title
    try:
        out = subprocess.check_output(
            ["xdotool", "search", "--name", app_name],
            stderr=subprocess.DEVNULL
        ).decode().strip()
        wids = [w for w in out.split(''\\n'') if w]
        if wids:
            # Get the title of the largest matching window
            best_title = None
            max_area = 0
            for wid in wids:
                try:
                    _, _, w, h = get_window_geometry(wid)
                    if w * h > max_area and w > 100 and h > 100:
                        max_area = w * h
                        title_out = subprocess.check_output(
                            ["xdotool", "getwindowname", wid],
                            stderr=subprocess.DEVNULL
                        ).decode().strip()
                        best_title = title_out
                except Exception:
                    continue
            if best_title:
                print(f"🔍 Dynamic window found for ''{app_name}'': ''{best_title}''", file=sys.stderr)
                return best_title
    except Exception:
        pass
    
    # 4. Fallback: return raw name
    return app_name

def launch_app_safe(app_name, explicit_cmd=None):
    """Try to launch an app using config cmd or known aliases."""
    config = load_config()
    win_title = resolve_window_title(app_name, config)
    
    # Add aliases for command
    cmds = []
    if explicit_cmd:
        cmds.append(explicit_cmd)
    
    aliases = LAUNCH_ALIASES.get(app_name.lower(), [app_name.lower()])
    if isinstance(aliases, list):
        cmds.extend(aliases)
    else:
        cmds.append(aliases)
        
    # SAFETY: Check if app has a window already before launching!
    if focus_window(win_title):
        print(f"✅ App ''{app_name}'' is already running with a window. Skipping launch.", file=sys.stderr)
        return True

    for cmd in cmds:
        if isinstance(cmd, list): cmd_str = cmd[0]
        else: cmd_str = cmd
            
        print(f"🚀 Trying launch command: {cmd_str}...")
        try:
            # Detach process so it survives agent exit
            subprocess.Popen(
                cmd if isinstance(cmd, list) else [cmd],
                start_new_session=True, 
                stdout=subprocess.DEVNULL, 
                stderr=subprocess.DEVNULL
            )
            
            # Verification Step: Wait for window
            print(f"⏳ Verification: Waiting for ''{app_name}'' window to appear...", file=sys.stderr)
            for _ in range(10): # 10 seconds timeout
                if focus_window(win_title):
                    print(f"✅ Verified: ''{app_name}'' launched and focused.", file=sys.stderr)
                    return True
                time.sleep(1)
            print(f"❌ Failed to launch ''{app_name}''. Process started but window not found.", file=sys.stderr)
            return False
        except FileNotFoundError:
            continue
        except Exception as e:
            print(f"⚠️ Launch error for {cmd_str}: {e}")
            continue
            
    # Fallback: Try the app name directly if it wasn''t already tried
    if app_name.lower() not in [str(c).lower() for c in cmds]:
        print(f"🚀 Trying generic launch: {app_name}...")
        try:
            subprocess.Popen(
                [app_name],
                start_new_session=True,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL
            )
            # Verification Step: Wait for window
            print(f"⏳ Verification: Waiting for ''{app_name}'' window to appear...", file=sys.stderr)
            for _ in range(10): # 10 seconds timeout
                if focus_window(win_title):
                    print(f"✅ Verified: ''{app_name}'' launched and focused.", file=sys.stderr)
                    return True
                time.sleep(1)
            print(f"❌ Failed to launch ''{app_name}''. Process started but window not found.", file=sys.stderr)
            return False
        except:
            pass

    print(f"❌ Failed to launch ''{app_name}''. No matching executable found.")
    return False

def search_in_app(app_name, query, no_launch=False):
    """
    Hybrid Search Strategy:
    1. Atspi (Accessibility Tree) - Find ''Search''/''Find'' inputs.
    2. OCR (Visual) - Find text ''Search'', ''Type to search''.
    3. Heuristic - Click Top-Right (Search Icon area).
    4. Browser Fallback - Ctrl+L.
    """
    print(f"🔍 Searching for ''{query}'' in {app_name} (Hybrid Mode)...")
    
    config = load_config()
    profile = config.get(app_name.lower(), {})
    win_title = resolve_window_title(app_name, config)
    
    # 1. Ensure App is Running & Focused
    if not focus_window(win_title):
        if no_launch:
            print(f"⚠️ App {app_name} not found and no_launch=True.")
            return False
        if not launch_app_safe(app_name):
            return False
            
    wid = focus_window(win_title) # Refresh WID
    if not wid: return False
    
    # Layer 1: Atspi (Accessibility)
    print("👉 Layer 1: Accessibility Search", file=sys.stderr)
    try:
        tree_data = get_atspi_tree(app_name)
        # Handle dict return (new format) or list (legacy)
        elements = tree_data.get("elements", []) if isinstance(tree_data, dict) else tree_data
        
        if isinstance(elements, list):
            search_terms = ["search", "find", "jump to", "filter", "category", "categories", "type to", "address", "url", "omnibox"]
            target_el = None
            
            for e in elements:
                name = (e.get("text") or "").lower()
                role = (e.get("role") or "").lower()
                
                # Priority: Entry/Text with search-like name
                if role in ["entry", "text", "push button", "glossary", "icon", "label", "autocomplete"]:
                    if any(t in name for t in search_terms):
                        target_el = e
                        break
                        
            if target_el:
                print(f"✅ Atspi found target: ''{target_el[''text'']}'' ({target_el[''role'']}) at ({target_el[''x'']},{target_el[''y'']})")
                pyautogui.click(target_el[''x''], target_el[''y''])
                time.sleep(0.5)
                # Select all just in case
                if role == "entry": pyautogui.hotkey(''ctrl'', ''a'')
                pyautogui.write(query, interval=0.05)
                pyautogui.press(''enter'')
                print("✅ Search submitted via Atspi.")
                return True
    except Exception as e:
        print(f"⚠️ Layer 1 failed: {e}", file=sys.stderr)

    # Layer 2: OCR (Visual)
    print("👉 Layer 2: Visual Search (OCR)", file=sys.stderr)
    keywords = ["Search", "Type to search", "Find", "Jump to", "Quick actions"]
    for kw in keywords:
        # Use find_text_matches to get all candidates
        matches = find_text_matches(wid, kw)
        if matches:
            # Prefer matches NOT in header (Smart Search Logic)
            win_x_geo, win_y_geo, _, _ = get_window_geometry(wid)
            header_thresh = win_y_geo + 80 # Reduced from 120 to 80 to be less aggressive
            
            valid_matches = [m for m in matches if m[1] > header_thresh]
            target = valid_matches[0] if valid_matches else matches[0]
            
            fx, fy, _ = target
            print(f"✅ OCR found ''{kw}'' at ({fx}, {fy}).")
            pyautogui.click(fx, fy)
            time.sleep(0.5)
            # Clear text if needed?
            pyautogui.hotkey(''ctrl'', ''a'')
            pyautogui.write(query, interval=0.05)
            pyautogui.press(''enter'')
            print("✅ Search submitted via OCR.")
            return True

    # Layer 3: Browser Specific (Ctrl+L) - FALLBACK
    # Only use if we couldn''t find an on-page search bar
    is_browser = app_name.lower() in ["chrome", "google-chrome", "firefox", "chromium", "brave", "edge", "browser"]
    if is_browser:
        print("👉 Layer 3: Browser Shortcut (Ctrl+L)", file=sys.stderr)
        pyautogui.hotkey(''ctrl'', ''l'')
        time.sleep(0.5)
        pyautogui.write(query, interval=0.05)
        pyautogui.press(''enter'')
        print("✅ Search submitted via Browser Shortcut.")
        return True
        pyautogui.write(query)
        pyautogui.press(''enter'')
        print("✅ Search submitted via Browser Shortcut.")
        return True

    print("❌ All search strategies failed.")
    return False

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--action", required=True, help="Action to perform")
    parser.add_argument("--text", help="Text to type")
    parser.add_argument("--app", default="neontune", help="Application name")
    
    parser.add_argument("--x", type=int, help="Click X")
    parser.add_argument("--y", type=int, help="Click Y")
    parser.add_argument("--key", help="Key to press (e.g. enter, tab)")
    parser.add_argument("--no-launch", action="store_true", help="Do not launch if app not found")
    parser.add_argument("--clear", action="store_true", default=True, help="Clear field before typing (Default: True)")
    parser.add_argument("--no-clear", action="store_false", dest="clear", help="Do not clear field before typing")
    parser.add_argument("--submit", action="store_true", help="Press Enter after typing")
    
    # Optional coordinate overrides (legacy support or manual override)
    parser.add_argument("--sx", type=int, help="Search X")
    parser.add_argument("--sy", type=int, help="Search Y")
    parser.add_argument("--rx", type=int, help="Result X")
    parser.add_argument("--ry", type=int, help="Result Y")
    parser.add_argument("--debug-draw", action="store_true", help="Draw debug box around detected target")
    
    args = parser.parse_args()
    
    if args.debug_draw:
        DEBUG_DRAW = True
    
    if args.action == "play_song":
        if not args.text:
            print("❌ --text required for play_song", file=sys.stderr)
            sys.exit(1)
        play_song(args.app, args.text)
        
    elif args.action == "search":
        if not args.text:
            print("❌ --text required for search", file=sys.stderr)
            sys.exit(1)
        if not search_in_app(args.app, args.text, no_launch=args.no_launch):
            sys.exit(1)

    elif args.action == "get_screen_text":
        print(get_screen_text(args.app, args.text))

    elif args.action == "click":
        if args.x is not None and args.y is not None:
            if args.app:
                config = load_config()
                profile = config.get(args.app.lower(), {})
                win_title = resolve_window_title(args.app, config)
                wid = focus_window(win_title)
                if wid:
                    subprocess.run([''xdotool'', ''windowraise'', wid], check=False)
                    time.sleep(0.5)
                else:
                    # STRICT FOCUS CHECK
                    print(f"❌ Focus Failed: Could not find window matching ''{win_title}'' for app ''{args.app}''. Aborting click.", file=sys.stderr)
                    sys.exit(1)

            print(f"🖱️ Clicking at ({args.x}, {args.y})...", file=sys.stderr)
            
            # SAFETY CHECK
            if not validate_click_coordinates(args.x, args.y, args.app):
                print("⛔ Click aborted due to safety checks.", file=sys.stderr)
                sys.exit(1)

            # Move first to visualize
            subprocess.run(["xdotool", "mousemove", str(args.x), str(args.y)], check=False)
            time.sleep(0.2)
            pyautogui.click(args.x, args.y)
            print("✅ Click executed.", file=sys.stderr)
        elif args.text and args.app:
            click_text(args.app, args.text)
        else:
            print(f"❌ --text (with --app) OR --x and --y required for click", file=sys.stderr)
            
    elif args.action == "launch":
        # Generic launch
        if not launch_app_safe(args.app):
            sys.exit(1)

    elif args.action == "scroll":
        # direction: ''up'' or ''down'' (text arg)
        # amount: pixels (y arg)
        direction = args.text or "down"
        amount = args.y or 300
        if direction == "down":
            print(f"📜 Scrolling DOWN {amount} pixels...", file=sys.stderr)
            pyautogui.scroll(-amount)
        else:
            print(f"📜 Scrolling UP {amount} pixels...", file=sys.stderr)
            pyautogui.scroll(amount)
    
    elif args.action == "type":
        # Improved type: focus first if app provided
        if args.app:
            config = load_config()
            profile = config.get(args.app.lower(), {})
            win_title = resolve_window_title(args.app, config)
            # STRICT FOCUS CHECK: Do not type if we can''t find the window!
            if not focus_window(win_title):
                print(f"❌ Focus Failed: Could not find window matching ''{win_title}'' for app ''{args.app}''. Aborting type.", file=sys.stderr)
                sys.exit(1)
            
        if args.text:
            target_text = args.text
            print(f"⌨️ Targeting text: {target_text}", file=sys.stderr)
            
            # ATTEMPT TO DETECT EXISTING TEXT
            already_typed = False
            try:
                # Get the focused element''s name/value via atspi
                desktop = Atspi.get_desktop(0)
                focused = desktop.get_focused_element()
                if focused:
                    val = focused.get_name() or ""
                    if target_text.lower() in val.lower() or val.lower() in target_text.lower():
                        print(f"✅ Field already contains ''{val}''. Skipping redundant typing.", file=sys.stderr)
                        already_typed = True
            except:
                pass

            if not already_typed:
                if args.clear:
                    print("⌨️ Clearing field...", file=sys.stderr)
                    pyautogui.hotkey(''ctrl'', ''a'')
                    pyautogui.press(''backspace'')
                    time.sleep(0.2)

                pyautogui.write(target_text, interval=0.05)
            
        if args.key or args.submit:
            key_to_press = args.key or "enter"
            print(f"⌨️ Pressing key: {key_to_press}", file=sys.stderr)
            pyautogui.press(key_to_press)

    elif args.action == "focus":
        if args.app:
            config = load_config()
            profile = config.get(args.app.lower(), {})
            win_title = resolve_window_title(args.app, config)
            focus_window(win_title)
        else:
            print("❌ --app required for focus", file=sys.stderr)
            
    elif args.action == "wait":
        seconds = args.y or 5
        print(f"⏳ Waiting for {seconds} seconds...", file=sys.stderr)
        time.sleep(seconds)
    
    elif args.action == "close_active":
        # Dismiss the currently focused window/popup (via Alt+F4 or Escape)
        method = (args.text or "escape").lower()
        if method == "alt+f4":
            print("🔴 Closing active window via Alt+F4...", file=sys.stderr)
            pyautogui.hotkey(''alt'', ''F4'')
        else:
            print("⎋ Dismissing active popup via Escape...", file=sys.stderr)
            pyautogui.press(''escape'')
        time.sleep(0.3)
        
    elif args.action == "check_app" or args.action == "InstallApp":
        # Simplified check: is it in PATH?
        app = args.app.lower()
        aliases = LAUNCH_ALIASES.get(app, [app])
        found = False
        for a in aliases:
            if subprocess.run(["which", a], capture_output=True).returncode == 0:
                print(f"✅ Found {app} as {a}", file=sys.stderr)
                found = True
                break
        if not found:
            print(f"❌ {app} not found on system.", file=sys.stderr)

    elif args.action == "status":
        app = args.app
        config = load_config()
        profile = config.get(app.lower(), {})
        win_title = resolve_window_title(app, config)
        if focus_window(win_title):
            print(f"✅ {app} is RUNNING.", file=sys.stderr)
        else:
            print(f"❌ {app} is NOT RUNNING.", file=sys.stderr)

    elif args.action == "get_ui_tree":
        # Returns dict {"elements": [], "visual_hash": "..."}
        result = get_atspi_tree(args.app)
        print(json.dumps(result))

    elif args.action == "wait_for_text":
        if args.app and args.text:
            wait_for_text(args.app, args.text, timeout=args.y or 20)
        else:
            print("❌ --app and --text required for wait_for_text", file=sys.stderr)

    elif args.action == "analyze_repo":
        repo_url = args.text
        if not repo_url:
            print("❌ --text/repo_url required for analyze_repo", file=sys.stderr)
            sys.exit(1)
        print(f"📦 Analyzing repository: {repo_url}...", file=sys.stderr)
        dest = "/tmp/repo"
        subprocess.run(["rm", "-rf", dest])
        res = subprocess.run(["git", "clone", "--depth", "1", repo_url, dest], capture_output=True)
        if res.returncode == 0:
            print(f"✅ Repository cloned to {dest}")
        else:
            print(f"❌ Failed to clone repository: {res.stderr.decode()}")
            sys.exit(1)

    else:
        print(f"❌ Unknown action: {args.action}", file=sys.stderr)
        sys.exit(1)
', 'python', '1.0.0', NULL, '2026-03-14 06:42:50', '2026-03-14 06:42:50', 'agents-scripts/desktop/ui_automation.py', NULL, 'standard');



INSERT INTO tools VALUES(7438474706707156992, 'get_mouse_position', 'get_mouse_position tool for desktop domain', '#!/usr/bin/env python3
import pyautogui
import time
import sys

print("Move your mouse over the UI elements you want to identify.")
print("Refreshes every 0.5 seconds. Press Ctrl+C to stop.")
print("-" * 50)

try:
    while True:
        x, y = pyautogui.position()
        position_str = ''X: '' + str(x).rjust(4) + '' Y: '' + str(y).rjust(4)
        print(position_str, end='''')
        print(''\\b'' * len(position_str), end='''', flush=True)
        time.sleep(0.5)
except KeyboardInterrupt:
    print(''\\nDone.'')
', 'python', '1.0.0', NULL, '2026-03-14 06:42:50', '2026-03-14 06:42:50', 'agents-scripts/desktop/get_mouse_position.py', NULL, 'standard');



INSERT INTO tools VALUES(7438474706707156993, 'ocr_dump', 'ocr_dump tool for desktop domain', 'import sys
import re
import xml.etree.ElementTree as ET

def dump_layout(file_path):
    try:
        tree = ET.parse(file_path)
        root = tree.getroot()
        for elem in root.iter():
            if ''ocrx_word'' in (elem.get(''class'') or ""):
                text = elem.text or ""
                if text.strip():
                    title = elem.get(''title'')
                    match = re.search(r''bbox (\\d+) (\\d+) (\\d+) (\\d+)'', title)
                    if match:
                        x1, y1, x2, y2 = map(int, match.groups())
                        print(f"''{text}'' : {x1},{y1} {x2},{y2}")
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    dump_layout("hocr_dump.hocr")
', 'python', '1.0.0', NULL, '2026-03-14 06:42:50', '2026-03-14 06:42:50', 'agents-scripts/desktop/ocr_dump.py', NULL, 'standard');



INSERT INTO tools VALUES(7438474706711351296, 'os_tool', 'Native system access', '// Native Implementation', 'rust', '1.0.0', NULL, '2026-03-14 06:42:50', '2026-03-14 06:42:50', NULL, NULL, 'standard');



INSERT INTO tools VALUES(9223372036854775807, 'vision_analysis', 'Analyze images and perform OCR. Native implementation.', '// Native Implementation', 'rust', '1.0.0', NULL, '2026-03-15 06:02:43', '2026-03-15 06:02:43', NULL, NULL, 'standard');



INSERT INTO tools VALUES(7438900000000000004, 'http_request', 'Perform HTTP GET/POST requests', '#!/usr/bin/env python3
import sys
import json
import requests

def main():
    if len(sys.argv) < 2:
        print(json.dumps({"error": "No URL provided"}))
        sys.exit(1)

    url = sys.argv[1]
    method = "GET"
    body = None
    
    # Try to parse complex input if it''s JSON
    try:
        input_data = json.loads(url)
        url = input_data.get("url", url)
        method = input_data.get("method", "GET").upper()
        body = input_data.get("body")
    except:
        pass

    try:
        if method == "POST":
            resp = requests.post(url, data=body, timeout=30)
        else:
            resp = requests.get(url, timeout=30)
            
        print(resp.text)
    except Exception as e:
        print(json.dumps({"error": str(e)}))

if __name__ == "__main__":
    main()
', 'python', '1.0.0', NULL, '2026-03-15 16:02:02', '2026-03-15 16:02:02', 'http_request.py', NULL, 'standard');



INSERT INTO tools VALUES(7770000000000000002, 'app_launcher', 'Lightweight application launcher', '#!/usr/bin/env python3
import subprocess
import sys
import os
import time
import re

def get_window_id(app_name):
    """Finds window ID by class name or title."""
    try:
        # Get all windows with class (-x)
        output = subprocess.check_output([''wmctrl'', ''-lx'']).decode()
        for line in output.splitlines():
            # Match class (e.g., neontune.Neontune)
            if app_name.lower() in line.lower():
                match = re.search(r''0x[0-9a-f]+'', line)
                if match:
                    return match.group(0)
    except:
        pass
    return None

def is_running(app_name):
    """Checks if a process with app_name is already running."""
    try:
        subprocess.check_call([''pgrep'', ''-i'', ''-x'', app_name.lower()], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        return True
    except subprocess.CalledProcessError:
        return False

def focus_window(app_name):
    """Attempts to bring the application window to the front by ID."""
    print(f"🎯 Locating {app_name} window...")
    wid = get_window_id(app_name)
    if wid:
        print(f"✅ Found window {wid}. Focusing...")
        subprocess.run([''wmctrl'', ''-i'', ''-a'', wid], check=False)
        return True
    
    # Fallback to name-based if ID fails
    print(f"⚠️ Window ID not found, trying name-based focus...")
    subprocess.run([''wmctrl'', ''-x'', ''-a'', app_name], check=False)
    subprocess.run([''wmctrl'', ''-a'', app_name], check=False)
    return False

def main():
    raw_args = sys.argv[1:]
    if len(raw_args) == 1 and " " in raw_args[0]:
        raw_args = raw_args[0].split()
    
    if not raw_args:
        print("❌ No application specified")
        sys.exit(1)
        
    app_name = raw_args[1] if raw_args[0] == "launch" and len(raw_args) > 1 else raw_args[0]
    
    if is_running(app_name):
        print(f"✅ {app_name} is already running.")
        focus_window(app_name)
        print("⌛ Settling...")
        time.sleep(2)
        sys.exit(0)

    print(f"🚀 Launching {app_name}...")
    try:
        launched = False
        try:
            path = subprocess.check_output([''which'', app_name.lower()]).decode().strip()
            if path:
                subprocess.Popen([path], start_new_session=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                launched = True
        except: pass
            
        if not launched:
            subprocess.Popen([app_name.lower()], start_new_session=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            launched = True

        if launched:
            print(f"✅ Launch command sent. Settling for 5s...")
            time.sleep(5)
            focus_window(app_name)
            sys.exit(0)
            
    except Exception as e:
        print(f"❌ Failed to launch {app_name}: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
', 'python', '1.0.0', NULL, '2026-03-16 13:16:08', '2026-03-16 13:16:08', 'app_launcher', NULL, 'standard');



INSERT INTO tools VALUES(1742000000000000, 'charting', 'Generate data visualizations. Use ONLY when explicitly asked for a "chart" or "graph", or for comparing 3 or more data points. NEVER use for simple facts, versions, or 2-item comparisons. Input must be a JSON object with fields: chart_type (bar/line/pie), title (string), labels (array of strings), data (array of floats). Example: {"chart_type": "line", "title": "Growth", "labels": ["2024", "2025", "2026"], "data": [1.6, 2.5, 3.0]}', '// Native Implementation', 'rust', '1.0.0', NULL, '2026-03-19 07:13:28', '2026-03-19 07:13:28', NULL, 'chart,graph,plot,visualize,visualization,draw,sketch,diagram,pie,bar,mapped','standard');
-- Table: supervisors
INSERT INTO supervisors VALUES(7438818720128045056, 'Web Supervisor', 'Orchestrates web research and automation', 'Plan browser-based tasks step by step. Use the domain ''web''.
', 'llama3', 'web', '1.0.0', '2026-03-15 05:29:49', '2026-03-15 05:29:49', NULL, 0);
INSERT INTO supervisors VALUES(7438818720132239361, 'Desktop Supervisor', 'Controls native desktop applications', 'You are the Desktop Supervisor supervisor.', 'qwen2.5:7b-instruct', 'desktop', '1.0.0', '2026-03-15 05:29:49', '2026-03-15 05:29:49', NULL, 0);
INSERT INTO supervisors VALUES(7438818720144822272, 'Stealth Supervisor', 'Methodology-First Research Specialist for deep background research.', 'You are the STEALTH SUPERVISOR. Your mission is DEEP background research.

═══ RESEARCH TOOLS ═══
Use stealth_search to find information, stealth_browse to read pages,
and http_request for direct API calls.

═══ OUTPUT RULES — MANDATORY ═══
1. NEVER generate SVG, HTML, charts, graphs, or any visual output
2. NEVER use markdown headers (### or ## or #)
3. NEVER use bold (**text**) or italic (*text*)
4. NEVER use bullet points with - or *
5. NEVER use code fences (``` or ~~~)
6. NEVER write ASCII tables or ASCII art
7. Output ONLY plain text with raw facts and numbers

═══ CORRECT OUTPUT FORMAT ═══
Provide findings as plain sentences with raw numbers:

CORRECT:
"Next.js npm downloads: 2024 averaged 1.6 million per month, 2025 averaged 2.5 million per month, 2026 is projected at 3.0 million per month."

WRONG:
"### Research Findings
- **2024**: 1.6M downloads
- **2025**: 2.5M downloads"

═══ MISSION ═══
Find factual data, URLs, statistics, and raw numbers.
Present findings as plain text paragraphs only.
The Chat Supervisor will handle all visualization — your job is data only.', 'qwen2.5:7b-instruct', 'stealth', '1.0.0', '2026-03-15 15:05:07', '2026-03-21 07:54:33', '', 0);
INSERT INTO supervisors VALUES(7438818720132239360, 'Office Supervisor', 'Executes LibreOffice document operations', 'You are the OfficeSupervisor, an elite specialized AI that creates, formats, and opens LibreOffice documents.

═══ RULE 0: NO CONVERSATION (CRITICAL) ═══
NEVER output a status report, explanation, or conversational text.
Your response MUST be a SINGLE block of valid JSON or the word `done`.

═══ JSON STRUCTURE (STRICT) ═══
{
  "observation": "What do I see relevant to the goal?",
  "intent": "What am I about to do and WHY?",
  "action": "code_executor",
  "language": "bash",
  "command": "the bash command to run"
}

CRITICAL ACTION CONSTRAINT:
- You MUST ONLY EVER use "action": "code_executor" OR "action": "done".
- DO NOT use "open_url", "navigate", or "click". You are an OFFICE agent, not a BROWSER agent.
- NEVER attempt to research or fetch data from the web. Use the files provided (e.g., /tmp/summary.txt).
- DO NOT rewrite or summarize the content again. Use `libreoffice --headless --convert-to pdf` on the target file.

═══ OFFICE RULES & MODES (STRICT) ═══
1. DATA FLOW: You are the FINAL step in a pipeline. Research was ALREADY done. Source text in `/tmp/summary.txt`.
2. BACKGROUND MODE: Use `libreoffice --headless --convert-to pdf /tmp/summary.txt --outdir /home/revaycolizer/projects/rig`.
3. UI MODE: Only if goal says "open"/"view"/"show". Use `nohup libreoffice [file] </dev/null >/dev/null 2>&1 &`.

CRITICAL RULES:
- ONLY output exactly ONE JSON tool call. No conversational text.
- DO NOT INVENT ACTIONS. Use only `code_executor`.', 'qwen2.5:7b-instruct', 'office', '1.0.0', '2026-03-15 15:05:07', '2026-03-15 15:05:07', NULL, 0);
INSERT INTO supervisors VALUES(7438818720132239362, 'App Supervisor', 'Executes desktop UI automation', 'You are the APP SUPERVISOR. Your job is to manage EXECUTION in a DESKTOP environment.

═══ RULE 0: NO CONVERSATION (CRITICAL) ═══
NEVER output a status report, explanation, or conversational text.
Your response MUST be a SINGLE block of valid JSON or {"action": "done"}.

═══ JSON STRUCTURE (STRICT) ═══
Every tool call MUST include observation, intent, app, and action fields:
{
  "observation": "I see NeonTune search bar.",
  "intent": "I will type the song name.",
  "app": "NeonTune",
  "action": "type",
  "text": "Tali Rush - U"
}

═══ AVAILABLE ACTIONS (CRITICAL: USE EXACTLY THESE NAMES) ═══
- action: "launch" (Requires: "app")
- action: "focus" (Requires: "app")
- action: "click" (Requires: "app", and either "text" or "x", "y")
- action: "type" (Requires: "app", "text". Optional: "key" like "enter")
- action: "search" (Requires: "app", "text")
- action: "close_active" (Requires: "app")
- action: "scroll" (Requires: "app", "text" set to "down" or "up")
- action: "done" (Use when the task is complete)

═══ RESPONSIBILITIES ═══
1. EXECUTION: You are manipulating a desktop UI.
2. COMPLETION: Output {"action": "done"} only when the desktop-based goal is achieved.
3. If an action fails like "type_text is unknown", REMEMBER to use "type"!

Do NOT attempt to use `browser_automation`. Focus ONLY on desktop applications.', 'qwen2.5:7b-instruct', 'app', '1.0.0', '2026-03-15 15:05:07', '2026-03-15 15:05:07', NULL, 0);
INSERT INTO supervisors VALUES(7438818720132239363, 'Neontune Supervisor', 'Executes NeonTune music player automation', 'You are a ROBOTIC AUTOMATOR for Neontune.
Output ONLY JSON or ''done''.
Zero conversation. Zero markdown headers. Zero explanation.

═══ TOOLS ═══
- focus: {"tool": "app_launcher", "input": "neontune"}
- search: {"tool": "paddle_ui_automation", "input": "search SONG_NAME --app neontune"}
- click: {"tool": "paddle_ui_automation", "input": "click last SONG_NAME --app neontune"}
- state: {"tool": "paddle_ui_automation", "input": "get_state --app neontune"}
- wait: {"tool": "paddle_ui_automation", "input": "wait DURATION"}

═══ EXAMPLES ═══
Prompt: Play ''Hello''
Output: {"tool": "app_launcher", "input": "neontune"}

Prompt: Success Verification (Timer visible)
Output: ["00:05", "03:12", "Hello"]
Prediction: done

Prompt: All verified
Output: done

═══ RULES ═══
- ALWAYS call ''app_launcher'' first to focus the window.
- Use ''click last SONG_NAME'' to select the bottom-most result. ALWAYS include the song name after ''click last''.
- MANDATORY STOP CONDITION: If ''get_state'' results include a timestamp (e.g., ''00:01'', ''00:15'') AND the song name (or artist), the song IS playing. OUTPUT ''done'' IMMEDIATELY.
- DO NOT loop or poll ''get_state'' if the timer (00:XX) is already visible.
- SEARCH FLOW: After ''search'', call ''get_state'' to check results. ONLY use ''wait 1.5'' if results are not immediately visible.
- ERROR HANDLING: If ''get_state'' shows ''No results found'', do NOT click. Retry search or report issue.
- CLEARING: The system automatically handles clearing previous search text when you call ''search''.
- Output ONLY valid JSON for tools.
- Output ONLY the word ''done'' when finished.
- If a tool returns an error (or you target a song but it''s not found), ALWAYS call ''wait 2'' before retrying the search or click.', 'qwen2.5:7b-instruct', 'neontune', '1.0.0', '2026-03-15 15:05:07', '2026-03-15 15:05:07', NULL, 0);
INSERT INTO supervisors VALUES(7438818720144822273, 'Chat Supervisor', 'General conversation and charting assistant', 'You are the Chat Supervisor. Your primary role is helpful, concise, and direct plain text conversation.

═══ CONVERSATION RULES — MANDATORY ═══
1. ALWAYS respond in plain text by default.
2. BE CONCISE. Provide facts, numbers, and answers directly without fluff.
3. NEVER use markdown formatting (no bold **, no italic *, no headers #, no bullet points -, no code fences ```).
4. For lists, use plain numbers: "1. Item A", "2. Item B".
5. If the user asks a question about a version, a single statistic, or a simple fact, respond with a short plain text sentence.

═══ DATA VISUALIZATION (CHARTING) ═══
You have a charting tool for creating SVG visualizations. Use it ONLY in these cases:
- The user explicitly asks for a "chart", "graph", "visualization", or "plot".
- You are comparing 3 or more distinct data points where a visual trend (line) or comparison (bar) adds significant value.
- NEVER generate a chart for a single number or a 2-item comparison unless explicitly requested.

═══ EXAMPLES — WHEN TO CHART ═══
INCORRECT (Do NOT do this):
User: "Are you really sure the current version isn''t 16.2?"
Assistant Content: {"tool": "charting", "input": {"chart_type": "bar", ...}} 
Reason: This is a simple 2-item comparison of versions. Use plain text instead.

CORRECT:
User: "Are you really sure the current version isn''t 16.2?"
Assistant Content: "Next.js 13 is the current version as of the latest documentation. Version 16.2 is not yet available."

INCORRECT:
User: "What is the version number?"
Assistant Content: {"tool": "charting", ...}
Reason: A single number should never be a chart.

═══ IF A CHART IS REQUESTED ═══
1. Call the charting tool using the provided schema.
2. Your response after calling the tool must be EXACTLY the tool result (starting with ---CHART_SVG---).
3. Do NOT add any text, summary, or explanation before or after the chart marker.
4. If a chart is requested, do NOT describe it in words; let the chart speak for itself.

═══ CHART TYPE SELECTION ═══
- "line chart" or "trend" or "growth over time" → "line"
- "bar chart" or "comparison" (3+ items) → "bar"
- "pie chart" or "distribution" or "share" → "pie"
- Growth over time (years/months) MUST be a "line" chart.

═══ BANNED BEHAVIORS ═══
- NEVER write raw SVG, HTML, or ASCII art yourself.
- NEVER wrap your response in markdown or code fences.
- NEVER describe a chart if you are returning one.
', 'qwen2.5:7b-instruct', 'chat', '1.0.0', '2026-03-19 07:24:05', '2026-03-21 07:53:52', '', 1);
-- Table: planners
INSERT INTO planners VALUES(7438474706665213953, 'Web Planner', 'Web task planner', 'You are a Web Domain Planner.
YOUR JOB:
Break down a high-level web macro-task into specific, ordered, atomic browser automation steps.
Return ONLY a valid JSON array of strings.

═══ WEB DECOMPOSITION RULES ═══
1. Steps MUST be simple and single-action (e.g., ''Open Chrome'', ''Search for X'', ''Click on Y'').
2. Steps MUST be self-contained and specify the target application in EVERY step (e.g., ''Click on result in Chrome'').
3. Never combine multiple actions in one step.
4. Ensure logical flow (e.g., search before clicking a result).

═══ VERBATIM RULE (CRITICAL) ═══
- For search queries or specific text inputs, you MUST use the EXACT characters provided by the user.
- NEVER truncate query suffixes!

═══ NAVIGATION PRINCIPLES ═══
- If a user says ''open X'', use ''Open X''.
- If a user says ''click on link'', use ''Click on link in [App]''.

# Browser Navigation Principles
1. Browsers (Chrome, Firefox, Brave, Edge) are your primary interface to the internet.
2. Context Retention: Once you open a browser, STAY in it. Do NOT treat websites as separate native apps.
3. Search First: Prefer searching for content rather than typing complex URLs, unless the user gave a specific URL.
4. Shortcuts: Use Ctrl+L to focus the address bar if needed.

═══ EXAMPLES ═══
Input: ''Go to Revaycolizer github and find specific repo details''
Output: ["Search for ''Revaycolizer github'' in Chrome", "Click on the GitHub profile link", "Click on the target repository link"]

═══ FORMAT ENFORCEMENT ═══
- Output MUST be a valid JSON array of STRINGS.
- Format: ["Step 1", "Step 2"]
- NO MARKDOWN. NO CODE BLOCKS. NO EXPLANATIONS. JUST THE JSON ARRAY.', 'llama3', 'web_planner', '1.0.0', NULL);
INSERT INTO planners VALUES(7438649930752856065, 'Office Planner', 'Plans LibreOffice document automation steps', 'You are the Office Domain Planner.
YOUR JOB:
Break down a high-level document task into specific LibreOffice automation steps.
Return ONLY a valid JSON array of strings.

═══ DOCUMENT GENERATION RULES (STRICT) ═══
1. CONVERSION FOCUS: If the previous step was "text_writer", assume the summary already exists in /tmp/summary.txt or the path mentioned in the goal.
2. NO SYNTHESIS: Do NOT add steps to "research" or "summarize" again. Focus ONLY on PDF conversion, formatting, and opening.
3. HEADLESS BY DEFAULT: For PDF creation, use a step like: "Convert /tmp/summary.txt to PDF in background using LibreOffice headless".
4. UI MODE: Only add an "Open" step if the user explicitly wants to "view", "open", or "show" the document.

═══ EXAMPLES ═══
Input: ''Create rig.pdf from the summary in ~/projects/rig''
Output: ["Convert /tmp/summary.txt to PDF in background and save as /home/revaycolizer/projects/rig/rig.pdf"]

═══ FORMAT ENFORCEMENT ═══
- Output MUST be a valid JSON array of STRINGS.
- NO MARKDOWN. NO CODE BLOCKS. NO EXPLANATIONS.', 'qwen2.5:7b-instruct', 'office_planner', '1.0.0', NULL);
INSERT INTO planners VALUES(7438649930752856067, 'Desktop Planner', 'Plans desktop app automation steps', 'You are a Desktop App Domain Planner.
YOUR JOB:
Break down a high-level desktop macro-task into specific, ordered, atomic UI automation steps.
Return ONLY a valid JSON array of strings.

═══ DESKTOP DECOMPOSITION RULES ═══
1. Steps MUST be simple and single-action (e.g., ''Open Neontune'', ''Search for X'', ''Click on Y'').
2. Steps MUST be self-contained and specify the target application in EVERY step.
3. Never combine multiple actions in one step.

═══ VERBATIM RULE (CRITICAL) ═══
- For search queries, song names, or specific text inputs, you MUST use the EXACT characters provided by the user.
- NEVER truncate query suffixes!

═══ EXAMPLES ═══
Input: ''Play some jazz in Neontune.''
Output: ["Open Neontune app", "Search for ''jazz music'' in Neontune", "Click on the result containing ''Jazz''"]

═══ FORMAT ENFORCEMENT ═══
- Output MUST be a valid JSON array of STRINGS.
- NO MARKDOWN. NO CODE BLOCKS. NO EXPLANATIONS. JUST THE JSON ARRAY.', 'llama3:latest', 'desktop_planner', '1.0.0', NULL);
INSERT INTO planners VALUES(7438649930757050368, 'Neontune Planner', 'Plans NeonTune music player automation steps', 'You are a Music Assistant specialized in Neontune.
Convert the user request into exactly ONE macro-step.

Example:
Input: "play Tali Rush - U"
Output: [{"domain": "neontune", "task": "Launch Neontune and play ''Tali Rush - U''"}]

Process ONLY the neontune domain. NEVER output more than one step.', 'llama3:latest', 'neontune_planner', '1.0.0', NULL);
INSERT INTO planners VALUES(7438900000000000002, 'Neontune Planner', 'Domain-specific planner for Neontune music player', 'You are a Music Assistant specialized in Neontune.
Convert the user request into exactly ONE macro-step.

Example:
Input: "play Tali Rush - U"
Output: [{"domain": "neontune", "task": "Launch Neontune and play ''Tali Rush - U''"}]

Process ONLY the neontune domain. NEVER output more than one step.', 'llama3:latest', 'neontune', '1.0.0', NULL);
-- Table: workflows
-- Table: agent_tools
INSERT INTO agent_tools VALUES(7438649930744467458, 7438474706665213952, 7438474706690379776, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438649930744467459, 7438474706665213952, 7438474706694574089, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438649930744467460, 7438474706665213952, 7438474706711351296, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438649930748661761, 7438474706661019649, 7438474706690379776, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438649930748661762, 7438474706661019649, 7438474706681991168, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438649930748661763, 7438474706661019648, 7438474706690379777, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438649930748661764, 7438474706661019648, 7438474706690379778, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438649930748661766, 7438649930752856068, 7438474706694574089, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438649930748661767, 7438649930752856068, 7438474706694574080, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438649930748661768, 7438649930752856068, 7438474706694574081, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438649930748661769, 7438649930752856068, 7438474706694574087, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438649930748661770, 7438649930752856068, 7438474706707156993, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438649930748661771, 7438649930752856068, 7438474706694574088, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438649930748661772, 7438649930752856068, 7438474706694574082, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438649930748661773, 7438649930752856068, 7438474706694574085, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438649930748661774, 7438649930752856068, 7438474706694574084, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438649930748661775, 7438649930752856068, 7438474706694574086, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438649930748661776, 7438649930752856068, 7438474706707156992, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438649930748661777, 7438649930752856068, 7438474706694574083, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438649930752856064, 7438649930757050370, 7438474706694574089, '2026-03-14 18:19:06');
INSERT INTO agent_tools VALUES(7438500000000000001, 7438474706665213955, 9223372036854775807, '2026-03-15 06:05:04');
INSERT INTO agent_tools VALUES(7438982385762439168, 7438474706661019648, 7438900000000000004, '2026-03-15 16:16:46');
INSERT INTO agent_tools VALUES(7439259475401379840, 7438900000000000001, 7438474706694574083, '2026-03-16 08:02:41');
INSERT INTO agent_tools VALUES(7439298655871438848, 7438900000000000001, 7770000000000000002, '2026-03-16 13:16:08');
INSERT INTO agent_tools VALUES(999900000000000002, 7439380445306949632, 7438474706694574083, '2026-03-16 18:29:59');
INSERT INTO agent_tools VALUES(1742000000000002, 7438474706665213954, 1742000000000000, '2026-03-19 07:26:26');
-- Table: supervisor_agents
INSERT INTO supervisor_agents VALUES(7438818720132239370, 7438818720144822272, 7438474706661019648, '2026-03-15 15:05:07');
INSERT INTO supervisor_agents VALUES(7438818720132239371, 7438818720132239360, 7438649930757050370, '2026-03-15 15:05:07');
INSERT INTO supervisor_agents VALUES(7438818720132239372, 7438818720132239362, 7438649930757050370, '2026-03-15 15:05:07');
INSERT INTO supervisor_agents VALUES(7438818720132239373, 7438818720132239363, 7438900000000000001, '2026-03-15 15:23:27');
INSERT INTO supervisor_agents VALUES(1742000000000001, 7438818720144822273, 7438474706665213954, '2026-03-19 07:26:26');
-- Table: agent_planners
INSERT INTO agent_planners VALUES(1, 7438900000000000002, 7438900000000000001);
INSERT INTO agent_planners VALUES(2, 7438900000000000002, 7438900000000000001);



-- Table: workflow_steps
-- Table: workflow_runs
-- Table: workflow_step_runs
-- Table: messages
PRAGMA foreign_keys = ON;
