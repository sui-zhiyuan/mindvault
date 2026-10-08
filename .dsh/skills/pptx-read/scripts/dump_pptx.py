#!/usr/bin/env python3
"""pptx-read / dump_pptx.py

Structured, read-only dump of a .pptx through python-pptx.

This complements the PowerPoint-COM path: COM gives a pixel-perfect picture of
each slide, this gives exact structure -- shape names and types, geometry in
points, tables cell by cell, and speaker notes -- which is what you want when
the text has to be reasoned over rather than looked at.

Run it through uv so nothing is installed into the system interpreter:

    UV_CACHE_DIR=<writable-dir> uv run --no-project --with python-pptx -- \
        python dump_pptx.py <deck.pptx> [--format md|json] [--out FILE]
        [--notes/--no-notes] [--geometry/--no-geometry]
"""

from __future__ import annotations

import argparse
import json
import sys

from pptx import Presentation
from pptx.util import Emu

EMU_PER_PT = 12700


def pt(v) -> float | None:
    """EMU -> points, rounded; None when the shape has no such value."""
    if v is None:
        return None
    return round(Emu(v).pt, 1)


def shape_text(shape) -> str:
    if not getattr(shape, "has_text_frame", False):
        return ""
    return shape.text_frame.text.strip()


def table_rows(shape) -> list[list[str]]:
    """A table's cells, with merged cells carrying their origin's text.

    python-pptx stores a merged cell's text in its **origin** cell and reports
    every covered cell as empty (`is_spanned`), so a vertically merged first
    column -- the ordinary shape of a Chinese report table -- reads as a label on
    the first row and blank rows under it. PowerPoint's own model repeats the
    value, and the COM path therefore shows it on every row; these spans are what
    let the python-pptx path agree with it.
    """
    if not getattr(shape, "has_table", False):
        return []
    tbl = shape.table
    n_rows = len(tbl.rows)
    n_cols = len(tbl.columns)
    filled = [["" for _ in range(n_cols)] for _ in range(n_rows)]
    for r in range(n_rows):
        for c in range(n_cols):
            cell = tbl.cell(r, c)
            if getattr(cell, "is_spanned", False):
                continue
            text = cell.text.strip()
            origin = bool(getattr(cell, "is_merge_origin", False))
            height = (getattr(cell, "span_height", 1) or 1) if origin else 1
            width = (getattr(cell, "span_width", 1) or 1) if origin else 1
            for rr in range(r, min(r + max(height, 1), n_rows)):
                for cc in range(c, min(c + max(width, 1), n_cols)):
                    filled[rr][cc] = text
    return filled


def collect(deck_path: str, notes: bool, geometry: bool) -> dict:
    prs = Presentation(deck_path)
    slides = []

    for idx, slide in enumerate(prs.slides, start=1):
        entry = {
            "index": idx,
            "layout": slide.slide_layout.name,
            "shapes": [],
        }
        if notes and slide.has_notes_slide:
            entry["notes"] = slide.notes_slide.notes_text_frame.text.strip()

        for shape in slide.shapes:
            item = {
                "name": shape.name,
                "type": str(shape.shape_type).split(" ")[0] if shape.shape_type else "",
            }
            text = shape_text(shape)
            if text:
                item["text"] = text
            rows = table_rows(shape)
            if rows:
                item["table"] = rows
            if geometry:
                item["box_pt"] = {
                    "left": pt(shape.left),
                    "top": pt(shape.top),
                    "width": pt(shape.width),
                    "height": pt(shape.height),
                }
            entry["shapes"].append(item)

        slides.append(entry)

    return {
        "file": deck_path,
        "slides": len(slides),
        "size_pt": {
            "width": round(Emu(prs.slide_width).pt, 1),
            "height": round(Emu(prs.slide_height).pt, 1),
        },
        "aspect": (
            round(prs.slide_width / prs.slide_height, 4) if prs.slide_height else None
        ),
        "deck": slides,
    }


def cell_line(cell: str) -> str:
    """One table cell as one line.

    A cell's text may hold newlines (a wrapped paragraph in a table is one cell,
    not two rows). Emitted raw, they split the row across lines and destroy the
    `| a | b |` shape the reader relies on -- the same defect the COM path fixed
    by flattening `\\r\\n` to a space. An unescaped pipe would forge a column
    boundary, so it is escaped too.
    """
    return (
        cell.replace("|", "\\|")
        .replace("\r\n", " ")
        .replace("\r", " ")
        .replace("\n", " ")
    )


