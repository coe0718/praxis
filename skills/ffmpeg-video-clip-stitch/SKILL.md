---
name: ffmpeg-video-clip-stitch
description: Extract segments from a video and stitch them together into a single file. Handles the VP8-from-GNOME-screen-recorder case correctly.
---

# FFmpeg Video Clip & Stitch

Extract multiple non-contiguous segments from a source video and join them into a single output file.

## The Right Approach

**DO NOT** use `trim`+`concat` filter chain — this builds a virtual chain of clips that recalculates PTS through the concat filter, producing massive frame duplication (99% skip rate at 0.18x speed).

**DO NOT** use `-c copy` for individual segment extraction when:
- The source codec is VP8 (GNOME screen recorder default)
- The source has no CUE index (common for MKV screen recordings)

Stream-copied VP8 segments contain P-frames referencing frames outside the segment. When concatenated, these produce massive duplicate-frame chains (99% skip rate).

**Note on `select` filter:** Works correctly for frame-level extraction (`select='between(t,start1,end1)+...' -vsync vfr`) but requires decoding ALL frames of the source video, making it slow for long recordings. Prefer the extract+re-encode approach below for performance.

### Working Method: Extract + Re-encode + Concat

**CRITICAL:** Add `-r 30` to the extraction step. GNOME screen recordings use a 1ms timebase (1000 tbn), so without `-r`, each segment encodes at ~1000fps. Trying to fix this post-hoc by re-encoding a 998fps file takes forever (timed out after 9+ minutes for 2.5 min of video). Baking it into extraction is the only viable path.

```bash
INPUT="input.mkv"
SCALE="scale=trunc(iw/2)*2:trunc(ih/2)*2"

# Step 1: Extract each segment with re-encode to h264 at 30fps
# -ss before -i is container-level fast seek
# -r 30 is MANDATORY — GNOME screen recorder uses 1000fps timebase
# Each segment becomes a self-contained h264 file with proper framerate
ffmpeg -y -ss START -to END -i "$INPUT" -vf "$SCALE" -r 30 -pix_fmt yuv420p -c:v libx264 -preset fast -crf 23 seg_N.mp4

# Step 2: Create concat demuxer file
printf "file '%s'\\n" seg_*.mp4 > concat.txt

# Step 3: Stitch with stream copy (instant since all are same codec)
ffmpeg -f concat -safe 0 -i concat.txt -c copy output.mp4
```

## Platform Upload Compatibility

GNOME screen recordings use a 1ms timebase (1000 tbn), producing files with ~1000fps frame timing. Social platforms reject this:
- **X/Twitter**: max 60fps (rejects with "frame rate too high")
- Most video hosting platforms expect 24-60fps

If you forgot `-r` during extraction (don't), you'll need to rebuild from scratch. Attempting to re-encode a 998fps file post-hoc is impractically slow — the re-encode has to process 147k+ frames even for a 2.5-minute video.

## Verification

After concat, always verify the output before uploading:

```bash
ffprobe -v error -select_streams v:0 -show_entries stream=r_frame_rate,avg_frame_rate,duration -of default=noprint_wrappers=1 output.mp4
```

Confirm `r_frame_rate=30/1` (or your target) — social platforms reject anything over 60fps.

For a sanity check: 2:28 of video at 30fps = ~4440 frames. If the frame count is wildly different (e.g. 147k), the framerate is wrong.

## Pitfalls

- **VP8 stream copy segments cannot be concatenated.** Always re-encode VP8 segments to h264 individually first.
- **GNOME screen recordings produce VP8 without CUEs.** The `-ss` seek is fast but not instant — it scans MKV packet headers linearly.
- **Odd video dimensions** (e.g. 2517x1293) cause `libx264` to fail with "width not divisible by 2". Always add `scale=trunc(iw/2)*2:trunc(ih/2)*2` to the filter chain when encoding to h264.
- **Never pipe ffmpeg through `tail -3` in background processes.** `2>&1 | tail -3` buffers all output until ffmpeg finishes, making mid-progress invisible. Use `grep` for specific markers or let output flow freely.
- **Post-hoc framerate fixes don't work.** Re-encoding a 998fps file to 30fps requires decoding all 147k+ frames — took 9+ minutes and still didn't finish for a 2.5-minute video. Always bake `-r` into extraction.
- **No audio.** GNOME screen recorder on this system produces video-only MKV. If audio is needed, it must be captured separately.
- **Deleted working directory breaks the terminal session.** If the terminal's cwd was a directory you delete, all subsequent commands fail with `FileNotFoundError` even with absolute paths. Use `workdir` parameter or spawn a new shell.
