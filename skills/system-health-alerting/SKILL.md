---
title: System Health Alerting via Telegram
name: system-health-alerting
category: devops
description: Send system health alerts to Jeremy via Telegram voice messages when system thresholds are exceeded
---

## Trigger Conditions
When system health checks detect warnings (disk >=90%, memory >=90%, services down, high load) and need to alert Jeremy via Telegram.

## Approach Overview
This skill provides a method to send system health alerts through existing Telegram infrastructure when dedicated alert systems aren't available. It adapts existing alerting patterns to new use cases.

## Step-by-Step Process

### 1. Check for Existing Alert Infrastructure
```bash
# First check if existing alert tools are available
which telegram-send || which slack-send || find /path/to/scripts -name "*alert*" -o -name "*notify*"
```

### 2. Examine Existing Telegram Scripts
```bash
# Look for existing Telegram-based alert scripts
ls -la /home/coemedia/.hermes/scripts/ | grep -E "(telegram|digest|alert)"
# Common scripts: telegram_digest.py, breaking_news.py, weather_alert.py
```

### 3. Check Configuration for Jeremy's User ID
```bash
# Load existing config to get Jeremy's Telegram user ID
cat /home/coemedia/.hermes/scripts/breaking_news_config.json
```

### 4. Create Alert Script (if needed)
If no existing alert infrastructure is found, create a new script following existing patterns:

```python
#!/home/coemedia/.hermes/hermes-agent/venv/bin/python3
"""System Health Alert - sends alerts to Jeremy when system issues are detected."""
import os
import sys
import json
import asyncio
import tempfile
from datetime import datetime
from telethon import TelegramClient
from gtts import gTTS
import logging

# API credentials
API_ID = 36081768
API_HASH = "687286d3b0f9dc714058a4870bfc99b0"
SESSION_FILE = os.path.expanduser("~/.hermes/scripts/tg_session")

# Load Jeremy's user ID from breaking news config
def load_config():
    config_file = os.path.expanduser("~/.hermes/scripts/breaking_news_config.json")
    try:
        with open(config_file, "r") as f:
            config = json.load(f)
            return config.get("jeremy_user_id")
    except:
        return None

JEREMY_USER_ID = load_config()

# Suppress logging to avoid stdout/stderr leakage
logging.basicConfig(level=logging.CRITICAL)

def text_to_speech(text, lang='en'):
    """Convert text to speech using gTTS and return path to audio file."""
    try:
        # Create temporary file for audio
        with tempfile.NamedTemporaryFile(suffix=".mp3", delete=False) as temp_file:
            tts = gTTS(text=text, lang=lang, slow=False)
            tts.save(temp_file.name)
            return temp_file.name
    except Exception as e:
        print(f"Error converting text to speech: {e}", file=sys.stderr)
        return None

async def send_voice_message(client, audio_path, user_id, caption=None):
    """Send voice message to a user."""
    try:
        await client.send_file(
            user_id,
            audio_path,
            voice_note=True,
            caption=caption
        )
        return True
    except Exception as e:
        print(f"Error sending voice message: {e}", file=sys.stderr)
        return False

async def main():
    """Main function to send system health alerts."""
    if not JEREMY_USER_ID:
        print("ERROR: Jeremy's user ID not configured in breaking_news_config.json")
        return
    
    client = TelegramClient(SESSION_FILE, API_ID, API_HASH)
    
    try:
        await client.connect()
        
        if not await client.is_user_authorized():
            print("ERROR: Not authorized. Run telegram_digest.py --auth first.")
            await client.disconnect()
            return
        
        # Create alert message based on system health check results
        alert_time = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        # Format: "System Health Alert - {time}: {alert details}"
        
        # Convert to speech
        audio_path = text_to_speech(alert_message)
        if audio_path:
            # Send voice message to Jeremy
            success = await send_voice_message(
                client, 
                audio_path, 
                JEREMY_USER_ID,
                "🚨 System Alert: {alert_type}"
            )
            
            # Clean up temporary file
            try:
                os.unlink(audio_path)
            except:
                pass
            
            if success:
                print(f"[ALERT_SENT] System health alert sent to Jeremy")
            else:
                print(f"[ALERT_FAILED] Failed to send system health alert")
        else:
            print(f"[ALERT_FAILED] Failed to convert alert to speech")
        
        await client.disconnect()
        
    except Exception as e:
        print(f"[ALERT_ERROR] {str(e)}")

if __name__ == "__main__":
    asyncio.run(main())
```

### 5. Handle Environment Dependencies
```bash
# Install required dependencies in the correct environment
source /home/coemedia/.hermes/hermes-agent/venv/bin/activate
pip install gTTS

# Test the script
python3 /path/to/system_alert.py
```

### 6. Integrate with Health Check Script
Modify the system health check to call the alert script when warnings are detected:

```python
# In system_health_check.py or cron job:
if "WARNING:" in output:
    # Send alert via Telegram
    subprocess.run([sys.executable, "/home/coemedia/.hermes/scripts/system_alert.py"])
```

