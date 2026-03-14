#!/usr/bin/env python3
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
            print(f"❌ Search failed: {result.get('message', 'Unknown error')}")
        else:
            print(f"✅ Search complete: '{title}' @ {url}")
    elif args.action == "click":
        if status == "clicked":
            print(f"✅ Clicked. Now at: '{title}' @ {url}")
        else:
            print(f"⚠️ Click result: {result.get('message', status)}")
    elif args.action == "navigate":
        print(f"✅ Navigated to: '{title}' @ {url}")
    elif args.action == "get_links":
        if status == "navigated":
            print(f"✅ Found matches and Navigated to: '{title}' @ {url}")
        else:
            links = result.get("links", [])
            print(f"✅ Found {len(links)} links:")
            for l in links[:10]:
                print(f"  [{l.get('text','')[:60]}] -> {l.get('href','')[:80]}")
    elif args.action == "get_title":
        print(f"📄 '{title}' @ {url}")
    else:
        print(f"✅ {args.action}: {status}")

    # Always also print the full JSON for the Rust side to parse
    print(json.dumps(result))


if __name__ == "__main__":
    main()
