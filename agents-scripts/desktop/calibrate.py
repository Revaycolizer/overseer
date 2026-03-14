import pyautogui
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
        input("\n1. Hover over the **Search Bar** (text input) and press ENTER...")
        sx, sy = pyautogui.position()
        print(f"   Captured: {sx}, {sy}")
        
        # 2. First Result
        input("\n2. Type a search, wait for results, then hover over the **First Song Row** and press ENTER...")
        rx, ry = pyautogui.position()
        print(f"   Captured: {rx}, {ry}")
        
        config = {
            "search": [sx, sy],
            "result": [rx, ry]
        }
        
        with open(CONFIG_FILE, "w") as f:
            json.dump(config, f)
            
        print(f"\n✅ Configuration saved to {CONFIG_FILE}")
        print("Automation attempts will now use these exact coordinates automatically.")
        
    except KeyboardInterrupt:
        print("\nCalibration cancelled.")
        sys.exit(1)

if __name__ == "__main__":
    calibrate()
