# AGENTS.md — mindvault

## What this repo is

`mindvault` (mind + vault — "a vault for the mind") is a **cross-harness LLM knowledge base**. It serves two purposes:

1. **Long-term memory** — persist analysis results as notes under `docs/` so a
   future session retrieves them directly instead of re-generating from scratch.
2. **User knowledge profile (goal, not yet implemented)** — record what the user
   already knows vs. doesn't know. When explaining something, consult this
   profile first and skip topics the user already understands, expanding only on
   unfamiliar ones, to improve interaction efficiency.

> No profile file exists yet; this is a stated goal, not an implemented feature.
>
> Renamed from `opencode-play` (2026-09) — kept for tracing any leftover references.

The repo currently takes the form of an [mdBook](https://rust-lang.github.io/mdBook/)
project — content lives in `docs/`, written primarily in Chinese (zh-CN). It is a
**knowledge/reference workspace, not application source code**.

## Project structure

```
docs/                   ← mdBook source (book.toml: src = "docs")
├── SUMMARY.md          ← mdBook table of contents (index)
├── arm-kunpeng/        ← ARM / Kunpeng CPU architecture notes
├── ascend-npu/         ← Ascend NPU (CANN) notes
├── rust-kunpeng/       ← Rust + Kunpeng ecosystem strategy
├── triton-cpu/         ← Triton on ARM CPU
├── tools-and-tips/     ← Editor tips, IDE bug records
│   └── jetbrains/
└── learning/           ← Self-study notes (Transformer, etc.)

daft_demo/              ← Demo code: Daft + Ray (not part of the book)
dist/                   ← Built artifacts of daft_demo
target/                 ← mdBook build output (gitignored)
```

## Commands

- `mdbook build` — build the book (output to `target/book/`)
- `mdbook serve` — live preview at `http://localhost:3000`
- No tests, lint, or deploy for the book content.

## Rules for agents

### Long-term memory (implemented)

- Before researching a topic, scan `docs/` for existing notes and reuse what is
  already there.
- After completing analysis, persist the result as a note under `docs/` and add
  an entry to `docs/SUMMARY.md` (following mdBook SUMMARY conventions), so it is
  retrievable later.

### User knowledge profile (goal)

- Intended behavior: when answering, consult the profile first and skip topics
  the user already knows. This is a goal only — no profile exists yet.

### Answer from knowledge first

- If you know the answer confidently, answer directly.
- If unsure, search the web AND scan relevant local docs in parallel — synthesize
  a complete answer from both sources. Do NOT rely on local docs alone without
  external verification.
- Do NOT use explore/librarian agents for factual questions (those search
  codebases, not the web).
- Do NOT consult Oracle/Metis/Momus for factual questions — those are for
  architecture/debugging/planning of actual codebases.

### Keep it concise

- This is a reference/knowledge workspace. No project-structure exploration,
  codebase assessment, or implementation phases unless explicitly asked.

### Language

- Content is primarily Chinese (zh-CN). Match the language of the document you're
  reading or the user's prompt.
