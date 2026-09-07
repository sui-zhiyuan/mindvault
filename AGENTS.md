# AGENTS.md — mindvault

## What this repo is

A personal technical knowledge base organized as an [mdBook](https://rust-lang.github.io/mdBook/) project. Content is in `docs/` — structured reference notes, not application source code.

## Project structure

```
docs/                   ← mdBook source (configured via `book.toml`: `src = "docs"`)
├── arm-kunpeng/        ← ARM / Kunpeng CPU architecture notes
├── rust-kunpeng/       ← Rust + Kunpeng ecosystem strategy
├── triton-cpu/         ← Triton on ARM CPU (placeholder)
├── tools-and-tips/     ← Editor tips, IDE bug records
│   └── jetbrains/
└── learning/           ← Self-study notes (Transformer, etc.)

daft_demo/              ← Demo code: Daft + Ray (not part of the book)
triton-cpu-docs/        ← CSV data files for Triton CPU ops
```

## Commands

- `mdbook build` — build the book (output to `target/book/`)
- `mdbook serve` — live preview at `http://localhost:3000`
- No tests, lint, or deploy for the book content.

## Rules for agents

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

This is a reference/knowledge workspace. No project-structure exploration,
codebase assessment, or implementation phases unless explicitly asked.

### Language

Content is primarily Chinese (zh-CN). Match the language of the document you're
reading or the user's prompt.
