# memU (NevaMind-AI) — Feature Analysis vs Praxis

> **Source**: https://github.com/NevaMind-AI/memU (v1.5.1)
> **Note**: Original URL `https://github.com/memU-ai/memu` now redirects/404s; actual repo is `NevaMind-AI/memU`.
> **Cloned to**: `/tmp/memU` (since deleted)

## Architecture Overview

memU is a **memory framework for 24/7 proactive agents**, designed around a "memory as file system" metaphor with three persistent layers:

- **Resource** — raw source artifacts (conversation, document, image, video, audio)
- **MemoryItem** — extracted atomic memories with embeddings
- **MemoryCategory** — grouped topic summaries with auto-generated LLM summaries

It's built as a Python package with a tiny Rust core (via PyO3/maturin — currently just a hello-world stub). The `MemoryService` class is the composition root, with mixin-based APIs (`MemorizeMixin`, `RetrieveMixin`, `CRUDMixin`) and a workflow pipeline engine.

---

## Features Praxis Likely Already Has

- Memory ingestion from conversations/text (core to both systems)
- Embedding-based vector search (cosine similarity, brute-force)
- SQLite and Postgres backends
- Pydantic-based config models
- LangGraph integration (tool adapter pattern)
- OpenAI-compatible LLM clients
- CRUD operations for memory items/categories/resources
- Multi-user scoping via `user_id` field

---

## Features Praxis Might NOT Have

### 1. Workflow Pipeline Engine with Runtime Mutation
**File**: `src/memu/workflow/pipeline.py`, `src/memu/workflow/step.py`

memU has a **declarative workflow pipeline engine** where every major operation (memorize, retrieve, CRUD) is composed of named `WorkflowStep` objects with:
- Explicit `requires`/`produces` state key contracts
- Declared `capabilities` tags (`llm`, `vector`, `db`, `io`, `vision`)
- Per-step `config` for LLM profile routing

Critically, pipelines can be **mutated at runtime**:
- `insert_step_before/after(target_step_id, new_step)`
- `replace_step(target_step_id, new_step)`
- `remove_step(target_step_id)`
- `configure_step(target_step_id, config_dict)`

This enables **runtime pipeline customization** — users can inject custom processing steps, replace LLM extraction with custom logic, or remove stages without forking the codebase.

**Relevance**: If Praxis's memory pipeline is monolithic or non-extensible at runtime, this is a significant architectural differentiator.

---

### 2. Salience-Aware Memory Retrieval with Reinforcement Tracking
**Files**: `src/memu/database/inmemory/vector.py` (lines 16–53, 94–127), `src/memu/app/settings.py` (`RetrieveItemConfig`)

memU supports **salience scoring** as an alternative to pure cosine similarity for item ranking:

```
salience_score = similarity × log(reinforcement_count + 1) × recency_factor
```

Where:
- **Reinforcement count**: How many times a memory was "reinforced" (re-encountered/re-validated via memorization)
- **Recency factor**: Exponential decay with configurable half-life (`recency_decay_days`, default 30 days)
- **Reinforcement factor**: Logarithmic scaling to prevent frequently-repeated facts from dominating

Configurable via `RetrieveItemConfig.ranking` (`"similarity"` | `"salience"`) and `MemorizeConfig.enable_item_reinforcement`.

Also includes **reinforcement tracking** (`reinforcement_count`, `last_reinforced_at` in `MemoryItem.extra`) — every time the same content hash is encountered again during memorization, the count increments and the timestamp updates.

**Relevance**: If Praxis uses pure cosine similarity, this salience-aware scoring with recency + reinforcement weighting is a differentiator.

---

### 3. Multi-Profile LLM Routing with Interceptor System
**Files**: `src/memu/llm/wrapper.py`, `src/memu/app/settings.py` (`LLMProfilesConfig`), `src/memu/app/service.py` (lines 228–282)

memU has a sophisticated **LLM profile system**: named profiles (`"default"`, `"embedding"`, plus custom ones) each with their own provider, model, base URL, API key, and client backend (SDK/httpx/LazyLLM). Workflow steps can reference specific profiles via `config.chat_llm_profile` / `config.embed_llm_profile`.

On top of that, there are **two interceptor systems**:

#### LLM Call Interceptors (`LLMInterceptorRegistry`)
- `intercept_before_llm_call(fn)` — inspect/modify requests before they hit the LLM
- `intercept_after_llm_call(fn)` — inspect responses with token usage metadata
- `intercept_on_error_llm_call(fn)` — handle LLM errors
- Has `where` filtering by operation, step_id, provider, model, status
- Supports priority ordering and handle-based disposal

#### Workflow Step Interceptors
- `intercept_before_workflow_step(fn)` — before each pipeline step
- `intercept_after_workflow_step(fn)` — after each pipeline step
- `intercept_on_error_workflow_step(fn)` — on step errors

**Relevance**: If Praxis lacks fine-grained LLM call interception (for logging, monitoring, caching, or policy enforcement) or multi-profile routing per pipeline step, memU's approach is more advanced.

---

### 4. Six Memory Types with Specialized Extraction Prompts
**Files**: `src/memu/prompts/memory_type/*.py`

memU defines **6 distinct memory types**, each with its own LLM extraction prompt with modular prompt blocks (objective → workflow → rules → category → output → examples → input):

| Type | Purpose |
|------|---------|
| `profile` | User identity, demographic, personality traits |
| `event` | Past experiences with temporal context |
| `knowledge` | Facts, concepts, learned information |
| `behavior` | Behavioral patterns and tendencies |
| `skill` | **Comprehensive skill profiles** with frontmatter, core principles, implementation guides, success patterns, pitfalls (300+ words, markdown format) |
| `tool` | Tool execution patterns with `when_to_use` hints, success/failure tracking, statistics |

The **skill type** is particularly notable — it extracts full structured documentation (not just one-line summaries). The **tool type** includes:
- `ToolCallResult` model with `tool_name`, `input`, `output`, `success`, `time_cost`, `token_cost`, `score`
- Hash-based deduplication of tool calls
- Statistics: `avg_time_cost`, `success_rate`, `avg_score`, `avg_token_cost`

Prompt blocks are also customizable per memory type via `CustomPrompt` (composable blocks with ordinals for ordering).

**Relevance**: If Praxis only has a single "memory" type or lacks specialized tool/skill extraction prompts, memU offers richer typing.

---

### 5. Multi-Modal Ingestion Pipeline with Dedicated Preprocessors
**Files**: `src/memu/prompts/preprocess/*.py`, `src/memu/utils/video.py`

memU preprocesses **5 modalities** before memory extraction:

| Modality | Preprocessing |
|----------|--------------|
| `conversation` | Segment into topic-based groups (≥20 messages), extract each segment |
| `document` | Text extraction |
| `image` | Vision-based caption generation |
| `video` | **ffmpeg frame extraction** — middle frame or N evenly-spaced frames, then vision captioning |
| `audio` | Speech-to-text transcription via VLM |

The `VideoFrameExtractor` class uses ffmpeg/ffprobe for frame extraction with configurable frame count and safety validations (CLI injection protection).

The `preprocess_multimodal` workflow step selects the right prompt based on modality and routes to the appropriate LLM (vision models for images/video, STT for audio).

**Relevance**: If Praxis's memory ingestion is text-only or lacks video/audio preprocessing, memU has a richer multimodal pipeline.

---

### 6. Staged Retrieval with Sufficiency Checks
**Files**: `src/memu/app/retrieve.py` (both `_build_rag_retrieve_workflow` and `_build_llm_retrieve_workflow`)

Retrieval proceeds in **three stages** with optional sufficiency checks between each:

1. **Route intention** → decide if retrieval is needed, rewrite query
2. **Category recall** → find relevant memory categories via embedding similarity
3. **Sufficiency check** (LLM-based) → "Is the information retrieved so far sufficient to answer the query?"
4. **Item recall** → find relevant memory items
5. **Sufficiency check** → decide if more info is needed
6. **Resource recall** → find original source resources
7. **Build context** → assemble results

If the sufficiency check determines enough context has been gathered at any stage, it shortcuts to skip further retrieval — **reducing unnecessary LLM calls and vector searches**.

There's also a `retrieve_llm` pipeline (alternative to `retrieve_rag`) that uses LLM ranking instead of embedding similarity for all stages.

**Relevance**: If Praxis does a single-stage vector search without sufficiency-based early termination or LLM-based ranking fallback, memU's approach is more sophisticated.

---

### 7. Item References in Category Summaries
**Files**: `src/memu/utils/references.py`

memU supports **inline citations** in auto-generated category summaries. When `MemorizeConfig.enable_item_references` is True, category summaries contain `[ref:ITEM_ID]` markers that link summary claims back to source memory items.