## Key Patterns and Conventions

### Existing Script Patterns to Follow:
- **Authentication**: Use existing `tg_session` file and API credentials
- **User ID**: Load from `breaking_news_config.json`
- **Error handling**: Suppress logging, clean up temporary files
- **Voice messages**: Use gTTS for text-to-speech conversion
- **Telegram client**: Follow existing async patterns from breaking_news.py

### Common Pitfalls to Avoid:
- **Environment issues**: Always use the virtual environment path
- **Missing dependencies**: Install gTTS in the correct environment
- **Authentication**: Ensure Telegram session is properly authenticated
- **Error handling**: Don't leak internal errors in stdout

## Practical Implementation Lessons

### From Real-World Experience:
1. **Start with text alerts first**: Voice alerts (gTTS) require additional dependencies and may fail due to environment issues. Text alerts are more reliable and sufficient for most use cases.

2. **Test authentication first**: Always run `telegram_digest.py --auth` before using any Telegram scripts to establish the session file.

3. **Handle dependency gracefully**: If gTTS fails, fall back to text alerts rather than failing completely.

4. **Environment matters**: Use `#!/usr/bin/env python3` instead of hardcoded venv paths for better portability.

5. **User ID validation**: Verify the user ID from config is valid before attempting to send messages.

### Updated Alert Script Pattern:
```python
#!/usr/bin/env python3
"""System Health Alert - sends alerts to Jeremy when system issues are detected."""
import os
import sys
import json
import asyncio
from datetime import datetime
from telethon import TelegramClient

# API credentials
API_ID = 36081768
API_HASH = "687286d3b0f9dc714058a4870bfc99b0"
SESSION_FILE = os.path.expanduser("~/.hermes/scripts/tg_session")

def load_config():
    """Load Jeremy's user ID from breaking news config."""
    config_file = os.path.expanduser("~/.hermes/scripts/breaking_news_config.json")
    try:
        with open(config_file, "r") as f:
            config = json.load(f)
            return config.get("jeremy_user_id")
    except Exception as e:
        print(f"Config load error: {e}", file=sys.stderr)
        return None

JEREMY_USER_ID = load_config()

async def send_text_message(client, user_id, message):
    """Send text message to a user."""
    try:
        await client.send_message(user_id, message)
        return True
    except Exception as e:
        print(f"Error sending text message: {e}", file=sys.stderr)
        return False

async def main():
    """Main function to send system health alerts."""
    if not JEREMY_USER_ID:
        print("ERROR: Jeremy's user ID not configured", file=sys.stderr)
        return
    
    alert_time = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    alert_message = f"🚨 System Health Alert - {alert_time}\n\nWARNING: Disk space critically low\n\n• /dev/sdb2: 84.8% used (23.6GB free)\n• /dev/sdd1: 89.8% used (760.2GB free)\n\nPlease check and clean up disk space immediately."
    
    client = TelegramClient(SESSION_FILE, API_ID, API_HASH)
    
    try:
        await client.connect()
        
        if not await client.is_user_authorized():
            print("ERROR: Not authorized. Run telegram_digest.py --auth first.", file=sys.stderr)
            await client.disconnect()
            return
        
        success = await send_text_message(client, JEREMY_USER_ID, alert_message)
        
        if success:
            print("[ALERT_SENT] System health alert sent to Jeremy")
        else:
            print("[ALERT_FAILED] Failed to send system health alert")
        
        await client.disconnect()
        
    except Exception as e:
        print(f"[ALERT_ERROR] {str(e)}", file=sys.stderr)

if __name__ == "__main__":
    asyncio.run(main())
```

### Environment Setup:
```bash
# Install dependencies (only if needed)
source /home/coemedia/.hermes/hermes-agent/venv/bin/activate
pip install telethon --user

# Authenticate first (one-time setup)
python3 /home/coemedia/.hermes/scripts/telegram_digest.py --auth

# Test the alert script
python3 /home/coemedia/.hermes/scripts/system_alert_text.py
```

## Error Handling and Fallbacks

1. **Missing user ID**: Check config file and handle gracefully
2. **Authentication failures**: Verify session file exists and run auth if needed
3. **gTTS failures**: Handle conversion errors gracefully
4. **Network issues**: Handle Telegram connection errors

## Integration Examples

### For cron jobs:
```json
{
  "name": "system-health-monitor",
  "prompt": "You are running a system health check. Execute these steps: 1) Run python3 /home/coemedia/.hermes/scripts/system_health_check.py 2) If warnings detected, send alert via system_alert.py 3) If healthy, output nothing silently",
  "schedule": "30m"
}
```

### For custom health checks:
```bash
# Custom script pattern
python3 /home/coemedia/.hermes/scripts/system_health_check.py | grep "WARNING:" && \
python3 /home/coemedia/.hermes/scripts/system_alert.py
```

## Related Skills
- `system-health-check`: System monitoring and threshold detection
- `telegram-alerting`: General Telegram alert patterns