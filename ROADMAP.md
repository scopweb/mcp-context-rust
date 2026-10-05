# 🛣️ MCP Context Rust — Active Roadmap (2026+)

> **Project:** High-performance context & memory engine for AI coding assistants (MCP server)  
> **Language:** Rust  
> **Current Status:** Functional with strong foundations (analysis + pattern training + Endless Mode)  
> **Strategic Goal:** Become the best **persistent memory + intelligent context** MCP server in the ecosystem.

---

## 📍 Current Reality (Honest Assessment)

The project already has excellent technical foundations:

**Strengths**
- Solid multi-language project analysis (7 languages)
- Mature, secure **TrainingManager** with scoring and indexes
- Very powerful **Endless Mode** + `ObservationStore` (95% token reduction)
- `.rustscp` persistent project context caching + CLI `read-context`
- Clean 8-tool MCP interface
- High code quality (Clippy pedantic, ~42 tests, good security hygiene)

**Critical Gaps** (vs what the official site promises and what users actually need)
- **No real conversation / persistent memory** — Claude forgets everything between sessions
- **No documentation intelligence** — cannot fetch or cache dependency docs
- Pattern catalog is heavily skewed (~84 of 88+ patterns are Blazor-only)
- No unified high-level context tool (`get-context`)
- Analysis is not incremental/cached

**Bottom line:** Today it is a very good *pattern + analysis* server. It is not yet a true **"memory engine"**.

---

## 🎯 Prioritized 5-Phase Roadmap

### Phase 1: Memory Core (Highest Priority)

**Goal:** Give the server **real persistent memory** so it finally becomes a "memory engine".

**Key Deliverables**
- New `src/memory/` module
- Persistent storage for:
  - Project memories (decisions, conventions, gotchas, architecture notes)
  - Conversation summaries / important facts
  - User preferences per project
- New MCP tools (or major extensions):
  - `remember` (store important context)
  - `recall` (search memory)
  - `get-memory` (retrieve relevant memories for current task)
- Deep integration:
  - `analyze-project` automatically surfaces relevant memories
  - `get-help` and pattern suggestions are memory-aware
- Storage strategy: JSON files + indexes (initially) or SQLite (if complexity justifies it)

**Why first?**
Without memory, patterns and analysis lose most of their long-term value (as documented in `HONEST_ASSESSMENT.md`).

**Estimated effort:** 3–5 weeks

---

### Phase 2: Context Unification + Pattern Renaissance

**Goal:** Make the existing strengths (patterns + analysis) actually deliver massive value.

**Key Deliverables**
- New unified tool: `get-context` (the single most useful tool for users)
  - Combines: current project analysis + relevant memories + best matching patterns + smart suggestions
- Massive expansion of the built-in pattern catalog
  - Strong baseline patterns for: Rust, Go, Python, Node/TypeScript, Laravel, Spring, ASP.NET
  - Better auto-tagging and relevance scoring when training new patterns
- Improvements to `TrainingManager` (project-specific patterns, better auto-extraction)
- Update `ContextBuilder` to support the new unified context generation

**Why this phase?**
Once memory exists, the next biggest win is reducing friction ("one tool call instead of four").

**Estimated effort:** 3–4 weeks

---

### Phase 3: Documentation Intelligence

**Goal:** Close the "fetch dependency docs" promise from the official positioning.

**Key Deliverables**
- New `fetch-docs` tool (and integration into `get-context`)
- Local documentation cache (`cache/docs/`)
- Hybrid fetching:
  - Local first (previously fetched docs)
  - Fallback to docs.rs / crates.io / PyPI / npm / NuGet / etc.
  - Optional Context7 or similar external service
- Smart summarization of dependency documentation relevant to the current project

**Estimated effort:** 4–6 weeks

---

### Phase 4: Advanced Context Engine

**Goal:** Make the system feel magical and production-grade.

**Key Deliverables**
- Incremental / differential project analysis (fingerprinting + cache invalidation)
- Significantly richer `.rustscp` format and CLI experience
- Full-featured CLI (`mcp-context-rust analyze`, `memory`, `patterns`, `recall`, etc.)
- Better monorepo / workspace support
- Optional multi-transport (HTTP + SSE) in addition to stdio
- Usage metrics and self-observation of the server

**Estimated effort:** 3–4 weeks

---

### Phase 5: Ecosystem & Hardening (Optional / Demand-driven)

