//! Tool-calling evaluation harness.
//!
//! Sends each case's prompt through Overseer's provider layer with tools
//! available and records the tool calls the model chooses. Nothing is
//! executed: only the model's first turn is captured and later scored by
//! `evals/score.py`.
//!
//!   cargo run --release --example eval_tools -- \
//!       --cases evals/cases/overseer_tools.jsonl --model qwen2.5:7b \
//!       --out evals/results/overseer_tools.qwen2.5-7b.jsonl
//!
//! Cases are JSON lines with `id` and `prompt`, plus optional `tools`
//! (`[{name, description, parameters}]`, defaulting to Overseer's catalog)
//! and `system` (`"overseer"`, the default, or `"none"`). Results already in
//! `--out` are skipped, so an interrupted run can be resumed.

use std::collections::HashSet;
use std::io::{BufRead, Write};
use std::time::Instant;

use overseer::engine::agent_loop::{tool_catalog, SYSTEM_PROMPT};
use overseer::engine::providers::{
    complete_with_tools, AgentMessage, Provider, ProviderConfig, ToolSpec,
};
use serde_json::{json, Value};

struct Args {
    cases: String,
    model: String,
    out: String,
    limit: Option<usize>,
    base_url: String,
}

fn parse_args() -> Args {
    let mut args = std::env::args().skip(1);
    let mut cases = None;
    let mut model = None;
    let mut out = None;
    let mut limit = None;
    let mut base_url = "http://localhost:11434".to_string();
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .unwrap_or_else(|| panic!("missing value for {flag}"));
        match flag.as_str() {
            "--cases" => cases = Some(value),
            "--model" => model = Some(value),
            "--out" => out = Some(value),
            "--limit" => limit = Some(value.parse().expect("--limit must be a number")),
            "--base-url" => base_url = value,
            other => panic!("unknown flag {other}"),
        }
    }
    Args {
        cases: cases.expect("--cases is required"),
        model: model.expect("--model is required"),
        out: out.expect("--out is required"),
        limit,
        base_url,
    }
}

fn tools_for(case: &Value) -> Vec<ToolSpec> {
    match case.get("tools").and_then(Value::as_array) {
        Some(tools) => tools
            .iter()
            .map(|t| ToolSpec {
                name: t["name"].as_str().unwrap_or_default().to_string(),
                description: t["description"].as_str().unwrap_or_default().to_string(),
                parameters: t["parameters"].clone(),
            })
            .collect(),
        None => tool_catalog(),
    }
}

fn done_ids(path: &str) -> HashSet<String> {
    let Ok(file) = std::fs::File::open(path) else {
        return HashSet::new();
    };
    std::io::BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str::<Value>(&line).ok())
        .filter_map(|v| v["id"].as_str().map(str::to_string))
        .collect()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = parse_args();
    let cfg = ProviderConfig {
        provider: Provider::Ollama,
        api_key: String::new(),
        base_url: args.base_url.clone(),
        model: args.model.clone(),
    };

    let cases: Vec<Value> = std::io::BufReader::new(std::fs::File::open(&args.cases)?)
        .lines()
        .map_while(Result::ok)
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(&line))
        .collect::<Result<_, _>>()?;
    let skip = done_ids(&args.out);
    let mut out = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&args.out)?;

    let todo: Vec<&Value> = cases
        .iter()
        .filter(|c| !skip.contains(c["id"].as_str().unwrap_or_default()))
        .take(args.limit.unwrap_or(usize::MAX))
        .collect();
    eprintln!(
        "{} cases, {} already done, running {}",
        cases.len(),
        skip.len(),
        todo.len()
    );

    for (i, case) in todo.iter().enumerate() {
        let id = case["id"].as_str().unwrap_or_default();
        let system = match case.get("system").and_then(Value::as_str) {
            Some("none") => "",
            _ => SYSTEM_PROMPT,
        };
        let messages = [AgentMessage::User(
            case["prompt"].as_str().unwrap_or_default().to_string(),
        )];
        let tools = tools_for(case);

        let started = Instant::now();
        let turn = complete_with_tools(&cfg, system, &messages, &tools).await;
        let latency_ms = started.elapsed().as_millis() as u64;

        let record = match turn {
            Ok(turn) => json!({
                "id": id,
                "model": args.model,
                "latency_ms": latency_ms,
                "text": turn.text,
                "tool_calls": turn.tool_calls.iter()
                    .map(|c| json!({ "name": c.name, "arguments": c.arguments }))
                    .collect::<Vec<_>>(),
            }),
            Err(err) => json!({
                "id": id,
                "model": args.model,
                "latency_ms": latency_ms,
                "error": err.to_string(),
            }),
        };
        writeln!(out, "{record}")?;
        eprintln!("[{}/{}] {id} {latency_ms} ms", i + 1, todo.len());
    }
    Ok(())
}
