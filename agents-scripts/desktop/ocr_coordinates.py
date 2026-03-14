import sys
import re
import xml.etree.ElementTree as ET

def find_all_coords(file_path, target_word):
    coords_list = []
    try:
        tree = ET.parse(file_path)
        root = tree.getroot()
        for elem in root.iter():
            if 'ocrx_word' in (elem.get('class') or ""):
                text = elem.text or ""
                if target_word.lower() in text.lower():
                    title = elem.get('title')
                    match = re.search(r'bbox (\d+) (\d+) (\d+) (\d+)', title)
                    if match:
                        x1, y1, x2, y2 = map(int, match.groups())
                        cx, cy = (x1 + x2) // 2, (y1 + y2) // 2
                        coords_list.append((cx, cy))
    except Exception as e:
        print(f"Error parsing HOCR: {e}")
    return coords_list

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: script.py <word>")
        sys.exit(1)
        
    coords = find_all_coords("hocr_results.hocr", sys.argv[1])
    for c in coords:
        print(f"{c[0]} {c[1]}")
