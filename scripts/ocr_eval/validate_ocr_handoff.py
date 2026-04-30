#!/usr/bin/env python3
import json, sys
from pathlib import Path

REQUIRED_KEYS = {'source', 'extracted_text', 'normalized_text', 'ready_for_text_pii_eval'}

def fail(msg: str):
    print(msg, file=sys.stderr)
    raise SystemExit(1)

obj = json.loads(Path(sys.argv[1]).read_text(encoding='utf-8'))
if not isinstance(obj, dict):
    fail('handoff must be a JSON object')
missing = REQUIRED_KEYS - set(obj)
if missing:
    fail(f'missing handoff keys: {sorted(missing)}')
if not isinstance(obj['source'], str) or not obj['source']:
    fail('source must be non-empty string')
if not isinstance(obj['extracted_text'], str):
    fail('extracted_text must be string')
if not isinstance(obj['normalized_text'], str):
    fail('normalized_text must be string')
if not isinstance(obj['ready_for_text_pii_eval'], bool):
    fail('ready_for_text_pii_eval must be bool')
print('ocr handoff contract OK')
