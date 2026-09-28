"""Launch a command with a requested per-thread SVE VL, inherited across exec."""
import ctypes
import os
import sys

vl = int(sys.argv[1])
libc = ctypes.CDLL(None, use_errno=True)
actual = libc.prctl(50, vl | (1 << 17), 0, 0, 0)
if actual < 0:
    raise OSError(ctypes.get_errno(), "PR_SVE_SET_VL failed")
if (actual & 0xffff) != vl:
    raise SystemExit(f"Requested VL={vl}, kernel selected {actual & 0xffff}")
os.execvp(sys.argv[2], sys.argv[2:])
