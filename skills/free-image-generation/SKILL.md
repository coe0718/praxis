---
name: free-image-generation
description: Generate images for free without API keys using Pollinations.ai. Fallback when NVIDIA FLUX API or paid services aren't available.
category: creative
---

# Free Image Generation (Pollinations.ai)

Generate images for free using Pollinations.ai — no API key, no signup, no rate limits (reasonable use).

## When to Use
- User wants illustrations, character art, concept art
- No image generation API key is configured (FAL_KEY, Stability, etc.)
- NVIDIA NIM FLUX models aren't available (they're download-only, NOT in the free API tier)
- Quick prototyping before committing to a paid service

## NVIDIA NIM Image Gen — DOESN'T WORK (Learned)
- FLUX models on build.nvidia.com are **downloadable only** — not available through the free API
- The free NVIDIA API key (`nvapi-*`) only covers text/chat models
- Tested: `https://integrate.api.nvidia.com/v1/images/generations` → 404 for all FLUX/SD models
- The `/v1/models` endpoint returns 133 models — zero image generation models

## Pollinations.ai — WORKS

### URL Format
```
https://image.pollinations.ai/prompt/{URL-encoded-prompt}?width=768&height=1024&nologo=true&seed=42
```

### Python Example
```python
import requests
import urllib.parse

prompt = "anime portrait, dark fantasy necromancer boy, detailed, dramatic lighting"
encoded = urllib.parse.quote(prompt)
url = f"https://image.pollinations.ai/prompt/{encoded}?width=768&height=1024&nologo=true"

response = requests.get(url, timeout=120)
with open("output.png", "wb") as f:
    f.write(response.content)
```

### Parameters
- `width` / `height`: Image dimensions (768x1024 for portraits, 1024x768 for landscapes)
- `nologo=true`: Remove watermark
- `seed`: Reproducible results
- `model`: Can specify model (default is FLUX)

### Tips
- Keep prompts concise — very long prompts may fail
- Add style keywords: "anime style", "shonen", "dark fantasy", "detailed character design"
- Response is JPEG despite .png extension — rename or convert if needed (see PNG conversion below)
- Timeout of 120s recommended — generation can be slow
- Works from any environment with internet access (no auth needed)

### ⚠️ Gotchas (learned 04/21)

**User-Agent header required:** `urllib.request.urlretrieve()` gets 403 Forbidden. Must use a Request with headers:
```python
req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
with urllib.request.urlopen(req, timeout=120) as resp:
    data = resp.read()
```

**Aggressive rate limiting:** Pollinations.ai returns 429 Too Many Requests quickly. Need 10-15 second delays between requests. Generating 9 avatars in sequence took ~25 minutes with 15s delays.

**File format is JPEG, not PNG:** Despite the `.png` extension, Pollinations returns JPEG data. Convert with Pillow if you need actual PNG:
```python
from PIL import Image
img = Image.open(filepath)
if img.mode in ('RGBA', 'P'):
    img = img.convert('RGB')
img.save(filepath, 'PNG')
```

**Batch strategy for multiple images:**
1. Generate 2-3 at a time (not all at once)
2. 15 second delay between requests
3. If you get 429, wait 60 seconds and retry
4. Convert all to proper PNG after downloading

## Iterative Character Design Workflow

When a user wants character illustrations refined through feedback:

1. Ask for key physical details first (hair, eyes, clothing, accessories, weapons)
2. Generate initial portrait with full description
3. Get user feedback — they WILL find things wrong
4. Regenerate with corrected details — change seed for fresh variation
5. Focus the prompt on what needs to be right (if dagger is wrong, lead with dagger details)
6. Repeat until "locked in" — user explicitly approves
7. Save each version: `character-portrait.png`, `character-portrait-v2.png`, etc.

### Prompt Tips from Experience
- For specific artifacts (weapons, armor), describe EACH COMPONENT separately: blade material + color, guard material + color, handle material
- If an element keeps coming out wrong, make it the FOCUS by putting "focus on [item]" or "close up of [item]" in the prompt
- Art style must be explicit — "shonen anime style" or "dark fantasy" — or the model defaults to photorealistic
- Background/setting matters — "in ancient colosseum" or "dim green necromantic light" sets mood

## Alternatives (if Pollinations is down)
- Bing Image Creator (free, Microsoft account)
- Leonardo.ai (free tier, better anime models)
- Civitai.com (community models, some free)
