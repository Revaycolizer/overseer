#!/usr/bin/env python3
import sys
import requests
import json
from bs4 import BeautifulSoup

def search_ddg_stealth(query):
    # Use DuckDuckGo HTML version for scraping (lite/html)
    url = "https://html.duckduckgo.com/html/"
    headers = {
        "User-Agent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
    }
    
    try:
        response = requests.post(url, data={"q": query}, headers=headers, timeout=10)
        response.raise_for_status()
        
        soup = BeautifulSoup(response.text, "html.parser")
        results = []
        
        for item in soup.select(".result"):
            title_el = item.select_one(".result__a")
            snippet_el = item.select_one(".result__snippet")
            
            if title_el:
                title = title_el.get_text(strip=True)
                href = title_el.get("href")
                snippet = snippet_el.get_text(strip=True) if snippet_el else ""
                
                results.append({
                    "title": title,
                    "url": href,
                    "snippet": snippet
                })
        
        return results[:5]
    except Exception as e:
        return {"error": str(e)}

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(json.dumps({"error": "No query provided"}))
        sys.exit(1)
        
    query = " ".join(sys.argv[1:])
    results = search_ddg_stealth(query)
    print(json.dumps(results, indent=2))
