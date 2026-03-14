#!/usr/bin/env python3
import pyautogui
import json
import os
import argparse
import time

CONFIG_PATH = os.path.expanduser("~/.config/rig/apps.json")

def load_config():
    if os.path.exists(CONFIG_PATH):
        try:
            with open(CONFIG_PATH, 'r') as f:
                return json.load(f)
        except:
            pass
    return {}

def save_config(config):
    os.makedirs(os.path.dirname(CONFIG_PATH), exist_ok=True)
    with open(CONFIG_PATH, 'w') as f:
        json.dump(config, f, indent=4)
    print(f"✅ Configuration saved to {CONFIG_PATH}")

def configure(app_name):
    print(f"\n🔧 Configuring '{app_name}' automation profile.")
    print("------------------------------------------------")
    print("We need to teach the agent where the Search Bar and Results are.")
    print("Launch the app manually and arrange it how you like (or let the agent reset it later).")
    print("NOTE: It is best if the app is maximized or in a standard position.\n")

    input(f"1. Open '{app_name}' and Hover mouse over the **SEARCH BAR**. Press ENTER when ready...")
    sx, sy = pyautogui.position()
    print(f"   📍 Captured Search: {sx}, {sy}")
    
    time.sleep(1)
    
    input(f"2. Type a search (e.g. 'Test') and Hover over the **FIRST RESULT**. Press ENTER when ready...")
    rx, ry = pyautogui.position()
    print(f"   📍 Captured Result: {rx}, {ry}")

    # Load existing
    config = load_config()
    profile = config.get(app_name.lower(), {})
    
    # Update profile
    profile["window_title"] = app_name # User can edit json later if complex
    profile["launch_cmd"] = [app_name.lower()]
    profile["search_pos"] = [sx, sy]
    profile["result_pos"] = [rx, ry]
    # Default to NO reset if user is calibrating manually, 
    # OR we assume they want the reset behavior? 
    # Let's ask? No, keep it simple. Enable reset for consistency.
    profile["reset_layout"] = True 

    config[app_name.lower()] = profile
    save_config(config)
    print(f"\n✨ Setup Complete for {app_name}!")
    print(f"You can now run: cargo run -- \"play 'Song' in {app_name}\"")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--app", required=True, help="App name to configure")
    args = parser.parse_args()
    
    configure(args.app)
