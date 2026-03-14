#!/usr/bin/env python3
import sys
import requests
import json
import time
from bs4 import BeautifulSoup

def search_ddg_stealth(query):
    # Use DuckDuckGo HTML version for scraping (lite/html)
    url = "https://html.duckduckgo.com/html/"
    
    # Randomize user agent a bit more or use a very standard one
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36",
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8",
        "Accept-Language": "en-US,en;q=0.9",
        "Origin": "https://duckduckgo.com",
        "Referer": "https://duckduckgo.com/",
        "DNT": "1",
        "Upgrade-Insecure-Requests": "1"
    }
    
    try:
        # Start a session to handle potential cookies
        session = requests.Session()
        session.headers.update(headers)
        
        # Add a tiny delay to look more human if called in rapid succession
        # time.sleep(0.5) 
        
        response = session.post(url, data={"q": query}, timeout=15)
        response.raise_for_status()
        
        if "ddg-l" in response.text or "too many requests" in response.text.lower():
            return {"error": "Rate limited by DuckDuckGo"}
            
        soup = BeautifulSoup(response.text, "html.parser")
        results = []
        
        # DDG Lite/HTML structure
        for item in soup.select(".result"):
            title_el = item.select_one(".result__a")
            snippet_el = item.select_one(".result__snippet")
            
            if title_el:
                title = title_el.get_text(strip=True)
                href = title_el.get("href")
                
                # Clean up DDG proxy links if present (though lite usually gives direct)
                if href.startswith("//duckduckgo.com/y.js"):
                    # This happens in some versions, but usually Lite gives real URLs
                    pass
                
                snippet = snippet_el.get_text(strip=True) if snippet_el else ""
                
                if title and href:
                    results.append({
                        "title": title,
                        "url": href,
                        "snippet": snippet
                    })
        
        return results[:7] # Return a few more for better context
    except Exception as e:
        return {"error": str(e)}

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(json.dumps({"error": "No query provided"}))
        sys.exit(1)
        
    query = " ".join(sys.argv[1:])
    results = search_ddg_stealth(query)
    
    # If error is a dict, wrap it for consistent output
    if isinstance(results, dict) and "error" in results:
        print(json.dumps(results))
    else:
        print(json.dumps(results, indent=2))
