#!/usr/bin/env python3
"""
Breaking News Monitor Script

This script searches for breaking news and identifies urgent events that need
immediate attention. It's designed to be used as a cron job pre-run script
to collect breaking news data before the main agent processes it.
"""

import json
import re
import sys
from datetime import datetime
from typing import Dict, List, Any

def extract_source_from_url(url: str) -> str:
    """Extract source name from URL"""
    if not url:
        return "Unknown"
    
    # Remove protocol and www.
    source = url.replace("https://", "").replace("http://", "").replace("www.", "")
    
    # Get domain up to first slash
    source = source.split("/")[0]
    
    # Remove common TLDs
    source = re.sub(r'\.(com|org|net|gov|edu|io|co)\b', '', source)
    
    return source

def calculate_urgency_score(title: str, description: str, query: str) -> int:
    """
    Calculate urgency score based on keywords and context.
    
    Returns:
        Score from 0-100, higher means more urgent
    """
    score = 0
    
    # Convert to lowercase for case-insensitive matching
    title_lower = title.lower()
    desc_lower = description.lower()
    
    # High-priority keywords
    high_priority_keywords = [
        "emergency", "urgent", "breaking", "critical", "disaster", "crisis", 
        "attack", "shooting", "explosion", "earthquake", "hurricane", "fire",
        "accident", "fatal", "death toll", "casualties", "evacuation",
        "lockdown", "warning", "alert", "threat", "danger"
    ]
    
    # Medium-priority keywords
    medium_priority_keywords = [
        "major", "significant", "serious", "investigation", "arrest",
        "incident", "event", "update", "developing", "latest"
    ]
    
    # Score based on high-priority keywords
    for keyword in high_priority_keywords:
        if keyword in title_lower:
            score += 20
        if keyword in desc_lower:
            score += 10
    
    # Score based on medium-priority keywords
    for keyword in medium_priority_keywords:
        if keyword in title_lower:
            score += 10
        if keyword in desc_lower:
            score += 5
    
    # Bonus for breaking news context
    if "breaking" in query.lower() or "urgent" in query.lower():
        score += 15
    
    # Bonus if title is all caps (often used for breaking news)
    if title.isupper() and len(title) > 5:
        score += 10
    
    # Bonus if title contains exclamation marks
    if "!" in title:
        score += 5
    
    # Cap the score at 100
    return min(score, 100)

def search_breaking_news() -> List[Dict[str, Any]]:
    """
    Search for breaking news using multiple search queries.
    
    Returns:
        List of news articles with title, source, url, and timestamp
    """
    # web_search is available as a function in this environment
    
    # Search queries for breaking news
    search_queries = [
        "breaking news",
        "urgent news",
        "latest news",
        "current events",
        "breaking stories"
    ]
    
    all_results = []
    
    for query in search_queries:
        try:
            print(f"Searching for: {query}")
            search_result = web_search(query=query)
            
            if search_result and search_result.get("data") and search_result["data"].get("web"):
                for result in search_result["data"]["web"][:3]:  # Limit to 3 results per query
                    # Add timestamp and relevance score
                    article = {
                        "title": result.get("title", ""),
                        "url": result.get("url", ""),
                        "description": result.get("description", ""),
                        "source": extract_source_from_url(result.get("url", "")),
                        "query": query,
                        "timestamp": datetime.now().isoformat(),
                        "urgency_score": calculate_urgency_score(result.get("title", ""), result.get("description", ""), query)
                    }
                    all_results.append(article)
                    
        except Exception as e:
            print(f"Error searching for '{query}': {e}", file=sys.stderr)
            continue
    
    # Sort by urgency score (highest first)
    all_results.sort(key=lambda x: x.get("urgency_score", 0), reverse=True)
    
    return all_results

def main():
    """Main function to run the breaking news monitor."""
    try:
        # Search for breaking news
        print("Starting breaking news monitor...")
        all_news = search_breaking_news()
        
        # Filter for urgent news only (score >= 30)
        urgent_news = [item for item in all_news if item.get("urgency_score", 0) >= 30]
        
        # Format output
        output = []
        output.append(f"=== BREAKING NEWS MONITOR REPORT ===")
        output.append(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        output.append(f"Total urgent items found: {len(urgent_news)}")
        output.append("")
        
        for i, item in enumerate(urgent_news[:3], 1):  # Show top 3 urgent items
            output.append(f"#{i} URGENT NEWS")
            output.append(f"Source: {item.get('source', 'Unknown')}")
            output.append(f"Title: {item.get('title', 'No title')}")
            output.append(f"Urgency Score: {item.get('urgency_score', 0)}/100")
            output.append(f"URL: {item.get('url', 'No URL')}")
            output.append("")
        
        output_text = "\n".join(output)
        
        # Print output (will be captured by cron system)
        print(output_text)
        
        # If there's urgent news, return special marker to trigger voice alerts
        if urgent_news:
            print("\n[URGENT_NEWS_DETECTED]")
            print(f"Count: {len(urgent_news)}")
            # Return JSON data for further processing
            urgent_data = json.dumps(urgent_news[:3], indent=2)  # Top 3 urgent items
            print(f"[URGENT_DATA]\n{urgent_data}")
        
        return 0
        
    except Exception as e:
        print(f"Error in breaking news monitor: {e}", file=sys.stderr)
        return 1

if __name__ == "__main__":
    sys.exit(main())