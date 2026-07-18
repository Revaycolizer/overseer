//! Static bypass guard (enforced, not advisory).
//!
//! Fails the build if a low-level tool executor is referenced outside the
//! reviewed allowlist of files. This is how "one governed execution path" is
//! kept true over time: any NEW caller of `call_dynamic_tool` / `native_os_tool`
//! — e.g. a new route or agent path — breaks this test until it either goes
//! through `engine::kernel::kernel_execute` or is explicitly reviewed and added
//! to the allowlist here.
//!
//! It runs from the kernel crate (which compiles without the desktop GUI stack)
//! but scans the main `overseer` crate's `src/` tree, so it is enforceable in
//! this environment and in CI regardless of the desktop toolchain.

use std::fs;
use std::path::{Path, PathBuf};

fn main_src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("src")
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn rel(src: &Path, file: &Path) -> String {
    file.strip_prefix(src)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Files permitted to reference `call_dynamic_tool` (the low-level executor).
const CALL_DYNAMIC_TOOL_ALLOW: &[&str] = &[
    // The single governed entry point.
    "engine/kernel.rs",
    // Definition + internal dispatch.
    "engine/tools/executor/mod.rs",
    "engine/tools/mod.rs",
    // Dead rig `Tool` router (never attached to an agent; not LLM-reachable).
    "engine/tools/router.rs",
    // Developer CLI (human-invoked `overseer info`), chart render only.
    "engine/shell.rs",
    // Voice loop: gated by kernel::evaluate_named before the call.
    "engine/orchestrator/loop.rs",
    // Read-only grounding observation (get_state / live desktop snapshot).
    "engine/orchestrator/discovery/grounding/mod.rs",
];

/// Files permitted to reference `native_os_tool` (the low-level OS executor).
const NATIVE_OS_TOOL_ALLOW: &[&str] = &[
    "engine/tools/mod.rs",
    // Implementation subtree.
    // (matched by prefix below)
    // Dispatch reached only via call_dynamic_tool (i.e. through the kernel).
    "engine/tools/executor/rust.rs",
    "engine/tools/executor/native.rs",
    // Gated voice paths.
    "engine/orchestrator/loop.rs",
    "engine/voice/pipeline.rs",
];

fn allowed(rel_path: &str, allow: &[&str]) -> bool {
    // os_tool implementation subtree is always allowed to reference native_os_tool.
    if rel_path.starts_with("engine/tools/os_tool/") {
        return true;
    }
    allow.iter().any(|a| a == &rel_path)
}

#[test]
fn no_ungoverned_execution_calls() {
    let src = main_src_dir();
    assert!(
        src.is_dir(),
        "expected main crate src at {} (run from the workspace)",
        src.display()
    );

    let mut files = Vec::new();
    collect_rs(&src, &mut files);
    assert!(
        !files.is_empty(),
        "no .rs files scanned under {}",
        src.display()
    );

    let mut violations = Vec::new();
    for file in &files {
        let rel_path = rel(&src, file);
        let Ok(text) = fs::read_to_string(file) else {
            continue;
        };
        if text.contains("call_dynamic_tool(") && !allowed(&rel_path, CALL_DYNAMIC_TOOL_ALLOW) {
            violations.push(format!(
                "{rel_path}: references call_dynamic_tool outside the governed allowlist"
            ));
        }
        if text.contains("native_os_tool(") && !allowed(&rel_path, NATIVE_OS_TOOL_ALLOW) {
            violations.push(format!(
                "{rel_path}: references native_os_tool outside the governed allowlist"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "ungoverned tool-execution call sites found (route them through \
         engine::kernel::kernel_execute or review + add to the allowlist):\n{}",
        violations.join("\n")
    );
}

#[test]
fn no_raw_shell_in_voice_loop() {
    // The voice loop must never resurrect `sh -c` / `powershell -Command`.
    let src = main_src_dir();
    let loop_rs = src.join("engine/orchestrator/loop.rs");
    let text = fs::read_to_string(&loop_rs).expect("read loop.rs");
    assert!(
        !text.contains(r#"arg("-c")"#),
        "loop.rs must not use `sh -c` (raw shell) for command execution"
    );
    assert!(
        !text.contains(r#"arg("-Command")"#),
        "loop.rs must not use `powershell -Command` (raw shell) for command execution"
    );
}
