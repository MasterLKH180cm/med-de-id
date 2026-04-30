#!/usr/bin/env python3
import json, sys
from pathlib import Path

ALLOWED_PREVIEW_VALUES = {'<redacted>'}

def load(path: str):
    text = Path(path).read_text(encoding='utf-8') if path != '-' else sys.stdin.read()
    return json.loads(text)

def fail(msg: str):
    print(msg, file=sys.stderr)
    raise SystemExit(1)

obj = load(sys.argv[1] if len(sys.argv) > 1 else '-')
if not isinstance(obj, dict):
    fail('top-level JSON must be an object')
for key in ['summary', 'masked_text', 'spans']:
    if key not in obj:
        fail(f'missing top-level key: {key}')
if not isinstance(obj['summary'], dict):
    fail('summary must be an object')
for key in ['input_char_count', 'detected_span_count', 'category_counts']:
    if key not in obj['summary']:
        fail(f'missing summary key: {key}')
if not isinstance(obj['summary']['input_char_count'], int) or obj['summary']['input_char_count'] < 0:
    fail('summary.input_char_count must be a non-negative int')
if not isinstance(obj['summary']['detected_span_count'], int) or obj['summary']['detected_span_count'] < 0:
    fail('summary.detected_span_count must be a non-negative int')
if not isinstance(obj['summary']['category_counts'], dict):
    fail('summary.category_counts must be an object')
if not isinstance(obj['masked_text'], str) or not obj['masked_text'].strip():
    fail('masked_text must be a non-empty string')
if obj['masked_text'] == '<masked-text>':
    fail('masked_text placeholder is not allowed')
if not isinstance(obj['spans'], list):
    fail('spans must be a list')
counts = {}
prev_end = -1
for i, span in enumerate(obj['spans']):
    if not isinstance(span, dict):
        fail(f'span {i} must be an object')
    for key in ['label', 'start', 'end', 'preview']:
        if key not in span:
            fail(f'span {i} missing key: {key}')
    if not isinstance(span['label'], str) or not span['label']:
        fail(f'span {i} label must be non-empty string')
    if not isinstance(span['start'], int) or not isinstance(span['end'], int):
        fail(f'span {i} start/end must be ints')
    if span['start'] < 0 or span['end'] <= span['start']:
        fail(f'span {i} must have 0 <= start < end')
    if span['start'] < prev_end:
        fail(f'span {i} overlaps or is unsorted')
    prev_end = span['end']
    if not isinstance(span['preview'], str):
        fail(f'span {i} preview must be string')
    if span['preview'] not in ALLOWED_PREVIEW_VALUES:
        fail(f'span {i} preview must be one of {sorted(ALLOWED_PREVIEW_VALUES)}')
    counts[span['label']] = counts.get(span['label'], 0) + 1
if obj['summary']['detected_span_count'] != len(obj['spans']):
    fail('summary.detected_span_count does not match number of spans')
if obj['summary']['category_counts'] != counts:
    fail('summary.category_counts does not match span labels')
print('privacy-filter output contract OK')
