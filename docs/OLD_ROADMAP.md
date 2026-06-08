# 🛣️ OLD ROADMAP (Archived)

**Archived:** 2026 — during roadmap replanning session

This file preserves the previous roadmap for historical reference. It was written during the early phases of the project (focused on completing Phase 0 security/MCP fixes and then proposing corporate integrations).

It has been replaced by the active roadmap in [ROADMAP.md](../ROADMAP.md).

---

## Original Roadmap Content (as of early 2026)

# 🛣️ Roadmap - From PoC to Production-Ready Tool

## 📊 Current State (v0.2.0)

**Status:** ✅ **Phase 0 Complete** - Ready for Phase 1 development

### What Works
- ✅ Basic MCP protocol structure with Content-Length framing
- ✅ 27+ code patterns for multiple frameworks
- ✅ Pattern storage and retrieval (with security validation)
- ✅ Tree-sitter C# parsing infrastructure
- ✅ Multi-language project detection (7 languages)
- ✅ Custom error types with thiserror
- ✅ Structured logging with tracing
- ✅ Clippy pedantic lints enabled
- ✅ UTF-8 safe string handling
- ✅ 42 tests passing

### What Was Fixed (Phase 0)
- ✅ **Security:** Path traversal vulnerability fixed with `sanitize_framework_name()`
- ✅ **MCP Protocol:** Content-Length framing implemented
- ✅ **Functionality:** Project analyzer returns file list correctly
- ✅ **Tests:** All 42 tests compile and pass

---

## 🚨 Phase 0: Critical Fixes (URGENT)

> **These issues must be fixed before any other development.**
> **Estimated time: 1 day**

### 0.1 Security: Path Traversal Vulnerability

**File:** `src/training/mod.rs` (lines 103-108)

**Problem:** `save_patterns()` uses unsanitized `framework` parameter to construct file paths...

(Full original content continues in git history under the commit before this archiving.)

---

**Key Insight from Old Roadmap:**
> The MCP isn't useful for what Claude already knows.
> It's useful for what Claude CAN'T know without it.

This principle remains valid and was carried forward into the new roadmap.

---

*Original file last updated around 2026-02-12.*