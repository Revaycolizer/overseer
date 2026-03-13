import subprocess
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
        wid = subprocess.check_output(['xdotool', 'search', '--name', '--limit', '1', window_name]).decode().strip()
        print(f"✅ Found Window ID: {wid}")
        subprocess.run(['xdotool', 'windowactivate', '--sync', wid])
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
