import json
from pathlib import Path

path = Path(__file__).resolve().parent.parent / 'OrderPulse' / 'test.ipynb'
try:
    with open(path, 'r', encoding='utf-8') as f:
        nb = json.load(f)
    for i, cell in enumerate(nb.get('cells', [])):
        if cell.get('cell_type') == 'code':
            print(f"--- Cell {i+1} ---")
            print("".join(cell.get('source', [])))
except Exception as e:
    print(e)
