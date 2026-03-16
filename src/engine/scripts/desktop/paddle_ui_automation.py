#!/usr/bin/env python3
"""
PaddleOCR-based UI Automation script for testing.
Replaces Tesseract OCR with PaddleOCR to compare performance and accuracy.
"""

import os
# SET FLAGS BEFORE ANY IMPORTS
os.environ["PADDLE_PDX_DISABLE_MODEL_SOURCE_CHECK"] = "True"
os.environ["FLAGS_use_onednn"] = "0"
os.environ["FLAGS_use_mkldnn"] = "0"
os.environ["FLAGS_enable_pir_api"] = "0"
os.environ["FLAGS_enable_executor_v2"] = "0"
os.environ["PADDLE_RECOGNIZER_BATCH_SIZE"] = "1"
os.environ["FLAGS_allocator_strategy"] = "naive_best_fit"

import sys
import time
import subprocess
import pyautogui
import argparse
import json
import tempfile
import paddle
from paddleocr import PaddleOCR

# Disable pyautogui failsafe for automation
pyautogui.FAILSAFE = False

try:
    # Strategy 1: Modern 3.x/PaddleX initialization
    ocr = PaddleOCR(lang='en', use_textline_orientation=False)
except Exception:
    try:
        # Strategy 2: Stable 2.x initialization (Standard)
        ocr = PaddleOCR(lang='en', use_angle_cls=False, use_gpu=False)
    except Exception as e:
        if "AnalysisConfig" in str(e):
            print(f"❌ Version Mismatch detected: {e}", file=sys.stderr)
            print("You have PaddleOCR 3.x with PaddlePaddle 2.x. This is incompatible.")
            print("Please align them by running:")
            print("pip install \"paddleocr<3.0.0\" \"paddlepaddle<3.0.0\" --break-system-packages")
            sys.exit(1)
        try:
             # Strategy 3: Absolute barebones
             ocr = PaddleOCR(lang='en')
        except Exception as e:
            print(f"❌ Failed to initialize PaddleOCR: {e}", file=sys.stderr)
            print("Please ensure consistent versions are installed:")
            print("pip install \"paddleocr<3.0.0\" \"paddlepaddle<3.0.0\" --break-system-packages")
            sys.exit(1)

def get_window_geometry(wid):
    """Returns (x, y, width, height) of the window."""
    try:
        out = subprocess.check_output(['xdotool', 'getwindowgeometry', str(wid)]).decode()
        pos_line = [l for l in out.split('\n') if 'Position:' in l][0]
        geo_line = [l for l in out.split('\n') if 'Geometry:' in l][0]
        
        # Use regex to find geometry
        import re
        x, y = map(int, re.search(r'Position: (\d+),(\d+)', pos_line).groups())
        w, h = map(int, re.search(r'Geometry: (\d+)x(\d+)', geo_line).groups())
        return x, y, w, h
    except:
        return 0, 0, 0, 0

def get_window_id(window_title):
    try:
        out = subprocess.check_output(['xdotool', 'search', '--name', window_title])
        wids = out.decode().strip().split('\n')
        if not wids: return None
        return wids[-1] # Return the last/most recent window
    except:
        return None

def get_screen_text(app_name, grep_str=None):
    """Captures window and returns JSON list of all text with coords."""
    print(f"📖 Reading screen text using PaddleOCR for {app_name}...", file=sys.stderr)
    
    wid = None
    if app_name.lower() == "active":
        try:
            wid = subprocess.check_output(['xdotool', 'getactivewindow']).decode().strip()
        except: pass
    else:
        subprocess.run(['wmctrl', '-a', app_name], check=False)
        wid = get_window_id(app_name)

    if not wid:
        return json.dumps({"error": "Window not found"})
        
    time.sleep(0.5)
    
    try:
        tmp_img = "/tmp/rig_paddle_capture.png"
        subprocess.run(['import', '-window', wid, tmp_img], check=True)
        
        win_x, win_y, _, _ = get_window_geometry(wid)
        
        start_time = time.time()
        # Newer PaddleOCR might prefer predict() but ocr() should still work if flags are right
        # We try to use the most stable call
        result = ocr.ocr(tmp_img)
        elapsed = time.time() - start_time
        print(f"⚡ OCR took {elapsed:.2f} seconds", file=sys.stderr)

        final_results = []
        if result and result[0]:
            for line in result[0]:
                if not line or len(line) < 2: continue
                points = line[0] # [[x1, y1], [x2, y1], [x2, y2], [x1, y2]]
                text = line[1][0]
                conf = line[1][1]
                
                if grep_str and grep_str.lower() not in text.lower():
                    continue
                
                # Bounding box
                x0 = min(p[0] for p in points)
                y0 = min(p[1] for p in points)
                x1 = max(p[0] for p in points)
                y1 = max(p[1] for p in points)
                
                target_x = win_x + (x0 + x1) // 2
                target_y = win_y + (y0 + y1) // 2
                
                final_results.append({
                    "text": text,
                    "x": int(target_x),
                    "y": int(target_y),
                    "width": int(x1 - x0),
                    "height": int(y1 - y0),
                    "confidence": float(conf)
                })
        
        return json.dumps(final_results)
    except Exception as e:
        import traceback
        return json.dumps({"error": str(e), "trace": traceback.format_exc()})