- Team/shared memory (multiple developers using the same memory store)
- Deeper integrations (GitHub Issues, Linear, Jira, App Insights, etc.)
- Public distribution improvements (easier install, prebuilt binaries)
- Plugin/extension system for custom memory sources or doc fetchers

---

## 📊 Priority Matrix

| Phase | Strategic Value | User Impact | Technical Risk | Recommended Order |
|-------|------------------|-------------|----------------|-------------------|
| **1. Memory Core** | ★★★★★ | ★★★★★ | Low-Medium | **P0** |
| **2. Context Unification + Patterns** | ★★★★☆ | ★★★★★ | Low | **P1** |
| **3. Documentation Intelligence** | ★★★★ | ★★★★ | Medium | P2 |
| **4. Advanced Context Engine** | ★★★★ | ★★★★ | Medium | P3 |
| **5. Ecosystem** | ★★★ | ★★★ | Higher | P4 |

---

## 🧠 Core Design Principles (for all future work)

1. **Memory is the product** — Everything else exists to feed better memory and better retrieval.
2. **Leverage what already exists** — TrainingManager, ObservationStore, ContextBuilder, and .rustscp are excellent foundations. Extend them instead of replacing.
3. **One great tool beats four good ones** — Prioritize `get-context` and memory-aware flows.
4. **Private data > Public patterns** — The highest value always comes from what *this specific user/project* knows that Claude doesn't.
5. **Endless Mode is a superpower** — Never break the ability to do 20–50× more tool calls.

---

## ❓ Open Questions (to be answered before/during Phase 1)

1. **Storage for Memory**
   - Start with simple JSON + indexes (consistent with current patterns approach)?
   - Or introduce SQLite (`rusqlite`) from the beginning for better querying?

2. **Scope of Memory**
   - Per-project only (recommended for v1)?
   - Global + per-project?
   - Should we also persist full conversation transcripts or just structured facts/decisions?

3. **Documentation Fetching Priority**
   - Is a lightweight local cache + basic fetcher enough for Phase 3, or do we need Context7-level quality immediately?

4. **Go Sibling Alignment**
   - Do we want to keep `mcp-go-context` in sync with this roadmap (API compatibility for `remember`/`recall`/`get-context`, etc.)?

---

## 🚀 How to Resume This Work Later

When you come back to continue:

1. Read this file (`ROADMAP.md`)
2. Read `HONEST_ASSESSMENT.md` (still very relevant)
3. Read `docs/CLAUDE.md` (architecture overview)
4. Start with **Phase 1** unless a specific business reason pushes something else higher.
5. Before coding Phase 1, re-answer the "Open Questions" section above.

---

## 📚 Related Documents

- [HONEST_ASSESSMENT.md](HONEST_ASSESSMENT.md) — Brutally honest evaluation of current value
- [docs/CLAUDE.md](docs/CLAUDE.md) — Development guide and architecture
- [docs/OLD_ROADMAP.md](docs/OLD_ROADMAP.md) — Previous roadmap (archived)
- [docs/PATTERNS_CATALOG.md](docs/PATTERNS_CATALOG.md) — Current pattern inventory

---

**Last Updated:** 2026 (after deep codebase analysis)  
**Maintained by:** Grok + Project Owner

**Current Phase:** Phase 2 in progress — `get-context` unified tool implemented (analysis + task-aware memories + patterns + suggestions). Pattern catalog expansion and TrainingManager/ContextBuilder improvements next. See CHANGELOG for details.

## ✅ Phase 1 Decisions (answered during implementation kickoff)

- **Storage:** Hybrid — start with JSON files + indexes (single `data/memories/memories.json`). SQLite (`rusqlite`) deferred until query needs justify it.
- **Scope:** Global + per-project from day one. Project memories use canonical path for stable identity.
- **Content v1:** Structured facts/decisions only (category, title, content, tags, importance). No full transcripts.
- **Go sibling:** Tool names (`remember`/`recall`/`get-memory`) + core record shape designed with cross-impl compatibility in mind. IDs are UUIDs.

See `src/memory/mod.rs`, `src/types.rs` (Memory*, RememberInput, MemorySearchCriteria), and MCP tool additions for the concrete design.

**Implementation notes:**
- New `MemoryStore` (modeled after TrainingManager for consistency + security patterns).
- 3 new MCP tools + deep integration: `analyze-project` now auto-surfaces relevant memories (global + project) and bumps recall stats.
- Memories persisted alongside patterns/observations under the configured `base_path`.
- Full test coverage for the store; all existing tests continue to pass.