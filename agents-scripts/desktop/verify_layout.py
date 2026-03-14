import sys
import time
import subprocess
import pyautogui
import os

# Reuse functions from ui_automation.py implicitly or re-define
def reset_window_layout(window_name="tao window"):
    print(f"📏 Resetting '{window_name}'...")
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
    print(f"⌨️ Typing '{text}'...")
    pyautogui.hotkey('ctrl', 'a')
    pyautogui.press('backspace')
    pyautogui.write(text, interval=0.1)
    pyautogui.press('enter')
    
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
