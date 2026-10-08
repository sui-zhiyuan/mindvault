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
#                [--width N] [--height N] [--timeout SEC] [--net-timeout SEC]
#   read_pptx.sh --check
#
#   --out DIR        also copy text.md + PNGs into a WSL directory
#   --no-render      text only, skip PNG/PDF export (much faster)
#   --dump           additionally run the python-pptx structured dump
#                    (needs `uv`; first run downloads python-pptx)
#   --timeout SEC    give up on the PowerPoint step after SEC seconds (default 180)
#   --net-timeout SEC  give up on `uv` after SEC seconds (default 90)
#   --check          report the environment this skill depends on, then exit
#
# Every external call is bounded. A blocked PowerPoint COM call used to hang the
# whole run silently, forever, because nothing here had a timeout and a launch
# failure was swallowed by `2>&1 || true`; now each failure mode has its own exit
# code and a diagnosis on stderr.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PS1="$SCRIPT_DIR/render_pptx.ps1"
PY="$SCRIPT_DIR/dump_pptx.py"

DECK=""; OUT=""; WIDTH=1600; HEIGHT=900; RENDER=1; DUMP=0; CHECK=0
TIMEOUT="${PPTX_TIMEOUT:-180}"
NET_TIMEOUT="${PPTX_NET_TIMEOUT:-90}"
while [ $# -gt 0 ]; do
  case "$1" in
    --out)         OUT="${2:-}";        shift 2;;
    --width)       WIDTH="${2:-}";      shift 2;;
    --height)      HEIGHT="${2:-}";     shift 2;;
    --timeout)     TIMEOUT="${2:-}";    shift 2;;
    --net-timeout) NET_TIMEOUT="${2:-}"; shift 2;;
    --no-render)   RENDER=0;            shift;;
    --dump)        DUMP=1;              shift;;
    --check)       CHECK=1;             shift;;
    -h|--help)     sed -n '2,26p' "$0"; exit 0;;
    -*)            echo "unknown option: $1" >&2; exit 2;;
    *)             DECK="$1";           shift;;
  esac
done

# The Windows PowerShell entry point.
#
# The bare name is not always enough: /etc/wsl.conf may set
# `[interop] appendWindowsPath = false`, which keeps every Windows executable off
# PATH even though interop itself is on (checked via the WSLInterop binfmt
# handler). Asking for the name first and falling back to the absolute path keeps
# both machines working, and returning "not found" is what lets the caller fail
# loudly instead of silently doing nothing.
resolve_powershell() {
  if [ -n "${PPTX_POWERSHELL:-}" ]; then printf '%s' "$PPTX_POWERSHELL"; return 0; fi
  for name in powershell.exe pwsh.exe; do
    if command -v "$name" >/dev/null 2>&1; then command -v "$name"; return 0; fi
  done
  for candidate in \
    /mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe \
    "/mnt/c/Program Files/PowerShell/7/pwsh.exe"; do
    if [ -x "$candidate" ]; then printf '%s' "$candidate"; return 0; fi
  done
  return 1
}

