import json
import sys
import io
import contextlib
import os
import shutil
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
PROJECT_ROOT = REPO_ROOT / 'OrderPulse'
PYTHON_PACKAGE_ROOT = PROJECT_ROOT / 'python'

if sys.version_info < (3, 12):
    python312 = shutil.which('python3.12')
    if python312:
        os.execv(python312, [python312, __file__])
    raise RuntimeError('python3.12 is required to run this notebook with the local fastreader build')

if str(PYTHON_PACKAGE_ROOT) not in sys.path:
    sys.path.insert(0, str(PYTHON_PACKAGE_ROOT))

path = PROJECT_ROOT / 'test.ipynb'
with open(path, 'r', encoding='utf-8') as f:
    nb = json.load(f)

shared_globals = {}

for i, cell in enumerate(nb.get('cells', [])):
    if cell.get('cell_type') == 'code':
        source = "".join(cell.get('source', []))
        print(f"Cell {i+1}:")
        stdout = io.StringIO()
        stderr = io.StringIO()
        try:
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                exec(source, shared_globals)
            print('Status: ok')
            out = stdout.getvalue()
            err = stderr.getvalue()
            if out:
                print(out.strip())
            if err:
                print(f"Error output:\n{err.strip()}")
        except Exception as e:
            print('Status: error')
            print(f"Execution failed: {e}")
            # Optional: print traceback if needed, but keeping it concise
