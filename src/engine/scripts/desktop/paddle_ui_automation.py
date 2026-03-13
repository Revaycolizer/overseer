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

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--action", required=True, choices=["get_text", "click"])
    parser.add_argument("--app", default="active")
    parser.add_argument("--text", help="Text to search for or click")
    args = parser.parse_args()
    
    if args.action == "get_text":
        print(get_screen_text(args.app, args.text))
    elif args.action == "click":
        if not args.text:
            print("❌ --text required for click action")
            sys.exit(1)
        results = json.loads(get_screen_text(args.app, args.text))
        if isinstance(results, list) and len(results) > 0:
            target = results[0]
            print(f"🖱️ Clicking '{target['text']}' at ({target['x']}, {target['y']})")
            pyautogui.click(target['x'], target['y'])
        else:
            print(f"❌ Text '{args.text}' not found")

if __name__ == "__main__":
    main()
