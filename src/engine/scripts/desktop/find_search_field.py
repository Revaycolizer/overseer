#!/usr/bin/env python3
"""
Test script to find the correct search field coordinates in Neontune.
This will click different locations and wait for you to verify which one worked.
"""

import pyautogui
import subprocess
import time

# Launch Neontune
print("🚀 Launching Neontune...")
subprocess.Popen(['neontune'])
time.sleep(3)

# Test different coordinates
test_coords = [
    (400, 100, "Top center"),
    (500, 150, "Slightly right and down"),
    (300, 100, "Slightly left"),
    (400, 80, "Higher up"),
    (400, 120, "Lower down"),
]

for x, y, desc in test_coords:
    print(f"\n🖱️ Clicking {desc} at ({x}, {y})")
    print("   Watch where the cursor goes...")
    pyautogui.click(x, y)
    time.sleep(2)
    
    response = input("   Did this click the search field? (y/n): ")
    if response.lower() == 'y':
        print(f"\n✅ Found it! Search field is at ({x}, {y})")
        print(f"   Update ui_automation.py line 83: click_search_field({x}, {y})")
        break
else:
    print("\n❌ None of the coordinates worked.")
    print("   The search field might be in a different location.")
    print("   Try running: xdotool getmouselocation")
    print("   Then hover over the search field and note the coordinates.")
