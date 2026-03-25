-- ═══════════════════════════════════════════════════════════════
-- Enable Image Extraction for Stealth Browse
-- ═══════════════════════════════════════════════════════════════

-- 1. Update stealth_browse tool code to extract images
UPDATE tools SET code = 'import sys
import json
import requests
from bs4 import BeautifulSoup

def get_page_info(url):
    try:
        headers = {''User-Agent'': ''Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36''}
        response = requests.get(url, headers=headers, timeout=30)
        response.raise_for_status()
        
        soup = BeautifulSoup(response.text, "html.parser")
        
        # 1. Extract Images
        images = []
        for img in soup.find_all("img"):
            src = img.get("src")
            alt = img.get("alt", "")
            if src and (src.startswith("http://") or src.startswith("https://")):
                # Filter out obvious small icons or generic trackers
                if len(src) < 512 and not any(x in src.lower() for x in ["pixel", "tracking", "icon", "logo", "spacer"]):
                    images.append(f"[IMAGE: {src} | {alt}]")
        
        # 2. Extract Text
        for script_or_style in soup(["script", "style", "header", "footer", "nav"]):
            script_or_style.decompose()
            
        text = soup.get_text(separator='' '')
        
        lines = (line.strip() for line in text.splitlines())
        chunks = (phrase.strip() for line in lines for phrase in line.split("  "))
        text = ''\n''.join(chunk for chunk in chunks if chunk)
        
        image_text = "\n".join(images[:15])
        full_output = f"--- VISUAL ASSETS ---\n{image_text}\n\n--- PAGE CONTENT ---\n{text}"
        
        return full_output[:18000] 
    except Exception as e:
        return f"Error fetching {url}: {str(e)}"

if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit(1)
        
    url = sys.argv[1]
    result = get_page_info(url)
    print(result)
' WHERE name = 'stealth_browse';

-- 2. Reinforce image extraction in supervisors
UPDATE supervisors SET instructions = instructions || '

═══ IMAGE DISCOVERY — CRITICAL ═══
When using browse/search tools, watch for [IMAGE: URL] markers in the results. 
If the user requested an image or diagram, you MUST include the most relevant [IMAGE: URL] in your response.
Place the URL in your "evidence" list as a direct string or in an object with "image_url".
Ensure the URL is high-quality and relevant to the diagram or topic requested.' 
WHERE domain IN ('chat', 'stealth', 'web', 'computer');
