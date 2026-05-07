# Praxis Shelldex Implementation Plan

All features from ECOSYSTEM_RESEARCH.md, MOLTIS_IRONCLAW_DEEP_DIVE.md, and SHELLDEX_FEATURE_HARVEST.md.

## Batch 1: High-Impact Core (Week 1-2)

### 1. Zero-LLM / Rule-Based Mode
- Add `rules/mod.rs` - JSON/YAML rule definitions
- `RuleEngine` that matches patterns without LLM calls
- `ZeroLLMConfig` toggle
- Estimated: 3 days

### 2. Proactive Agent Mode
- Extend cron system with condition checking
- Add `proactive/mod.rs` - agent-initiated actions
- "I noticed X, so I did Y" behavior
- Estimated: 4 days

### 3. Skill Packs
- `skill_pack/mod.rs` - bundle multiple skills
- Pack registry with metadata
- Install/uninstall pack commands
- Estimated: 2 days

### 4. Heartbeat Check-Ins
- Agent actively checks in across channels
- Health signal persistence
- Estimated: 2 days

## Batch 2: Architecture & Isolation (Week 3-4)

### 5. Multi-Process Architecture (Enhance ProcessManager)
- Channel daemon, Worker daemon, Compactor daemon, Corrector daemon
- IPC via message passing
- Already partially done - needs completion
- Estimated: 5 days

### 6. Docker-Per-Agent Isolation
- `deployment/docker.rs` - per-agent containers
- Credential sandbox
- Estimated: 4 days

### 7. Signed WASM Plugins
- Cryptographically signed WASM
- Verification before execution
- Estimated: 3 days

## Batch 3: Integration Expansion (Week 5-6)

### 8. Code-First Integration API
- Trait for integrations
- 30+ integrations (Gmail, Slack, GitHub, Notion, Stripe, etc.)
- Estimated: 1 week

### 9. 32+ Built-in Tools
- Expand from 4 to 32 tools
- Shell, git, web, file, crypto, time, etc.
- Estimated: 5 days

### 10. Chinese Platform Channels
- QQ, Feishu, DingTalk, WeChat, WeCom
- Chinese LLM providers (DeepSeek, Doubao, Qwen, Kimi, Zhipu)
- Estimated: 4 days

## Batch 4: Advanced Features (Week 7-8)

### 11. Heartware Personality System
- Relationship context, moods, preferences
- Personality traits with weights
- Estimated: 4 days

### 12. Git-Native Agent
- Identity/rules/memory as version-controlled files
- Fork, branch, diff agents
- Estimated: 5 days

### 13. Agent-as-Worker Marketplace
- Connect to Moltlaunch marketplace
- Evaluate tasks, quote prices, execute, collect ratings
- Estimated: 1 week

## Batch 5: Special Modes (Week 9-10)

### 14. Browser-Only PWA Mode
- Web Worker agent loop
- IndexedDB for storage, OPFS for files
- WebVM sandbox
- Estimated: 1 week

### 15. Mobile-Native Agent
- Android/iOS companion app
- Control surface and notifications
- Estimated: 1 week

### 16. Trigger-Based Tool Execution
- Event-driven tools without LLM
- Webhook → tool chain
- Estimated: 3 days

## Batch 6: Niche Features (Week 11-12)

### 17. Local STT (whisper.cpp)
- Offline speech-to-text
- Estimated: 3 days

### 18. Self-Improvement from Ratings
- Star ratings, behavior adjustment
- Estimated: 3 days

### 19. Onchain Reputation System
- Verifiable onchain agent reputation
- Estimated: 4 days

### 20. Skill Creation at Runtime
- Agent creates skills mid-session
- Estimated: 4 days

---

## Next Steps

Starting Batch 1 immediately. First: Zero-LLM Rule-Based Mode - creating deterministic agent behavior without LLM calls for routine tasks.