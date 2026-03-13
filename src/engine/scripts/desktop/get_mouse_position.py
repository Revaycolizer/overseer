#!/usr/bin/env python3
import pyautogui
import time
import sys

print("Move your mouse over the UI elements you want to identify.")
print("Refreshes every 0.5 seconds. Press Ctrl+C to stop.")
print("-" * 50)

try:
    while True:
        x, y = pyautogui.position()
        position_str = 'X: ' + str(x).rjust(4) + ' Y: ' + str(y).rjust(4)
        print(position_str, end='')
        print('\b' * len(position_str), end='', flush=True)
        time.sleep(0.5)
except KeyboardInterrupt:
    print('\nDone.')
