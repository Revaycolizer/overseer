#!/usr/bin/env python3
"""
browser_server.py — Persistent Playwright browser daemon for the rig agent.

Keeps ONE browser window alive across all agent calls.
Run once in the background: python3 browser_server.py &
Then browser_automation.py sends HTTP commands to this server.

Port: 9223
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
    Object.defineProperty(navigator, 'webdriver', {get: () => undefined});
    window.chrome = { runtime: {} };
    Object.defineProperty(navigator, 'languages', {get: () => ['en-US', 'en']});
    Object.defineProperty(navigator, 'plugins', {get: () => [1,2,3,4,5]});
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
            for sel in ["input[name='q']", "input[type='search']", "textarea[name='q']"]:
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
                print(f"Fallback: Searching via direct URL for '{text}'", file=sys.stderr)
                try:
                    await _page.goto(
                        f"https://duckduckgo.com/?q={text.replace(' ', '+')}",
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
                print(f"DEBUG: Attempting selector click: '{selector}'", file=sys.stderr)
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
                        const targets = Array.from(document.querySelectorAll('button, a, input, [role="button"], [role="link"]'));
                        const t = {safe_text};
                        const exact = targets.find(el => (el.textContent || el.innerText || el.value || '').toLowerCase().trim() === t);
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
                    () => Array.from(document.querySelectorAll('a[href]'))
                        .filter(a => a.offsetParent !== null)
                        .map(a => ({
                            text: a.textContent.trim().toLowerCase().substring(0, 100),
                            href: a.href
                        }))
                        .filter(l => l.href.startsWith('http') && l.text.length > 0)
                        .slice(0, 300)
                """)

                def score(link):
                    l_text = link["text"].lower()
                    l_href = link["href"].lower()
                    t_text = text.lower().strip()
                    
                    # Domain detection
                    current_domain = current_url.split('/')[2] if '/' in current_url else ""
                    link_domain = l_href.split('/')[2] if '/' in l_href else ""
                    
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
                    print(f"Link score match: '{best['text'][:50]}' (Score: {matches[0][0]}) -> {best['href']}", file=sys.stderr)
                    await _page.goto(best["href"], wait_until="domcontentloaded", timeout=15000)
                    clicked = True

            # Step 4: Final Fuzzy Text Match (Any visible element)
            if not clicked and text:
                safe_text = json.dumps(text.lower().strip() if text else "")
                clicked = await _page.evaluate(f"""
                    () => {{
                        const targets = Array.from(document.querySelectorAll('*'));
                        const t = {safe_text};
                        const match = targets.find(el => {{
                            if (el.children.length > 0) return false; // leaf nodes only
                            return (el.innerText || el.value || '').toLowerCase().includes(t);
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
                result = {"status": "not_found", "message": f"No element or link matching '{text}'"}

        elif action == "type":
            submit = data.get("submit", False)
            safe_text = json.dumps(text.lower() if text else "")
            typed = await _page.evaluate(f"""
                async () => {{
                    const t = {safe_text};
                    const inputs = Array.from(document.querySelectorAll('input, textarea, [contenteditable="true"], [role="textbox"]'))
                        .filter(el => {{
                            const style = window.getComputedStyle(el);
                            return style.display !== 'none' && style.visibility !== 'hidden' && el.offsetParent !== null;
                        }});
                    
                    // Try to find by placeholder, aria-label, or value
                    let target = inputs.find(el => 
                        (el.placeholder || "").toLowerCase().includes(t) || 
                        (el.getAttribute("aria-label") || "").toLowerCase().includes(t) ||
                        (el.value || "").toLowerCase().includes(t)
                    );

                    // Fallback: If no direct match, take the first visible input if it's a search-like page
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
                result = {"status": "error", "message": f"No input field found to type '{text}'"}

        elif action == "get_links":
            links = await _page.evaluate("""
                (filterText) => Array.from(document.querySelectorAll('a[href]'))
                    .filter(a => a.offsetParent !== null)
                    .map(a => ({text: a.textContent.trim().substring(0,100), href: a.href}))
                    .filter(l => l.text.length > 0 && l.href.startsWith('http'))
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
                        return !!(e.offsetWidth || e.offsetHeight || e.getClientRects().length) && window.getComputedStyle(e).visibility !== 'hidden';
                    }
                    const elements = document.querySelectorAll('a, button, input, select, textarea');
                    let res = [];
                    for (let el of elements) {
                        if (!isVisible(el)) continue;
                        let tag = el.tagName.toLowerCase();
                        let text = (el.innerText || el.value || el.placeholder || '').trim().replace(/\\n/g, ' ');
                        if (!text) text = el.getAttribute('aria-label') || el.getAttribute('title') || '';
                        if (!text) continue;
                        let info = tag;
                        if (tag === 'a') {
                            let href = el.getAttribute('href') || '';
                            if (href.startsWith('/')) href = window.location.origin + href;
                            if (href.length < 80) info += ` href="${href}"`;
                        }
                        res.push(`[${info}] ${text.substring(0, 100)}`);
                    }
                    return res.slice(0, 800).join('\\n');
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
                result = {"status": "error", "message": "wait_for_text requires 'text' parameter"}
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
            msg = str(e) or f"Async operation failed for {data.get('action')}"
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
        self.wfile.write(b'{"status":"alive"}')


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
