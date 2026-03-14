import sys
import re
import xml.etree.ElementTree as ET

def dump_layout(file_path):
    try:
        tree = ET.parse(file_path)
        root = tree.getroot()
        for elem in root.iter():
            if 'ocrx_word' in (elem.get('class') or ""):
                text = elem.text or ""
                if text.strip():
                    title = elem.get('title')
                    match = re.search(r'bbox (\d+) (\d+) (\d+) (\d+)', title)
                    if match:
                        x1, y1, x2, y2 = map(int, match.groups())
                        print(f"'{text}' : {x1},{y1} {x2},{y2}")
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    dump_layout("hocr_dump.hocr")