def launch_app_safe(app_name):
    """Launches the application using a binary search."""
    print(f"🚀 Attempting to launch '{app_name}'...", file=sys.stderr)
    try:
        # 1. Try which
        path = subprocess.check_output(['which', app_name.lower()]).decode().strip()
        if path:
            subprocess.Popen([path])
            return True
    except: pass
    
    # 2. Try common locations
    for loc in [f"/usr/bin/{app_name.lower()}", f"/usr/local/bin/{app_name.lower()}"]:
        if os.path.exists(loc):
            subprocess.Popen([loc])
            return True
            
    # 3. Last ditch: try to run as command
    try:
        subprocess.Popen([app_name.lower()])
        return True
    except:
        return False

def main():
    # Flexible CLI: if arguments look like 'launch neontune' or 'search song'
    # we handle them as actions even without flags.
    
    raw_args = sys.argv[1:]
    
    action = None
    app = "active"
    text = None
    
    if len(raw_args) >= 2 and raw_args[0] in ["launch", "search", "click", "get_text"]:
        action = raw_args[0]
        if action == "launch":
            app = raw_args[1]
        else:
            text = " ".join(raw_args[1:])
    else:
        parser = argparse.ArgumentParser()
        parser.add_argument("--action", choices=["get_text", "click", "launch", "search"])
        parser.add_argument("--app", default="active")
        parser.add_argument("--text", help="Text to search for or click")
        
        args, unknown = parser.parse_known_args()
        
        if not args.action and unknown:
            # Try to infer action from first unknown arg
            if unknown[0] in ["launch", "search", "click", "get_text"]:
                action = unknown[0]
                if len(unknown) > 1:
                    if action == "launch": app = unknown[1]
                    else: text = " ".join(unknown[1:])
        else:
            action = args.action
            app = args.app
            text = args.text

    if not action:
        print("❌ No action specified (launch, search, click, get_text)")
        sys.exit(1)

    if action == "launch":
        if launch_app_safe(app):
            print(f"✅ Launched {app}. Waiting for window...")
            time.sleep(5)
            sys.exit(0)
        else:
            print(f"❌ Failed to launch {app}")
            sys.exit(1)
            
    if action == "search":
        if not text:
            print("❌ Search requires text")
            sys.exit(1)
            
        print(f"🔍 Searching for '{text}' in {app}...")
        # Step 1: Find and click Search button
        results = json.loads(get_screen_text(app, "Search"))
        if results and len(results) > 0:
            target = results[0]
            print(f"🖱️ Clicking 'Search' at ({target['x']}, {target['y']})")
            pyautogui.click(target['x'], target['y'])
            time.sleep(1)
            # Step 2: Type query
            pyautogui.hotkey('ctrl', 'a')
            pyautogui.press('backspace')
            pyautogui.write(text, interval=0.05)
            pyautogui.press('enter')
            print("✅ Search query submitted")
        else:
            # Fallback: Just type if Search field is likely focused or we can't find it
            print("⚠️ 'Search' button not found via OCR, trying direct type...")
            pyautogui.hotkey('ctrl', 'f') # Typical shortcut
            time.sleep(0.5)
            pyautogui.write(text)
            pyautogui.press('enter')
            
    elif action == "get_text":
        print(get_screen_text(app, text))
    elif action == "click":
        if not text:
            print("❌ --text required for click action")
            sys.exit(1)
        results = json.loads(get_screen_text(app, text))
        if isinstance(results, list) and len(results) > 0:
            target = results[0]
            print(f"🖱️ Clicking '{target['text']}' at ({target['x']}, {target['y']})")
            pyautogui.click(target['x'], target['y'])
        else:
            print(f"❌ Text '{text}' not found")

if __name__ == "__main__":
    main()
