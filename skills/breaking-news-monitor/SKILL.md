---
name: breaking-news-monitor
description: Automated breaking news monitoring system that searches for urgent events and generates voice alerts for critical news items
category: research
tags: ["news", "monitoring", "alerts", "urgency", "voice", "telegram"]
version: 1.0
---
autoload: true

# Breaking News Monitor

This skill provides an automated system for monitoring breaking news, identifying urgent events, and converting critical news items to voice messages for delivery via Telegram.

## Features

- **Multi-query search**: Searches across multiple breaking news queries
- **Urgency scoring**: Calculates urgency scores based on keywords and context
- **Source identification**: Extracts source names from URLs for better reporting
- **Voice alert generation**: Converts urgent news to voice messages
- **Cron integration**: Designed to work as a pre-run script for cron jobs
- **JSON output**: Provides structured data for further processing

## Usage

### As a Cron Job Pre-Run Script

```python
# In your cron job configuration:
"script": "breaking_news_monitor.py",
"prompt": "Review the breaking news monitor output and send voice alerts for urgent news items"
```

### Manual Execution

```python
from skills.breaking_news_monitor import BreakingNewsMonitor

monitor = BreakingNewsMonitor()
urgent_news = monitor.search_breaking_news()
if urgent_news:
    monitor.send_voice_alerts(urgent_news[:3])  # Top 3 urgent items
```

## Configuration

### Urgency Threshold
- **High priority**: Score 80-100 (critical emergencies)
- **Medium priority**: Score 50-79 (major incidents)
- **Low priority**: Score 30-49 (developing stories)
- **Below threshold**: Score < 30 (routine news)

### Search Queries
- "breaking news"
- "urgent news" 
- "latest news"
- "current events"
- "breaking stories"

### Keywords for Urgency Scoring

#### High-Priority Keywords (20 points in title, 10 points in description)
- emergency, urgent, breaking, critical, disaster, crisis
- attack, shooting, explosion, earthquake, hurricane, fire
- accident, fatal, death toll, casualties, evacuation
- lockdown, warning, alert, threat, danger

#### Medium-Priority Keywords (10 points in title, 5 points in description)
- major, significant, serious, investigation, arrest
- incident, event, update, developing, latest

#### Bonus Scoring
- Breaking news context: +15 points
- All-caps titles: +10 points
- Exclamation marks: +5 points

## Output Format

The script outputs:

1. **Text report**: Formatted breaking news summary
2. **Special markers**: For cron job processing
3. **JSON data**: Structured urgent news items

```text
=== BREAKING NEWS MONITOR REPORT ===
Generated: 2026-04-16 14:24:44
Total urgent items found: 2

#1 URGENT NEWS
Source: CNN
Title: Major Earthquake Strikes California
Urgency Score: 85/100
URL: https://cnn.com/earthquake-news

[URGENT_NEWS_DETECTED]
Count: 2
[URGENT_DATA]
[
  {
    "title": "Major Earthquake Strikes California",
    "source": "CNN",
    "urgency_score": 85,
    "url": "https://cnn.com/earthquake-news"
  }
]
```

## Voice Alert Integration

For sending voice alerts via Telegram:

```python
def send_voice_alerts(news_items):
    """Convert urgent news to voice messages and send via Telegram"""
    for item in news_items:
        # Create voice message from news item
        message_text = f"Breaking News: {item['title']}. Source: {item['source']}"
        
        # Convert to speech
        audio_path = text_to_speech(message_text)
        
        # Send via Telegram
        send_telegram_voice(audio_path)
```

## Error Handling

The script includes comprehensive error handling:
- Import fallbacks for web_search function
- Graceful handling of search failures
- Output formatting that prevents crashes
- Clear error reporting for debugging

## Best Practices

1. **Schedule**: Run every 15-30 minutes for real-time monitoring
2. **Delivery**: Send alerts only for high-urgency items (score >= 50)
3. **Rate limiting**: Limit to 3-5 alerts per hour to avoid spam
4. **Source verification**: Cross-reference multiple sources for major events
5. **Geographic relevance**: Filter by location if needed (future enhancement)

## Critical: Model Selection

**Always pin this cron job to a Claude model (e.g. claude-haiku-4-5), never GLM.**

GLM hallucinated 13 fake news stories when the script returned `NO_BREAKING`. It ignored the instruction to respond `[SILENT]` and fabricated urgency scores and sources instead.

## Cron Job Prompt Requirements

The prompt MUST:
1. Call `text_to_speech` for each story — the cron system can't call tools itself
2. Output a text summary as the final response (cron delivery is text-based)
3. Respond `[SILENT]` only when script returns `NO_BREAKING`

Working prompt:
```
Run the breaking news script. If there are breaking news results, use text_to_speech to convert a brief summary of each story to a voice message. After sending the voice messages, output a short text summary of the stories as your final response. If there are no breaking news results, respond with exactly [SILENT].
```

**Do NOT use**: "Do NOT output any text responses or summaries — only voice messages." This causes the agent to silence itself even when stories exist.

## Integration with Existing Systems

### Hermes Cron Jobs
```python
# Create a cron job for breaking news monitoring
cronjob.create(
    name="breaking_news_monitor",
    prompt="Review breaking news and send voice alerts for urgent items",
    schedule="every 15m",  # Every 15 minutes
    script="scripts/breaking_news_monitor.py",
    deliver="telegram",  # Send alerts to Telegram
    skills=["breaking-news-monitor"]
)
```

### Multi-Agent Workflows
Can be integrated with:
- Alert agents for incident response
- Notification systems
- Emergency management workflows
- Media monitoring dashboards

## Future Enhancements

1. **Geographic filtering**: Add location-based relevance
2. **Source credibility scoring**: Weight sources by reliability
3. **Historical analysis**: Track news patterns over time
4. **Multi-language support**: International news monitoring
5. **Image analysis**: Extract visual information from news sites
6. **Social media integration**: Monitor Twitter/X for breaking news

## Troubleshooting

### Common Issues

1. **Import errors**: Ensure web_search tool is available
2. **No results**: Check internet connectivity and search queries
3. **Low urgency scores**: Adjust keyword weights or threshold
4. **Missing sources**: Verify URL extraction logic
5. **Script location issues**: If direct execution fails, use subprocess:
   ```python
   import subprocess
   result = subprocess.run([sys.executable, 'scripts/breaking_news_monitor.py'], 
                         capture_output=True, text=True, cwd='/path/to/scripts')
   ```

### Debug Mode
Run with environment variable `DEBUG=1` for detailed output:
```bash
DEBUG=1 python3 breaking_news_monitor.py
```

### Script Execution Issues

If you encounter "No such file or directory" errors:
1. Verify script location in `/home/coemedia/.hermes/scripts/breaking_news_monitor.py`
2. If using cron jobs, ensure the working directory is set correctly
3. Use subprocess.run for more robust execution in complex environments

## Files

- **scripts/breaking_news_monitor.py**: Main monitoring script
- **SKILL.md**: This documentation file
- **examples/**: Sample configurations and outputs

## License

This skill is part of the Hermes agent system and follows the same license terms.