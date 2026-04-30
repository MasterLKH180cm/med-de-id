#!/usr/bin/env python3
import argparse, json
from pathlib import Path

def norm(s: str) -> str:
    return ' '.join(s.split())

ap = argparse.ArgumentParser()
ap.add_argument('--source', required=True)
ap.add_argument('--input', required=True)
ap.add_argument('--output', required=True)
args = ap.parse_args()
text = Path(args.input).read_text(encoding='utf-8')
obj = {
    'source': Path(args.source).name,
    'extracted_text': text.strip(),
    'normalized_text': norm(text),
    'ready_for_text_pii_eval': bool(norm(text)),
}
Path(args.output).write_text(json.dumps(obj, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(args.output)