The `references.py` module provides:
- `extract_references(text)` — parse `[ref:id]` patterns
- `strip_references(text)` — clean for display
- `fetch_referenced_items(text, store)` — resolve refs to actual items
- `build_item_reference_map(items)` — format for LLM prompts

This enables **explainable memory recall** — the agent can see which specific memories support a category's summary.

**Relevance**: If Praxis generates category summaries without source attribution, memU's reference system provides traceability.

---

### 8. OpenAI Client Wrapper for Transparent Memory Injection
**File**: `src/memu/client/openai_wrapper.py`

memU provides a **drop-in OpenAI client wrapper** (`MemuChatCompletions`) that:
- Intercepts `chat.completions.create()` calls
- Extracts the user's latest query
- Automatically retrieves relevant memories via `MemoryService.retrieve()`
- Injects them as `<memu_context>` in the system prompt
- Supports both sync and async contexts
- Fails silently on memory errors (doesn't break the LLM call)

This is fully opt-in and backward compatible with existing OpenAI client code.

**Relevance**: If Praxis requires explicit memory retrieval calls in agent code, memU's auto-injection wrapper provides a simpler integration path.

---

### 9. LazyLLM Backend Integration
**File**: `src/memu/llm/lazyllm_client.py`, `src/memu/app/settings.py` (`LazyLLMSource`)

Beyond standard OpenAI-compatible backends (SDK, httpx), memU supports **LazyLLM** as a first-class client backend. LazyLLM provides access to Chinese LLM providers (Qwen, Doubao, Siliconflow, etc.) with separate source configuration for:
- `llm_source` (chat models)
- `vlm_source` (vision-language models)
- `embed_source` (embedding models)
- `stt_source` (speech-to-text models)

Each source can point to a different provider, enabling heterogeneous LLM routing.

**Relevance**: If Praxis only supports OpenAI-compatible providers, memU's LazyLLM integration opens access to Chinese/Asian LLM ecosystems.

---

### 10. Dedicated Memoization Blob Store
**File**: `src/memu/blob/local_fs.py`

Resources are **fetched and cached locally** in a configurable `resources_dir` before processing. The `LocalFS` class:
- Supports both local file paths and HTTP(S) URLs
- Handles query-parameter-based filenames (common with audio/image CDNs)
- Copies local files (doesn't move them)
- Caches all resources for re-processing

This decouples resource fetching from memory extraction — resources can be re-processed without re-downloading.

**Relevance**: If Praxis processes resources in-memory without local caching, memU's blob store enables offline re-processing and audit trails.

---

## Summary Table

| # | Feature | Why Praxis Might Not Have It |
|---|---------|------------------------------|
| 1 | **Runtime-mutable workflow pipelines** | Step-level insert/replace/remove at runtime vs monolithic pipeline |
| 2 | **Salience-aware ranking** | Weighted by reinforcement + recency, not just cosine similarity |
| 3 | **LLM + workflow interceptor systems** | Before/after/on-error hooks with filtering throughout |
| 4 | **6 typed memory types** (profile, event, knowledge, behavior, skill, tool) | Specialized skill profiles and tool execution memory |
| 5 | **Multi-modal preprocessor** (conversation, video, audio, image, document) | ffmpeg video frames, STT, vision captioning |
| 6 | **Staged retrieval with sufficiency checks** | Early-termination after category/item if context sufficient |
| 7 | **`[ref:ITEM_ID]` citations in summaries** | Traceable, explainable category summaries |
| 8 | **Drop-in OpenAI wrapper** | Auto-inject memories into any OpenAI client call |
| 9 | **LazyLLM backend** | Access to Qwen, Doubao, Siliconflow, etc. |
| 10 | **Local blob caching** | Resources cached to disk for offline re-processing |

---

## Verdict

memU is a **production-oriented memory framework** with significantly more architectural machinery than a simple embedding-index system. Its core differentiators are: (1) the **declarative workflow engine** with runtime mutability, (2) **salience-aware retrieval** with reinforcement tracking, (3) **multi-type memory** with specialized extraction (especially tool and skill types), (4) a **multi-modal preprocessing pipeline** with video/audio support, and (5) **comprehensive interceptor systems** for observability and customization.

If Praxis has a simpler, more monolithic memory pipeline, memU offers a more modular, extensible architecture worth studying.
