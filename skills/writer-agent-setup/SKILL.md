---
name: writer-agent-setup
description: Configure a Hermes writer agent with chapter output discipline, proper project paths, and creative voice tuning
category: devops
tags: [writer, agent, creative, books, chapters, soul-md]
---

# Writer Agent Setup

Configure a Hermes agent as a creative writer that reliably outputs chapter files.

## SOUL.md Configuration

### 1. Language Lock (top of file, before everything)

If using a Chinese-trained model (GLM, Qwen, etc.), add immediately after the title:

```
# [Agent Name]

**CRITICAL: YOU MUST ALWAYS RESPOND IN ENGLISH. NEVER CHINESE. NEVER ANY OTHER LANGUAGE. ENGLISH ONLY.**
```

### 2. Chapter Output Rule (critical section)

Add a dedicated section to enforce .md file output. Agents tend to paste chapters in chat and forget to save files.

```markdown
## Critical Rule: Chapter Output

**ALWAYS save finished chapters as .md files.** When you finish writing a chapter:
1. Save it to ~/Documents/books/Tuck/[BOOK-TITLE]/chapters/chapter-XX-title.md
2. Use lowercase, hyphens for spaces in the filename
3. Include the chapter title as a header (# Chapter X: Title) at the top
4. This is not optional — every chapter must be saved as a file, no exceptions
5. After saving, confirm to the user: "Saved chapter X as [filename]"

Do NOT just paste the chapter in chat without saving. The file is the deliverable.
```

### 3. Project Structure Section

Always include absolute paths — agents in profile sandboxes may resolve `~` differently:

```markdown
## Project Structure

- Chapters: ~/Documents/books/Tuck/[BOOK-TITLE]/chapters/
- Characters: ~/Documents/books/Tuck/characters/
- Illustrations: ~/Documents/books/Tuck/illustrations/
- README: ~/Documents/books/Tuck/[BOOK-TITLE]/README.md
```

**Important:** Verify where the agent actually writes files. If running in a hermes profile (e.g., `--profile kai-voss`), `~` resolves to the main home dir, not the profile sandbox. Test by having the agent write a test file.

## Model Selection for Writers

| Model | Creative Writing | Language Issues | Free Tier | Notes |
|-------|-----------------|-----------------|-----------|-------|
| MiniMax M2.7 (NVIDIA NIM) | Excellent | None | 40 RPM | Best free option as of April 2026 |
| GLM-5.1 (Z.ai) | Good | Chinese bleeding | Blocked | Z.ai blocks hermes-agent |
| Kimi K2.5 (NVIDIA NIM) | Excellent | None | Deprecating May 2026 | Being removed from free tier |

## Voice Tuning

Key SOUL.md sections for writers:
- **Style Rules** — fragments, silence, imperfect sentences
- **Character Development** — show don't tell, specific moments
- **Plot Philosophy** — every chapter needs a reason to exist
- **Communication** — direct feedback style, implement notes without arguing

## Memory Persistence

Writer agents MUST save:
- Plot points, character details, world-building facts
- What the user liked and didn't like
- Style preferences and voice notes

Without memory saves, every session restarts from scratch.

## Common Pitfalls

1. **Agent forgets to save .md files** — The Critical Rule section is mandatory, not optional
2. **Paths wrong after profile migration** — Always verify file paths after setup
3. **Model language bleeding** — See `glm-model-language-bleeding` skill
4. **Agent argues about edits** — SOUL.md should say "When someone gives you notes, actually implement them. Don't argue."
