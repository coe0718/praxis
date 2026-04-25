# Praxis — Implementation Roadmap (Scout's Independent Plan)
**Author:** Scout | **Date:** 2026-04-22
**Inputs:** CODE_REVIEW_VEX.md (12 Critical, 23 Warning), PRAXIS_NEW_FEATURES.md (10 features), ECOSYSTEM_REVIEW.md, CLAUDE.md

---

## Guiding Principles

1. **Security is existential.** 12 criticals found — any one could compromise the entire system. Fix these FIRST.
2. **Incremental wins.** Build momentum with quick fixes that improve daily experience, not just theoretical security.
3. **User visibility matters most.** Operators need to understand what's happening — timeline views and health dashboards are more important than obscure features.
4. **Defensive programming.** Assume the agent WILL be compromised — build in forensics and recovery.
5. **Phased dependencies.** Each phase must deliver working value, not just prerequisites.

---

## Phase 1 — Critical Security Fixes (Week 1: Immediate Action)

### P0.5 — Same-Day Criticals (Prevent immediate exploitation)

| # | Issue | Impact | Fix | Priority |
|---|-------|---------|-----|----------|
| C1 | Shell injection via newline/CR/backslash | **Critical** - RCE | Add `\n`, `\r`, `\\` to `DANGEROUS_SHELL_CHARS` | **IMMEDIATE** |
| C2 | OAuth token exfiltration via HTTP tools | **Critical** - Data breach | Filter tokens by `allowed_oauth_providers` in `run_http` | **IMMEDIATE** |
| C12 | SSE token in URL query / no CSRF | **Critical** - Session hijacking | Move token to Authorization header, add CSRF token | **HIGH** |

### P1 — First Week (Build Security Foundation)

| # | Issue | Impact | Fix | Dependencies |
|---|-------|---------|-----|-------------|
| C3 | Daemon urgency loss (double consume) | High - Missed critical tasks | Set `force: true` when daemon detects wake intent | C1 fix |
| C5 | Non-atomic hot/cold memory insert | High - Silent data corruption | Wrap INSERT + FTS in transaction | - |
| C7 | MCP `resources/read` arbitrary file read | Critical - Vault/config exposure | Restrict to enumerated resources only | - |
| C8 | MCP `tools/call` no tool validation | Critical - Phantom approvals | Validate tool_name against registry | - |
| C10 | Predictable pairing code (6-digit) | Medium - Auth bypass | Random 6-digit code + rate limit + expiry | - |
| W1 | No SSRF protection | High - Internal network access | Block localhost, private ranges, 169.254.0.0/16 | - |
| W2 | Unbounded command output | High - Resource exhaustion | 1MB cap on stdout/stderr | - |
| W11 | Per-operation DB connections | Medium - Performance | Basic connection pooling | C5 fix |

**Success Criteria:** 
- All critical security fixes implemented
- Vex review complete with sign-off
- CI green on security tests
- No new vulnerabilities introduced

---

## Phase 2 — Reliability & User Experience (Weeks 2-3)

### 2A — Memory Architecture Upgrade (Highest Impact Daily Improvement)

**Why:** Current keyword-only search is the biggest daily frustration. Vector search will change how users interact with memories.

| Task | Implementation | Dependencies |
|------|---------------|-------------|
| **Add embedding column** | `ALTER TABLE hot_cold_memories ADD COLUMN embedding BLOB` | Phase 1 complete |
| **Embedding generation** | OpenAI embeddings API + batch migration for existing memories | - |
| **Hybrid search** | 0.7 vector + 0.3 keyword weighted scoring | - |
| **Memory consolidation** | Use embeddings to detect near-duplicates | - |
| **Frontend integration** | Search interface with relevance scores, toggle modes | - |

**Risk Mitigation:** Feature flag behind `memory-v2`, rollback migration script.

### 2B — Frontend Visibility & Control (Immediate User Value)

| Feature | Implementation | User Impact |
|---------|----------------|-------------|
| **Session Timeline View** | Visualize Orient→Decide→Act→Reflect with timestamps | **HIGH** - Debug understanding |
| **Approval Queue Search** | Filter by tool, path, date, requested_by | **HIGH** - Faster triage |
| **Token Spend Tracking** | Real-time charts from `provider_usage` | **MEDIUM** - Cost awareness |
| **Agent Health Dashboard** | Memory, DB size, error rate, uptime trends | **HIGH** - System awareness |

### 2C — Critical Reliability Fixes

