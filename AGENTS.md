# AGENTS.md — mindvault

## What this repo is

`mindvault` (mind + vault — "a vault for the mind") is a **cross-harness LLM knowledge base**. It serves two purposes:

1. **Long-term memory** — persist analysis results as notes under `docs/` so a
   future session retrieves them directly instead of re-generating from scratch.
   Notes are written **only when the user asks for them** (see "Long-term memory"
   under Rules for agents) — agents answer in-conversation by default.
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

### Long-term memory (ask first — never auto-write)

- Before researching a topic, scan `docs/` for existing notes and reuse what is
  already there.
- After completing analysis, **do NOT create or modify anything under `docs/` on
  your own initiative.** Answer the user in the conversation first, then *offer*
  to persist the result: propose the note (topic, target path, whether an
  existing note should be extended instead) and ask the user to confirm.
- Write the note, create directories, and add the `docs/SUMMARY.md` entry
  (following mdBook SUMMARY conventions) **only after the user explicitly
  agrees**. If the user declines or does not answer, leave the repo untouched —
  the in-conversation answer is sufficient.
- No unsolicited notes, no unsolicited `docs/SUMMARY.md` edits, no unsolicited
  edits to existing notes.

### Knowledge note template (standard structure)

Every knowledge note under `docs/` MUST follow this fixed section structure.
Section names are canonical fixed strings so cross-note searches stay reliable
(e.g. `grep "^## 延伸" docs/ -r`). Empty sections are omitted, never left as
placeholder headers.

Directory placement follows the topic-based top-level layout (e.g. `docs/security/`),
created lazily; each topic dir gets a `README.md` landing page plus a `docs/SUMMARY.md`
entry.

````markdown
# <Topic Name>

> 更新：YYYY-MM-DD

## 简介
What it is / what problem it solves, 3–5 sentences.
(Optional one line) Prerequisites: … / Related: [note](../xx/yy.md)

## 术语列表
| Term | Full Name | Meaning |
List only abbreviations used in this note (≤10 rows).

## 核心内容
### <Sub-topic 1>     ← must break into named subsections
### <Sub-topic 2>

## 延伸             ← omit section if empty
Content beyond the core, related but worth recording; organize in named ### subsections
(ops/config, troubleshooting, related applications, related topics, etc.).

## Q&A              ← omit section if empty
Each entry = one question + a 1–3 sentence answer.

## 参考资料
Links + access date.
````

### Note content rules

- **Language**: notes are written primarily in Chinese (zh-CN). English is allowed
  only where necessary — e.g. term full names (`Generic Security Services API`),
  protocol/message identifiers, or quoting original wording.
- **No sensitive info**: notes MUST NOT contain security-sensitive private data —
  keys (private keys, tokens, secrets, API keys), passwords, usernames / account
  names, real IP addresses (especially public ones), internal hostnames / FQDNs,
  or real infrastructure addresses (KDC / server addresses).
- **Redaction style**: when an example needs a host or address, use realistic
  placeholders such as `192.168.x.x` or `example.com` — not abstract ones like
  `<目标服务器>` — so examples stay concrete and readable.

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