def to_markdown(d: dict) -> str:
    out = [
        f"# {d['file'].split('/')[-1]}",
        "",
        f"- slides: {d['slides']}",
        f"- page size (pt): {d['size_pt']['width']} x {d['size_pt']['height']}",
        f"- aspect: {d['aspect']}",
        "",
    ]
    for s in d["deck"]:
        out.append(f"## Slide {s['index']}  (layout: {s['layout']})")
        out.append("")
        for sh in s["shapes"]:
            head = sh["name"]
            if sh.get("type"):
                head += f"  [{sh['type']}]"
            out.append(f"### {head}")
            if sh.get("box_pt"):
                b = sh["box_pt"]
                out.append(
                    f"`{b['left']},{b['top']}  {b['width']}x{b['height']} pt`"
                )
            if sh.get("text"):
                out.append(sh["text"])
            for row in sh.get("table") or []:
                out.append("| " + " | ".join(cell_line(c) for c in row) + " |")
            out.append("")
        if s.get("notes"):
            # One blockquote line per note line: a note may hold newlines, and
            # emitted raw they would look like body text rather than a quote.
            # `grep '^> notes:'` still finds the block's first line.
            note_lines = s["notes"].split("\n")
            out.append(f"> notes: {note_lines[0]}")
            out.extend(f"> {line}" for line in note_lines[1:])
            out.append("")
    return "\n".join(out)


def to_text(d: dict, title: str) -> str:
    """The page-by-page text document, built without PowerPoint.

    `read_pptx.sh --no-render` used to get its `text.md` from the COM path, so
    "text only" still needed PowerPoint running. This writes the same document
    from python-pptx: the same `## Slide N` headings (so the per-page `awk`
    recipes keep matching) and the same `### shape` / `[TABLE RxC]` shapes, plus
    the speaker notes -- which the COM path never emitted.
    """
    out = [
        f"# {title}",
        "",
        f"- slides: {d['slides']}",
        # `:g` drops the trailing `.0` python-pptx keeps, so this line reads
        # exactly as the COM path writes it.
        f"- page size (pt): {d['size_pt']['width']:g} x {d['size_pt']['height']:g}",
        "",
    ]
    for s in d["deck"]:
        out.append(f"## Slide {s['index']}")
        out.append("")
        found = 0
        for sh in s["shapes"]:
            if sh.get("text"):
                found += 1
                out.append(f"### {sh['name']}")
                out.append(sh["text"])
                out.append("")
            rows = sh.get("table") or []
            if rows:
                found += 1
                cols = len(rows[0])
                out.append(f"### {sh['name']} [TABLE {len(rows)}x{cols}]")
                for row in rows:
                    out.append("| " + " | ".join(cell_line(c) for c in row) + " |")
                out.append("")
        if found == 0:
            out.append("(no text shapes)")
            out.append("")
        if s.get("notes"):
            # One blockquote line per note line: a note may hold newlines, and
            # emitted raw they would look like body text rather than a quote.
            # `grep '^> notes:'` still finds the block's first line.
            note_lines = s["notes"].split("\n")
            out.append(f"> notes: {note_lines[0]}")
            out.extend(f"> {line}" for line in note_lines[1:])
            out.append("")
    return "\n".join(out)


def main() -> int:
    ap = argparse.ArgumentParser(description="Read-only structured dump of a .pptx")
    ap.add_argument("deck")
    ap.add_argument("--format", choices=("md", "json", "text"), default="md")
    ap.add_argument("--title", default=None, help="title line for --format text")
    ap.add_argument("--out", default=None)
    ap.add_argument("--notes", dest="notes", action="store_true", default=True)
    ap.add_argument("--no-notes", dest="notes", action="store_false")
    ap.add_argument("--geometry", dest="geometry", action="store_true", default=False)
    ap.add_argument("--no-geometry", dest="geometry", action="store_false")
    args = ap.parse_args()

    data = collect(args.deck, args.notes, args.geometry)
    if args.format == "json":
        text = json.dumps(data, ensure_ascii=False, indent=2)
    elif args.format == "text":
        text = to_text(data, args.title if args.title else args.deck.split("/")[-1])
    else:
        text = to_markdown(data)

    if args.out:
        with open(args.out, "w", encoding="utf-8") as fh:
            fh.write(text)
        print(f"wrote {args.out} ({len(text)} chars)")
    else:
        sys.stdout.reconfigure(encoding="utf-8")
        print(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
