import subprocess
import time
import os

def check_capture():
    print("🔍 Searching for 'tao' window...")
    try:
        # Find window ID
        wid_bytes = subprocess.check_output(['xdotool', 'search', '--name', 'tao'])
        wids = wid_bytes.decode().strip().split('\n')
        
        if not wids:
            print("❌ No 'tao' window found.")
            return
            
        wid = wids[-1] # Take last one (usually the active one)
        print(f"✅ Found Window ID: {wid}")
        
        # Get Geometry
        geo = subprocess.check_output(['xdotool', 'getwindowgeometry', wid]).decode()
        print(f"📏 Geometry:\n{geo}")

        # Capture SPECIFIC window
        output_file = "neontune_window.png"
        print(f"📸 Capturing Window {wid} to {output_file}...")
        subprocess.run(['import', '-window', wid, output_file])
        
        # OCR
        print("📝 Running OCR...")
        subprocess.run(['tesseract', output_file, 'neontune_ocr', 'hocr'])
        subprocess.run(['cat', 'neontune_ocr.hocr']) # Dump raw for quick peek

    except subprocess.CalledProcessError as e:
        print(f"❌ Commands failed: {e}")
    except Exception as e:
        print(f"❌ Error: {e}")

if __name__ == "__main__":
    check_capture()
