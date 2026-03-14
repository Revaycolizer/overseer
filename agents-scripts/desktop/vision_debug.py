import urllib.request
import json
import base64
import sys

def analyze_image(image_path, prompt):
    try:
        with open(image_path, "rb") as f:
            image_base64 = base64.b64encode(f.read()).decode("utf-8")
    except FileNotFoundError:
        return "Error: File not found"

    url = "http://localhost:11434/api/generate"
    data = {
        "model": "qwen2.5vl:3b",
        "prompt": prompt,
        "images": [image_base64],
        "stream": False, 
        "format": "json"
    }

    try:
        req = urllib.request.Request(url, json.dumps(data).encode("utf-8"), headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(req) as response:
            return json.loads(response.read().decode("utf-8"))["response"]
    except Exception as e:
        return f"Request failed: {e}"

print("Analyzing Search Field...")
# Ask for specific point format
print(analyze_image("screenshot.png", "Return a JSON object with a single key 'point' containing [x, y] coordinates for the center of the search input field in the application window."))

print("\nAnalyzing First Result...")
print(analyze_image("screenshot.png", "Return a JSON object with a single key 'point' containing [x, y] coordinates for the center of the first song row in the list below the search bar."))
