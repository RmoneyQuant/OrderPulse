"""
FeedPathBuilder — step-through debug script.

Run via VS Code "Python: Debug FeedPathBuilder" launch config,
or directly: python debug_feedpath.py

Breakpoints are placed at every logical stage:
  BP1  — builder created
  BP2  — build() returns a path (no disk check)
  BP3  — disk existence confirmed for that path
  BP4  — build_and_verify() round-trip
  BP5  — short alias ("CM" / "FO") resolves correctly
  BP6  — custom base_path override
  BP7  — invalid inputs raise RuntimeError
"""

import os
import sys
from pathlib import Path

# ── make sure we use the venv interpreter ────────────────────────────────────
REPO_ROOT = Path(__file__).resolve().parent.parent
VENV_PYTHON = str(REPO_ROOT / "OrderPulse" / ".venv" / "bin" / "python")
if sys.executable != VENV_PYTHON and os.path.exists(VENV_PYTHON):
    os.execv(VENV_PYTHON, [VENV_PYTHON] + sys.argv)

from fastreader import FeedPathBuilder  # noqa: E402

# ─────────────────────────────────────────────────────────────────────────────
# BP1  builder created
# Inspect: b  →  FeedPathBuilder()
# ─────────────────────────────────────────────────────────────────────────────
b = FeedPathBuilder()
breakpoint()  # BP1 ── builder object

# ─────────────────────────────────────────────────────────────────────────────
# BP2  build() — NSE_CM stream 2, 29 Dec 2025
# Inspect: path  →  /nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin
# ─────────────────────────────────────────────────────────────────────────────
path = b.build("NSE_CM", stream_id=2, day=29, month=12, year=2025)
breakpoint()  # BP2 ── path string from build()

# ─────────────────────────────────────────────────────────────────────────────
# BP3  disk existence check
# Inspect: exists  →  True
# ─────────────────────────────────────────────────────────────────────────────
exists = os.path.exists(path)
breakpoint()  # BP3 ── exists on disk?

# ─────────────────────────────────────────────────────────────────────────────
# BP4  build_and_verify — should return same path
# Inspect: verified_path  →  same as `path`
# ─────────────────────────────────────────────────────────────────────────────
verified_path = b.build_and_verify("NSE_CM", stream_id=2, day=29, month=12, year=2025)
breakpoint()  # BP4 ── verified path

# ─────────────────────────────────────────────────────────────────────────────
# BP5  short alias "FO" for NSE_FO
# Inspect: fo_path  →  /nas/50.30/NSE_FO/Feed_FO_StreamID_10_01_01_2026.bin
# ─────────────────────────────────────────────────────────────────────────────
fo_path = b.build("FO", stream_id=10, day=1, month=1, year=2026)
fo_exists = os.path.exists(fo_path)
breakpoint()  # BP5 ── FO alias + disk check

# ─────────────────────────────────────────────────────────────────────────────
# BP6  custom base_path
# Inspect: custom_path  →  /mnt/archive/NSE_CM/Feed_CM_StreamID_1_01_05_2026.bin
# ─────────────────────────────────────────────────────────────────────────────
custom_path = b.build("NSE_CM", stream_id=1, day=1, month=5, year=2026,
                      base_path="/mnt/archive")
breakpoint()  # BP6 ── custom base path

# ─────────────────────────────────────────────────────────────────────────────
# BP7  invalid inputs — collect all errors for inspection
# Inspect: errors  →  list of (label, RuntimeError message)
# ─────────────────────────────────────────────────────────────────────────────
bad_cases = [
    ("bad segment",   dict(segment="INVALID", stream_id=1, day=1, month=1, year=2026)),
    ("stream_id=0",   dict(segment="NSE_CM",  stream_id=0, day=1, month=1, year=2026)),
    ("month=13",      dict(segment="NSE_CM",  stream_id=1, day=1, month=13, year=2026)),
    ("day=0",         dict(segment="NSE_CM",  stream_id=1, day=0, month=1, year=2026)),
    ("year=1999",     dict(segment="NSE_CM",  stream_id=1, day=1, month=1, year=1999)),
]

errors = []
for label, kw in bad_cases:
    try:
        b.build(**kw)
        errors.append((label, None))          # should not reach here
    except RuntimeError as e:
        errors.append((label, str(e)))

breakpoint()  # BP7 ── inspect `errors` list

# ─────────────────────────────────────────────────────────────────────────────
# Done — print summary
# ─────────────────────────────────────────────────────────────────────────────
print("\n── Summary ──────────────────────────────────────────────────")
print(f"CM  path : {path}")
print(f"     on disk : {exists}")
print(f"FO  path : {fo_path}")
print(f"     on disk : {fo_exists}")
print(f"custom   : {custom_path}")
print(f"verified : {verified_path}")
print("\nValidation errors captured:")
for label, msg in errors:
    status = "OK (raised)" if msg else "MISS (no error!)"
    print(f"  [{status}] {label}: {msg}")