| Issue | Fix | Impact |
|-------|-----|--------|
| C4 | SQL injection in schema migration | Validate identifiers with regex |
| C9 | Shell execution from eval JSON | Command allowlist for eval/reviewer |
| C11 | Prompt caching dead code | Compare input token count vs output limit |
| W6-W10 | Non-atomic multi-table ops | Transaction wrapping across providers, sessions, approvals |

---

## Phase 3 — Advanced Features & Ecosystem Integration (Weeks 4+)

### 3A — Goal & Agent Intelligence

| Feature | Implementation | Strategy |
|---------|----------------|----------|
| **Goal Dependency Graph** | Visualizer showing goal relationships and blockers | Network graph D3/React |
| **Config Diff on Evolution** | Side-by-side diff for evolution proposals | `diff` library integration |
| **Session Replay** | Step-by-step replay with timestamps | Log-based reconstruction |
| **Browser Notifications** | Push notifications for approvals | Service Worker + SSE |

### 3B — Advanced Security Features

| Feature | Implementation | Ecosystem Source |
|---------|----------------|------------------|
| **WASM Sandbox** | `wasmtime` fuel metering + epoch interruption | OpenFang approach |
| **Secret Zeroization** | Wipe key bytes from memory after use | NullClaw pattern |
| **Autonomy Levels** | Global ReadOnly/Supervised/Full toggle | NullClaw design |
| **Prompt Injection Scanner** | Detect override attempts in tool outputs | OpenFang feature |

### 3C — Ecosystem Integration

| Feature | Implementation | Value |
|---------|----------------|-------|
| **Rule-based Model Routing** | Task complexity → model selection | Cost optimization |
| **CodeAct Mode** | Agent writes code blocks, Praxis executes | Flexibility |
| **Merkle Audit Trail** | Cryptographic action linking | Forensics |

---

## Implementation Strategy

### Team Structure

| Role | Primary Agent | Secondary | Responsibilities |
|------|---------------|-----------|------------------|
| **Security Lead** | Vex | Drey | Critical fixes, security reviews, penetration testing |
| **Primary Coder** | Drey | Vex | Implementation, unit tests, integration |
| **UX/Integration** | Scout | Tuck | Frontend polish, user experience, ecosystem research |
| **Architecture** | Tuck | Scout | Technical oversight, dependency management, risk assessment |

### Quality Gates

1. **Every security fix requires Vex sign-off**
2. **All database changes include rollback migrations**
3. **Frontend features must work on mobile**
4. **Performance benchmarks must not degrade**
5. **Memory changes must preserve existing data**

### Risk Management

| Risk | Likelihood | Mitigation Strategy |
|------|-----------|---------------------|
| **Complexity explosion** | High | Feature flags, incremental delivery |
| **Performance degradation** | Medium | Benchmark testing, gradual rollout |
| **User confusion** | High | Documentation, gradual feature exposure |
| **Security regressions** | High | Automated scanning, Vex review |

---

## Success Metrics

### Week 1 Metrics
- [ ] 12 critical security fixes implemented
- [ ] Vex review complete with no outstanding issues
- [ ] CI passing on all security tests
- [ ] No new vulnerabilities reported

### Week 2-3 Metrics  
- [ ] Memory upgrade complete with 50%+ search improvement
- [ ] Frontend timeline view deployed
- [ ] Approval search reducing response time by 70%
- [ ] Health dashboard operational

### Week 4+ Metrics
- [ ] User satisfaction scores improved
- [ ] System reliability (error rate < 1%)
- [ ] Cost optimization through model routing
- [ ] Security audit passed

---

## Dependencies & Sequencing

**Critical Path:**
1. Security fixes (P0.5) → Enable trust in continued development
2. Memory upgrade (2A) → Daily impact justification  
3. Frontend visibility (2B) → User feedback loop
4. Advanced features (3A-3C) → Platform differentiation

**Non-Negotiable Dependencies:**
- No Phase 2 work until Phase 1 security fixes are complete
- No database schema changes without rollback plan
- No frontend features without mobile responsiveness

---

## Comparison with Tuck's Plan

**Key Differences:**

1. **Prioritization:** Focus on immediate user-visible improvements (timeline, health dashboard) alongside security
2. **Team Structure:** Vex as dedicated Security Lead rather than just reviewer
3. **Risk Management:** Feature flags and gradual rollout for complex changes
4. **User Focus:** Emphasis on visibility and control — operators need to understand what's happening
5. **Ecosystem Integration:** More aggressive adoption of proven patterns (OpenFang, NullClaw)

**Shared Principles:**
- Security first
- Backward compatibility  
- No new infrastructure
- Drey/Vex pairing for implementation/review

*Independent plan by Scout. Compare against Tuck's PLAN_TUCK.md for Jeremy's decision.*