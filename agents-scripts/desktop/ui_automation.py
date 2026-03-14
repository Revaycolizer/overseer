#!/usr/bin/env python3
"""
Adaptive UI Automation script using OCR.
Finds 'Search' and specific text on screen dynamically.
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
    paddle_ocr_engine = PaddleOCR(lang='en', use_angle_cls=False, use_gpu=False, show_log=False)
    print("🌀 Initialized PaddleOCR engine backend.", file=sys.stderr)
except Exception as e:
    print(f"❌ CRITICAL: Failed to init PaddleOCR: {e}. Automation will fail.", file=sys.stderr)
    sys.exit(1)

import gi
try:
    gi.require_version('Atspi', '2.0')
    from gi.repository import Atspi
except ImportError:
    Atspi = None
except ValueError:
    Atspi = None

# Disable pyautogui failsafe for automation
pyautogui.FAILSAFE = False


# word_pattern = re.compile(r'title=[\'"]bbox (\d+) (\d+) (\d+) (\d+);?\s*x_wconf \d+[\'"]>\s*([^<]+)\s*</span>')

DEBUG_DRAW = False

CONFIG_PATH = os.path.expanduser("~/.config/rig/apps.json")

def load_config():
    if os.path.exists(CONFIG_PATH):
        try:
            with open(CONFIG_PATH, 'r') as f:
                return json.load(f)
        except:
            pass
    return {}

def get_window_geometry(wid):
    """Returns (x, y, width, height) of the window."""
    try:
        out = subprocess.check_output(['xdotool', 'getwindowgeometry', str(wid)]).decode()
        # Window 83886092
        #   Position: 166,132 (screen: 0)
        #   Geometry: 800x600
        pos_line = [l for l in out.split('\n') if 'Position:' in l][0]
        geo_line = [l for l in out.split('\n') if 'Geometry:' in l][0]
        
        x, y = map(int, re.search(r'Position: (\d+),(\d+)', pos_line).groups())
        w, h = map(int, re.search(r'Geometry: (\d+)x(\d+)', geo_line).groups())
        return x, y, w, h
    except:
        return 0, 0, 0, 0

def get_window_id(window_title):
    try:
        # Get all IDs matching the name
        out = subprocess.check_output(['xdotool', 'search', '--name', window_title])
        wids = out.decode().strip().split('\n')
        
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
            wid = subprocess.check_output(['xdotool', 'getactivewindow']).decode().strip()
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
        subprocess.run(['import', '-window', wid, tmp_img], check=True)
        
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
        subprocess.run(['import', '-window', wid, tmp_img], check=True)
        
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
                        print(f"👁️ [Paddle] Found candidate '{text}' at Screen({screen_x},{screen_y})")
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
    print(f"🎯 Focusing '{window_title}'...", file=sys.stderr)
    # 1. Try direct match
    subprocess.run(['wmctrl', '-a', window_title], check=False)
    wid = get_window_id(window_title)
    
    # 2. If not found, try case-insensitive search
    if not wid:
        try:
            output = subprocess.check_output(['wmctrl', '-l']).decode('utf-8')
            for line in output.splitlines():
                parts = line.split(maxsplit=3)
                if len(parts) > 3:
                     current_wid = parts[0]
                     current_title = " ".join(parts[3:]) if len(parts) > 3 else parts[3]
                     if window_title.lower() in current_title.lower():
                         print(f"🎯 Found partial match: '{current_title}'")
                         subprocess.run(['wmctrl', '-ia', current_wid], check=False)
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
        subprocess.run(['xdotool', 'windowactivate', '--sync', wid], check=False)
        subprocess.run(['xdotool', 'windowraise', wid], check=False)
        return wid
    
    # 3. Last ditch effort: if app_name is likely a browser, try searching for "Chrome" or "Firefox"
    if window_title.lower() in ["chrome", "google-chrome", "chromium"]:
        for title in ["Google Chrome", "Chromium"]:
            wid = get_window_id(title)
            if wid:
                subprocess.run(['xdotool', 'windowactivate', '--sync', wid], check=False)
                subprocess.run(['xdotool', 'windowraise', wid], check=False)
                return wid
                
    return None

def wait_for_text(app_name, text, timeout=20):
    """Wait until specific text appears on screen"""
    print(f"⏳ Waiting for text '{text}' in {app_name} (timeout={timeout}s)...")
    start_time = time.time()
    while time.time() - start_time < timeout:
        screen_text = get_screen_text(app_name)
        if text.lower() in screen_text.lower():
            print(f"✅ Found text '{text}'!")
            return True
        time.sleep(2)
    print(f"❌ Timeout waiting for '{text}'")
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
        # Actually it's simpler to iterate and find the one with 'active' state, 
        # but let's try to match the window title from xdotool if we can.
        # For robustness, let's search for the window that has STATE_ACTIVE
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
                        # Let's return the app for now as the root.
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
        print(f"⚠️ App '{app_name}' not found by name, deep searching...", file=sys.stderr)
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
                 wid = subprocess.check_output(['xdotool', 'getactivewindow']).decode().strip()
             except: pass
        else:
             wid = get_window_id(win_title)
             if not wid and app_name.lower() == "chrome":
                  wid = get_window_id("Google Chrome")

        if wid:
            tmp_shot = "/tmp/rig_visual_hash.png"
            # Use import -window (fast)
            subprocess.run(['import', '-window', wid, '-resize', '640x480', tmp_shot], 
                         check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            
            with open(tmp_shot, 'rb') as f:
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
         # For now, let's treat it as a "Potential Change".
         pass

    # Standard Element Diff
    old_set = {f"{e['role']}:{e['text']}" for e in old_elements}
    new_set = {f"{e['role']}:{e['text']}" for e in new_elements}
    
    added = [e for e in new_elements if f"{e['role']}:{e['text']}" not in old_set]
    removed_count = len([e for e in old_elements if f"{e['role']}:{e['text']}" not in new_set])
    
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
    print(f"🔍 Searching for '{query}' in {app_name} (Hybrid Mode)...", file=sys.stderr)
    
    # 0. Focus App
    config = load_config()
    profile = config.get(app_name.lower(), {})
    win_title = resolve_window_title(app_name, config)
    
    wid = focus_window(win_title)
    if not wid:
        if no_launch:
            print(f"❌ '{app_name}' not running and --no-launch check failed.", file=sys.stderr)
            return False
        # Launch
        if not launch_app_safe(app_name):
            return False
        wid = focus_window(win_title)
        if not wid:
            print("❌ Failed to launch/focus app.", file=sys.stderr)
            return False

    time.sleep(1.0) # Wait for focus
    subprocess.run(['xdotool', 'windowraise', wid], check=False)

    # Layer 1: Generic Ctrl+F (Most Apps)
    # But wait, browsers use Ctrl+L for address bar, or Ctrl+F for find in page.
    # We want GLOBAL search usually?
    # Context matters. For "Search for text", Ctrl+F is fine.
    # For "Search web", Ctrl+L is better.
    # Let's try to detect context or just try Ctrl+F first?
    # Actually OCR is cleaner.
    
    # Layer 2: OCR "Search"
    print("👉 Layer 2: OCR Search Button", file=sys.stderr)
    # Look for "Search", "Find", "Start"
    for kw in ["Search", "Find", "Query"]:
        fx, fy = capture_and_find_text(wid, kw)
        if fx:
            print(f"✅ OCR found '{kw}' at ({fx}, {fy}).")
            pyautogui.click(fx, fy)
            time.sleep(0.5)
            # Clear text if needed?
            pyautogui.hotkey('ctrl', 'a')
            pyautogui.write(query, interval=0.05)
            pyautogui.press('enter')
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
            pyautogui.hotkey('ctrl', 'a')
            pyautogui.press('backspace')
            time.sleep(0.1)
            pyautogui.write(query, interval=0.05)
            pyautogui.press('enter')
            print("✅ Search submitted via Heuristic.")
            # We can't be sure, but we tried.
            return True
    except Exception as e:
        print(f"⚠️ Layer 3 failed: {e}")

    # Layer 4: Browser Specific (Ctrl+L)
    # Only if it LOOKS like a browser or explicitly allowed
    is_browser = app_name.lower() in ["chrome", "firefox", "chromium", "brave", "edge", "browser"]
    if is_browser:
        print("👉 Layer 4: Browser Fallback (Ctrl+L)", file=sys.stderr)
        pyautogui.hotkey('ctrl', 'l')
        time.sleep(0.5)
        # Ctrl+L usually selects all, but let's be safe
        pyautogui.press('backspace')
        pyautogui.write(query)
        pyautogui.press('enter')
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
        # Can't easily copy-paste 300 lines. 
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
    signature = "|".join([f"{e['role']}:{e['text']}" for e in elements])
    import hashlib
    return hashlib.md5(signature.encode()).hexdigest() if hasattr(hashlib, 'md5') else str(hash(signature))

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
    win_title = resolve_window_title(app_name, config) # Default to app_name instead of 'tao'
    
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

    # 3. Find 'Search'
    print("👀 Looking for 'Search' button...")
    sx, sy = capture_and_find_text(wid, "Search")
    
    if sx:
        print(f"🖱️ Clicking 'Search' at ({sx}, {sy})...", file=sys.stderr)
        pyautogui.click(sx, sy)
    else:
        print("❌ 'Search' not found. Aborting to prevent hallucination.", file=sys.stderr)
        return

    # 4. Type
    print(f"⌨️ Typing '{song_query}'...", file=sys.stderr)
    time.sleep(1) # Wait for animation
    # Clear just in case
    pyautogui.hotkey('ctrl', 'a')
    pyautogui.press('backspace')
    
    pyautogui.write(song_query, interval=0.1)
    pyautogui.press('enter')
    
    print("⏳ Waiting for results (3s)...", file=sys.stderr)
    time.sleep(3)
    
    # 5. Find Result (Artist or Title)
    # Search is case insensitive fuzzy
    # Split query into potential keywords
    parts = song_query.replace('-', ' ').split()
    # Filter short words (avoid matching 'a', 'the', 'in')
    keywords = [w for w in parts if len(w) > 2]
    
    found_x, found_y = None, None
    
    # Try keywords 
    for word in keywords:
        print(f"👀 Looking for result keyword '{word}'...", file=sys.stderr)
        fx, fy = capture_and_find_text(wid, word)
        if fx:
            found_x, found_y = fx, fy
            break # Found a match
    
    if found_x:
        print(f"🖱️ Clicking Result matching '{word}' at ({found_x}, {found_y})...")
        pyautogui.click(found_x, found_y)
        pyautogui.press('enter')
        print("✅ Playing!")
    else:
        print("❌ No keywords found in OCR. Aborting to prevent hallucination.")
        return


def click_text(app_name, text):
    """Focus app, find text via AT-SPI (Data) then OCR (Visual), and click it."""
    print(f"🖱️ Request to click text '{text}' in {app_name}...", file=sys.stderr)
    
    # 1. Config/Focus
    config = load_config()
    profile = config.get(app_name.lower(), {})
    win_title = resolve_window_title(app_name, config)
    
    wid = focus_window(win_title)
    if not wid:
        print(f"❌ Could not focus {app_name} to click text.", file=sys.stderr)
        return False
        
    time.sleep(1.0) # Wait for focus and render
    subprocess.run(['xdotool', 'windowraise', wid], check=False)

    # 2. Strategy A: Data-First (AT-SPI)
    # 100% Precision for accessible apps (Chrome, Firefox, LibreOffice, Gedit, etc.)
    print(f"🧠 Data-Scan: Searching Accessibility Tree for '{text}'...", file=sys.stderr)
    
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
                print(f"✅ Data-Hit (Exact): Found '{el['text']}' ({el_role}) at ({el['x']}, {el['y']})")
                pyautogui.click(el['x'], el['y'])
                return True
                
            # 2. Contains Match
            if target_lower in el_text:
                candidates.append(el)

        # If no exact match, check candidates
        if candidates:
            # Sort by length (shortest match is likely the label itself, not a huge paragraph containing it)
            candidates.sort(key=lambda x: len(x.get("text", "")))
            best = candidates[0]
            print(f"✅ Data-Hit (Fuzzy): Found '{best['text']}' ({best['role']}) at ({best['x']}, {best['y']})")
            pyautogui.click(best['x'], best['y'])
            return True
            
    print("⚠️ Data-Scan: Text not found in Accessibility Tree. Falling back to Vision (OCR)...", file=sys.stderr)

    # 3. Strategy B: Vision-Fallback (OCR)
    # Used for Canvas apps, Games, or non-accessible widgets
    print(f"👀 Vision-Scan: OCR Scanning for '{text}'...", file=sys.stderr)
    matches = find_text_matches(wid, text)
    
    if not matches:
        print(f"⚠️ Text '{text}' not found in {app_name} (Data nor Vision).", file=sys.stderr)
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
        # Fallback to first match if that's all we have
        print("⚠️ Only found matches in header/top area. Clicking anyway.")
        target = matches[0]

    fx, fy, word = target
    print(f"✅ Vision-Hit: Selected '{word}' at ({fx}, {fy}). Clicking...", file=sys.stderr)
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
        print(f"⚠️ Cannot validate coordinates: Window '{win_title}' not found.", file=sys.stderr)
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
        wids = [w for w in out.split('\n') if w]
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
                print(f"🔍 Dynamic window found for '{app_name}': '{best_title}'", file=sys.stderr)
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
        print(f"✅ App '{app_name}' is already running with a window. Skipping launch.", file=sys.stderr)
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
            print(f"⏳ Verification: Waiting for '{app_name}' window to appear...", file=sys.stderr)
            for _ in range(10): # 10 seconds timeout
                if focus_window(win_title):
                    print(f"✅ Verified: '{app_name}' launched and focused.", file=sys.stderr)
                    return True
                time.sleep(1)
            print(f"❌ Failed to launch '{app_name}'. Process started but window not found.", file=sys.stderr)
            return False
        except FileNotFoundError:
            continue
        except Exception as e:
            print(f"⚠️ Launch error for {cmd_str}: {e}")
            continue
            
    # Fallback: Try the app name directly if it wasn't already tried
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
            print(f"⏳ Verification: Waiting for '{app_name}' window to appear...", file=sys.stderr)
            for _ in range(10): # 10 seconds timeout
                if focus_window(win_title):
                    print(f"✅ Verified: '{app_name}' launched and focused.", file=sys.stderr)
                    return True
                time.sleep(1)
            print(f"❌ Failed to launch '{app_name}'. Process started but window not found.", file=sys.stderr)
            return False
        except:
            pass

    print(f"❌ Failed to launch '{app_name}'. No matching executable found.")
    return False

def search_in_app(app_name, query, no_launch=False):
    """
    Hybrid Search Strategy:
    1. Atspi (Accessibility Tree) - Find 'Search'/'Find' inputs.
    2. OCR (Visual) - Find text 'Search', 'Type to search'.
    3. Heuristic - Click Top-Right (Search Icon area).
    4. Browser Fallback - Ctrl+L.
    """
    print(f"🔍 Searching for '{query}' in {app_name} (Hybrid Mode)...")
    
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
                print(f"✅ Atspi found target: '{target_el['text']}' ({target_el['role']}) at ({target_el['x']},{target_el['y']})")
                pyautogui.click(target_el['x'], target_el['y'])
                time.sleep(0.5)
                # Select all just in case
                if role == "entry": pyautogui.hotkey('ctrl', 'a')
                pyautogui.write(query, interval=0.05)
                pyautogui.press('enter')
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
            print(f"✅ OCR found '{kw}' at ({fx}, {fy}).")
            pyautogui.click(fx, fy)
            time.sleep(0.5)
            # Clear text if needed?
            pyautogui.hotkey('ctrl', 'a')
            pyautogui.write(query, interval=0.05)
            pyautogui.press('enter')
            print("✅ Search submitted via OCR.")
            return True

    # Layer 3: Browser Specific (Ctrl+L) - FALLBACK
    # Only use if we couldn't find an on-page search bar
    is_browser = app_name.lower() in ["chrome", "google-chrome", "firefox", "chromium", "brave", "edge", "browser"]
    if is_browser:
        print("👉 Layer 3: Browser Shortcut (Ctrl+L)", file=sys.stderr)
        pyautogui.hotkey('ctrl', 'l')
        time.sleep(0.5)
        pyautogui.write(query, interval=0.05)
        pyautogui.press('enter')
        print("✅ Search submitted via Browser Shortcut.")
        return True
        pyautogui.write(query)
        pyautogui.press('enter')
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
                    subprocess.run(['xdotool', 'windowraise', wid], check=False)
                    time.sleep(0.5)
                else:
                    # STRICT FOCUS CHECK
                    print(f"❌ Focus Failed: Could not find window matching '{win_title}' for app '{args.app}'. Aborting click.", file=sys.stderr)
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
        # direction: 'up' or 'down' (text arg)
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
            # STRICT FOCUS CHECK: Do not type if we can't find the window!
            if not focus_window(win_title):
                print(f"❌ Focus Failed: Could not find window matching '{win_title}' for app '{args.app}'. Aborting type.", file=sys.stderr)
                sys.exit(1)
            
        if args.text:
            target_text = args.text
            print(f"⌨️ Targeting text: {target_text}", file=sys.stderr)
            
            # ATTEMPT TO DETECT EXISTING TEXT
            already_typed = False
            try:
                # Get the focused element's name/value via atspi
                desktop = Atspi.get_desktop(0)
                focused = desktop.get_focused_element()
                if focused:
                    val = focused.get_name() or ""
                    if target_text.lower() in val.lower() or val.lower() in target_text.lower():
                        print(f"✅ Field already contains '{val}'. Skipping redundant typing.", file=sys.stderr)
                        already_typed = True
            except:
                pass

            if not already_typed:
                if args.clear:
                    print("⌨️ Clearing field...", file=sys.stderr)
                    pyautogui.hotkey('ctrl', 'a')
                    pyautogui.press('backspace')
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
            pyautogui.hotkey('alt', 'F4')
        else:
            print("⎋ Dismissing active popup via Escape...", file=sys.stderr)
            pyautogui.press('escape')
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