# The Windows path of a WSL path, which is what PowerShell needs.
# Windows PowerShell cannot open /home/... ; it reaches the distro over 9p.
to_windows_path() {
  local distro="${WSL_DISTRO_NAME:-Ubuntu}"
  if [[ "$1" == /* ]]; then
    printf '\\\\wsl.localhost\\%s%s' "$distro" "$(printf '%s' "$1" | tr '/' '\\')"
  else
    printf '%s' "$1"
  fi
}

# --- --check: say what this skill depends on, and how it looks right now ------
if [ "$CHECK" = "1" ]; then
  echo "distro=${WSL_DISTRO_NAME:-Ubuntu}"
  if PS_FOUND="$(resolve_powershell)"; then
    echo "powershell=$PS_FOUND"
  else
    echo "powershell=(not found)"
  fi
  if [ -e /proc/sys/fs/binfmt_misc/WSLInterop ]; then
    echo "interop=enabled ($(head -1 /proc/sys/fs/binfmt_misc/WSLInterop))"
  else
    echo "interop=missing (no WSLInterop binfmt handler: Windows programs cannot run)"
  fi
  echo "appendWindowsPath=$(awk -F= '/^\[interop\]/{f=1} f&&/appendWindowsPath/{gsub(/ /,"",$2);print $2;exit}' /etc/wsl.conf 2>/dev/null || true)"
  if command -v uv >/dev/null 2>&1; then echo "uv=$(uv --version)"; else echo "uv=(not found)"; fi
  # The COM question the old script could not answer: is PowerPoint alive, is
  # this run going to borrow the user's instance, and has an earlier run left a
  # hidden copy behind in it? Attaches, reads, releases -- never Quits, because
  # an instance this script did not start belongs to the user.
  if [ -n "${PS_FOUND:-}" ]; then
    PS_PROBE='try { $a = New-Object -ComObject PowerPoint.Application; Write-Output ("app_version=" + $a.Version); Write-Output ("app_visible=" + $a.Visible); Write-Output ("presentations=" + $a.Presentations.Count); foreach ($p in $a.Presentations) { Write-Output ("open=" + $p.Name + " saved=" + $p.Saved + " readonly=" + $p.ReadOnly) }; [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($a) } catch { Write-Output ("com_error=" + $_.Exception.Message) }'
    timeout 30 "$PS_FOUND" -NoProfile -NonInteractive -Command "$PS_PROBE" 2>&1 || echo "com_probe_exit=$?"
  fi
  exit 0
fi

if [ -z "$DECK" ]; then
  echo "usage: read_pptx.sh <deck.pptx> [--out DIR] [--no-render] [--dump] [--width N] [--height N] [--timeout SEC] [--check]" >&2
  exit 2
fi
if [ ! -f "$DECK" ]; then
  echo "not a file: $DECK" >&2
  exit 2
fi

# PowerShell is resolved before any work: a missing one is an error, not a
# silent "no text, no PNG" run. The two ways it can be missing are worth naming,
# because they need different fixes.
if ! PS_BIN="$(resolve_powershell)"; then
  cat >&2 <<'EOF'
error: no PowerShell found, so the PowerPoint COM path cannot run.
  - interop handler: /proc/sys/fs/binfmt_misc/WSLInterop
  - looked for: powershell.exe / pwsh.exe on PATH, then
    /mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe
  If interop is missing, enable it in /etc/wsl.conf. If it is on but the bare
  name is not found, `[interop] appendWindowsPath = false` is hiding it from
  PATH -- the absolute path above is the fix. A working path can also be given
  directly: PPTX_POWERSHELL=/mnt/c/.../powershell.exe
EOF
  exit 4
fi

DECK_ABS="$(realpath "$DECK")"
WIN_SRC="$(to_windows_path "$DECK_ABS")"

# Non-ASCII must not travel through the argv boundary: Windows PowerShell would
# receive it in the ANSI code page. base64(UTF-8) is pure ASCII and lossless.
SRC_B64="$(printf '%s' "$WIN_SRC" | base64 -w0)"
SLUG="$(printf '%s' "$WIN_SRC" | sha1sum | cut -c1-10)"

SCRIPT_WIN="$(to_windows_path "$PS1")"

# Bounded, and the exit status is kept: 124 is the timeout, and it means
# PowerPoint is holding a call that will not come back. Killing the wrapper here
# is deliberately the last resort -- the .ps1's own `finally` (which closes the
# copy it opened) does not run when this process is killed, so the next
# `--check` is the way to see whether a hidden copy was left behind.
RC=0
OUT_RAW="$(timeout "$TIMEOUT" "$PS_BIN" -NoProfile -NonInteractive -ExecutionPolicy Bypass \
  -File "$SCRIPT_WIN" \
  -SrcB64 "$SRC_B64" -Slug "$SLUG" \
  -Width "$WIDTH" -Height "$HEIGHT" -Render "$RENDER" 2>&1)" || RC=$?

# PowerShell mixes CLIXML progress noise into stderr; keep only our own lines.
# It also writes CRLF, and a trailing CR would ride along inside every parsed
# value below -- which silently breaks the paths this script hands back.
STATUS="$(printf '%s\n' "$OUT_RAW" | grep -a -E '^[A-Z_]+=' | tr -d '\r' || true)"
printf '%s\n' "$STATUS"

if [ "$RC" -eq 124 ]; then
  echo "error: the PowerPoint step did not return within ${TIMEOUT}s and was killed." >&2
  echo "  (last status: $(printf '%s\n' "$STATUS" | tail -1))" >&2
  echo "  PowerPoint was probably busy or showing a dialog. Check for a hidden copy" >&2
  echo "  of the deck with: $(basename "$0") --check" >&2
  exit 4
fi
if printf '%s\n' "$STATUS" | grep -qa '^STATUS=ERR_NO_SOURCE'; then
  echo "error: PowerPoint could not see $WIN_SRC" >&2
  exit 3
fi
if printf '%s\n' "$STATUS" | grep -qa '^STATUS=ERR_COM'; then
  echo "error: PowerPoint COM failed (is PowerPoint installed and not showing a dialog?)" >&2
  exit 4
fi
# The old failure mode: PowerShell never ran, so every field was empty and this
# script cheerfully continued and hung in `uv` on the original file. Say it.
if [ -z "$STATUS" ]; then
  echo "error: the PowerPoint step produced no status at all (exit $RC)." >&2
  printf '%s\n' "$OUT_RAW" | tail -20 | sed 's/^/  /' >&2
  exit 4
fi
if ! printf '%s\n' "$STATUS" | grep -qa '^SLIDES='; then
  echo "error: PowerPoint never reported SLIDES=; the run stopped early (exit $RC)." >&2
  printf '%s\n' "$STATUS" | sed 's/^/  /' >&2
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
  if ! command -v uv >/dev/null 2>&1; then
    echo "error: --dump needs uv, which is not on PATH." >&2
    exit 5
  fi
  CACHE_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)/.dsh.local/uv-cache"
  mkdir -p "$CACHE_DIR"
  # Read the COPY, never the original. The copy is what PowerPoint opened and the
  # only file this skill is allowed to read; falling back to the original (which
  # is what an empty COPY_WSL used to cause, silently) also risks reading a file
  # the user is mid-edit in.
  if [ -z "$COPY_WSL" ]; then
    echo "error: no private copy was reported, so --dump will not read the original." >&2
    exit 4
  fi
  # Bounded too: `uv` resolving against an unreachable package index waits
  # forever otherwise, which is the second half of this skill's hang.
  DUMP_RC=0
  UV_CACHE_DIR="$CACHE_DIR" timeout "$NET_TIMEOUT" uv run --no-project --with python-pptx -- \
    python "$PY" "$COPY_WSL" || DUMP_RC=$?
  if [ "$DUMP_RC" -eq 124 ]; then
    echo "error: uv did not finish within ${NET_TIMEOUT}s (package index unreachable?)." >&2
    exit 5
  elif [ "$DUMP_RC" -ne 0 ]; then
    echo "error: the python-pptx dump failed (exit $DUMP_RC)." >&2
    exit 5
  fi
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
