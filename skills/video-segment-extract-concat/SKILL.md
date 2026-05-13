---
title: Video Segment Extraction and Concatenation
description: Extract clips from a source video and stitch them together into a clean h264 MP4. Handles GNOME screencast VP8 MKV with weird resolutions and high frame rates.
domain: media
name: video-segment-extract-concat
---

# Video Segment Extraction + Concat

## Problem
GNOME screen recorder produces VP8 MKV at weird resolutions (2517x1293) with ~1000fps timebase. Standard `trim`+`concat` ffmpeg filter chains produce millions of duplicate frames. Stream copy segment cuts break VP8 frame references.

## Phase 1: Extract segments with re-encode + -r 30

```bash
ffmpeg -y -ss START -to END -i input.mkv \
  -vf "scale=trunc(iw/2)*2:trunc(ih/2)*2" \
  -r 30 -pix_fmt yuv420p \
  -c:v libx264 -preset fast -crf 23 \
  seg_N.mp4
```

- `-ss` before `-i` = fast container seek
- `-r 30` fixes source timebase
- `scale=trunc(iw/2)*2:*2` fixes odd dimensions
- Each segment independently encoded = no cross-segment reference issues

## Phase 2: Concat with -c copy (instant, ~130x speed)

```
// concat.txt
file 'seg_00.mp4'
file 'seg_01.mp4'
```

```bash
ffmpeg -f concat -safe 0 -i concat.txt -c copy output.mp4
```

## Verify
```bash
ffprobe -v error -select_streams v:0 -show_entries stream=r_frame_rate output.mp4
```

## Pitfalls
- Don't use `trim`+`concat` filter chain for VP8 — millions of dupes
- Don't use `-c copy` for VP8 segments before concat — frame refs break
- X/Twitter rejects >60fps
- Odd width needs `scale=trunc(iw/2)*2:*2` for libx264
