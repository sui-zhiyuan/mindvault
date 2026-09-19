#!/usr/bin/env bash
# pptx-read / read_pptx.sh
#
# One-shot, read-only view of a .pptx for an Agent:
#   - every slide's text  -> text.md  (read it with the `read` tool)
#   - every slide         -> PNG      (look at it with the `read_image` tool)
#   - whole deck          -> PDF
#
# Rendering uses the real PowerPoint engine on the Windows host through WSL
# interop, so fidelity is 100%: what the Agent sees is what PowerPoint shows.
# Nothing is installed and the original deck is never modified.
#
# Usage:
#   read_pptx.sh <deck.pptx> [--out DIR] [--no-render] [--dump]
#                [--width N] [--height N]
#
#   --out DIR     also copy text.md + PNGs into a WSL directory
#   --no-render   text only, skip PNG/PDF export (much faster)
#   --dump        additionally run the python-pptx structured dump
#                 (needs `uv`; first run downloads python-pptx)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PS1="$SCRIPT_DIR/render_pptx.ps1"
PY="$SCRIPT_DIR/dump_pptx.py"

DECK=""; OUT=""; WIDTH=1600; HEIGHT=900; RENDER=1; DUMP=0
while [ $# -gt 0 ]; do
  case "$1" in
    --out)       OUT="${2:-}";    shift 2;;
    --width)     WIDTH="${2:-}";  shift 2;;
    --height)    HEIGHT="${2:-}"; shift 2;;
    --no-render) RENDER=0;        shift;;
    --dump)      DUMP=1;          shift;;
    -h|--help)   sed -n '2,20p' "$0"; exit 0;;
    -*)          echo "unknown option: $1" >&2; exit 2;;
    *)           DECK="$1";       shift;;
  esac
done

if [ -z "$DECK" ]; then
  echo "usage: read_pptx.sh <deck.pptx> [--out DIR] [--no-render] [--dump]" >&2
  exit 2
fi
if [ ! -f "$DECK" ]; then
  echo "not a file: $DECK" >&2
  exit 2
fi

DECK_ABS="$(realpath "$DECK")"
DISTRO="${WSL_DISTRO_NAME:-Ubuntu}"

# WSL path -> Windows path. Windows PowerShell cannot open /home/... ; it needs
# the distro reachable over the 9p share.
if [[ "$DECK_ABS" == /* ]]; then
  WIN_SRC="\\\\wsl.localhost\\${DISTRO}$(printf '%s' "$DECK_ABS" | tr '/' '\\')"
else
  WIN_SRC="$DECK_ABS"
fi

# Non-ASCII must not travel through the argv boundary: Windows PowerShell would
# receive it in the ANSI code page. base64(UTF-8) is pure ASCII and lossless.
SRC_B64="$(printf '%s' "$WIN_SRC" | base64 -w0)"
SLUG="$(printf '%s' "$WIN_SRC" | sha1sum | cut -c1-10)"

SCRIPT_WIN="\\\\wsl.localhost\\${DISTRO}$(printf '%s' "$PS1" | tr '/' '\\')"

OUT_RAW="$(powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass \
  -File "$SCRIPT_WIN" \
  -SrcB64 "$SRC_B64" -Slug "$SLUG" \
  -Width "$WIDTH" -Height "$HEIGHT" -Render "$RENDER" 2>&1 || true)"

# PowerShell mixes CLIXML progress noise into stderr; keep only our own lines.
# It also writes CRLF, and a trailing CR would ride along inside every parsed
# value below -- which silently breaks the paths this script hands back.
STATUS="$(printf '%s\n' "$OUT_RAW" | grep -a -E '^[A-Z_]+=' | tr -d '\r' || true)"
printf '%s\n' "$STATUS"

if printf '%s\n' "$STATUS" | grep -qa '^STATUS=ERR_NO_SOURCE'; then
  echo "error: PowerPoint could not see $WIN_SRC" >&2
  exit 3
fi
if printf '%s\n' "$STATUS" | grep -qa '^STATUS=ERR_COM'; then
  echo "error: PowerPoint COM failed (is PowerPoint installed and not showing a dialog?)" >&2
  exit 4
fi

wsl_of() { printf '%s\n' "$STATUS" | sed -n "s|^$1=||p" | head -1; }

TEXT_MD_WSL="$(wsl_of TEXT_MD)"
TEXT_MD_WSL="${TEXT_MD_WSL//\\//}"
PNG_DIR_WSL="$(wsl_of PNG_DIR)"
PNG_DIR_WSL="${PNG_DIR_WSL//\\//}"
WORKDIR_WSL="$(wsl_of WSL_WORKDIR)"
COPY_WSL="$(wsl_of COPY_WSL)"

if [ -n "$TEXT_MD_WSL" ]; then
  # C:\Users\... -> /mnt/c/Users/...
  TEXT_MD_WSL="$(printf '%s' "$TEXT_MD_WSL" | sed -E 's|^([A-Za-z]):|/mnt/\L\1|')"
fi
if [ -n "$PNG_DIR_WSL" ]; then
  PNG_DIR_WSL="$(printf '%s' "$PNG_DIR_WSL" | sed -E 's|^([A-Za-z]):|/mnt/\L\1|')"
fi

echo "---"
[ -n "$TEXT_MD_WSL" ] && echo "text : $TEXT_MD_WSL"
[ -n "$PNG_DIR_WSL" ] && echo "png  : $PNG_DIR_WSL"
[ -n "$WORKDIR_WSL" ] && echo "work : $WORKDIR_WSL"
[ -n "$COPY_WSL" ] && echo "copy : $COPY_WSL"

if [ "$DUMP" = "1" ]; then
  echo "--- structured dump (python-pptx) ---"
  CACHE_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)/.dsh.local/uv-cache"
  mkdir -p "$CACHE_DIR"
  # Read the COPY, never the original. python-pptx holds no lock after it exits,
  # but routing every reader through the same private copy keeps the isolation
  # rule absolute and easy to reason about.
  DUMP_TARGET="${COPY_WSL:-$DECK_ABS}"
  UV_CACHE_DIR="$CACHE_DIR" uv run --no-project --with python-pptx -- \
    python "$PY" "$DUMP_TARGET"
fi

if [ -n "$OUT" ] && [ -n "$TEXT_MD_WSL" ]; then
  mkdir -p "$OUT"
  cp -f "$TEXT_MD_WSL" "$OUT/text.md"
  if [ -n "$PNG_DIR_WSL" ] && [ -d "$PNG_DIR_WSL" ]; then
    mkdir -p "$OUT/png"
    cp -f "$PNG_DIR_WSL"/*.PNG "$OUT/png/" 2>/dev/null || true
  fi
  echo "---"
  echo "copied -> $OUT/text.md ${OUT}/png/"
fi
