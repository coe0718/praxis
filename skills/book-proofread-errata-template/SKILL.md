---
name: book-proofread-errata-template
description: Structured errata tracking template for fantasy novel continuity edits
trigger: Organizing proofread notes into trackable issues with impact levels and assignments
---

# Book Proofread Errata Template

Use this template to track continuity and logic errors discovered during proofreading. Prioritize **Critical** issues that break plot logic, character motivation, or power progression.

## Issue Template

```markdown
### [🔴/🟡/🟢] ISSUE TITLE — Chapter X
**Type:** Vote Count / Skill Acquisition / Tracking / Duplicate / Timeline / Material
**Current state:** What the text currently says/does
**Expected state:** What it should say/do
**Impact:** Critical (breaks logic/character motivation) | Medium (confusing but not breaking) | Low (typo/style)
**Chapter/Location:** Specific chapter and scene/line if known
**Assigned to:** Kai / Vailnx / Malik / etc.
**Status:** 🔄 In Progress | ✅ Complete | ⏳ Pending
**Notes:**任何额外上下文或修复说明
```

## Critical Path-Breaking Issues (Seed List)

1. **Vote Count Contradiction** — Chapter 3
   - Current: Marcus has 3 votes, Alastor gets sacrificed
   - Expected: Alastor 4 votes (Marcus + Tanya + Kevin + self), others 1 each
   - Impact: Critical — changes self-sacrifice to math error
   - Assigned: Kai (writing), Malik (approval)
   - Status: 🔄 In Progress

2. **Zombie→Skeleton Transition** — Chapter 6
   - Current: 10 zombies → 15 zombies + 15 skeletons by Round Ten; only zombie skill exists
   - Expected: Skeleton summon granted by Vailnx before Round 4
   - Impact: Critical — power progression breaks
   - Assigned: Kai (writing)
   - Status: 🔄 In Progress

3. **Skeleton Raising Not Shown** — Chapter 8
   - Current: 2 skeletons at floor 230, no skeleton-raising mentioned
   - Expected: Line added about skill acquisition around floor 260
   - Impact: Medium — skill acquisition gap
   - Assigned: Kai (writing)
   - Status: 🔄 In Progress

4. **Soul Reserve Gap** — Chapters 7→8→9
   - Current: Ch7 ends with 4 souls; Ch8 (floors 250→230) no tracking; Ch9 shows 12 souls; +283→+293 stat gap unexplained
   - Expected: Add soul reserve display at floor 250, explain straggler clear
   - Impact: Critical — soul economics broken
   - Assigned: Kai (writing)
   - Status: 🔄 In Progress

5. **Duplicate System Notification** — Chapter 9
   - Current: "Remaining soul reserve: 4" appears twice in same block
   - Expected: Delete duplicate
   - Impact: Low — immersion break
   - Assigned: Kai (editing)
   - Status: 🔄 In Progress

## Categorization

- **Vote Count:** Any Council sacrifice scene vote tallies
- **Skill Acquisition:** New abilities appearing without proper acquisition scene
- **Tracking:** Soul/stat/resource counts that drift or have gaps
- **Duplicate:** Repeated lines or notifications
- **Timeline:** Age/date inconsistencies (Theo's birth month vs "X years later")
- **Materials:** Naming inconsistencies (dragonsteel vs dragon bone ivory)

## Source

Proofread notes: `~/Documents/books/Tuck/proofread-notes.md`
Book: "Resurrection of the Last Necromancer" by Kai Voss