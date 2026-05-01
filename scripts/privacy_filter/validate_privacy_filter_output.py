#!/usr/bin/env python3
import json, sys
from pathlib import Path

ALLOWED_PREVIEW_VALUES = {'<redacted>'}
ALLOWED_LABELS = {'NAME', 'MRN', 'EMAIL', 'PHONE', 'ID', 'DATE', 'ADDRESS', 'SSN', 'PASSPORT', 'ZIP'}


class PrivacyFilterOutputValidationError(ValueError):
    pass


def load(path: str):
    text = Path(path).read_text(encoding='utf-8') if path != '-' else sys.stdin.read()
    return json.loads(text)


def _ensure(condition: bool, msg: str) -> None:
    if not condition:
        raise PrivacyFilterOutputValidationError(msg)


def validate_privacy_filter_output(obj) -> None:
    _ensure(isinstance(obj, dict), 'top-level JSON must be an object')
    for key in ['summary', 'masked_text', 'spans', 'metadata']:
        _ensure(key in obj, f'missing top-level key: {key}')
    _ensure(isinstance(obj['summary'], dict), 'summary must be an object')
    _ensure(isinstance(obj['metadata'], dict), 'metadata must be an object')
    for key in ['engine', 'network_api_called', 'preview_policy']:
        _ensure(key in obj['metadata'], f'missing metadata key: {key}')
    _ensure(isinstance(obj['metadata']['engine'], str) and bool(obj['metadata']['engine']), 'metadata.engine must be a non-empty string')
    _ensure(obj['metadata']['network_api_called'] is False, 'metadata.network_api_called must be false for local POC')
    _ensure(isinstance(obj['metadata']['preview_policy'], str) and bool(obj['metadata']['preview_policy']), 'metadata.preview_policy must be a non-empty string')
    for key in ['input_char_count', 'detected_span_count', 'category_counts']:
        _ensure(key in obj['summary'], f'missing summary key: {key}')
    _ensure(isinstance(obj['summary']['input_char_count'], int) and obj['summary']['input_char_count'] >= 0, 'summary.input_char_count must be a non-negative int')
    _ensure(isinstance(obj['summary']['detected_span_count'], int) and obj['summary']['detected_span_count'] >= 0, 'summary.detected_span_count must be a non-negative int')
    _ensure(isinstance(obj['summary']['category_counts'], dict), 'summary.category_counts must be an object')
    _ensure(isinstance(obj['masked_text'], str) and bool(obj['masked_text'].strip()), 'masked_text must be a non-empty string')
    _ensure(obj['masked_text'] != '<masked-text>', 'masked_text placeholder is not allowed')
    _ensure(isinstance(obj['spans'], list), 'spans must be a list')
    counts = {}
    prev_end = -1
    for i, span in enumerate(obj['spans']):
        _ensure(isinstance(span, dict), f'span {i} must be an object')
        for key in ['label', 'start', 'end', 'preview']:
            _ensure(key in span, f'span {i} missing key: {key}')
        _ensure(isinstance(span['label'], str) and bool(span['label']), f'span {i} label must be non-empty string')
        _ensure(span['label'] in ALLOWED_LABELS, f"span {i} label must be one of {sorted(ALLOWED_LABELS)}")
        _ensure(isinstance(span['start'], int) and isinstance(span['end'], int), f'span {i} start/end must be ints')
        _ensure(span['start'] >= 0 and span['end'] > span['start'], f'span {i} must have 0 <= start < end')
        _ensure(span['start'] >= prev_end, f'span {i} overlaps or is unsorted')
        prev_end = span['end']
        _ensure(isinstance(span['preview'], str), f'span {i} preview must be string')
        _ensure(span['preview'] in ALLOWED_PREVIEW_VALUES, f'span {i} preview must be one of {sorted(ALLOWED_PREVIEW_VALUES)}')
        counts[span['label']] = counts.get(span['label'], 0) + 1
    _ensure(obj['summary']['detected_span_count'] == len(obj['spans']), 'summary.detected_span_count does not match number of spans')
    _ensure(obj['summary']['category_counts'] == counts, 'summary.category_counts does not match span labels')


def fail(msg: str):
    print(msg, file=sys.stderr)
    raise SystemExit(1)


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    try:
        obj = load(argv[0] if argv else '-')
        validate_privacy_filter_output(obj)
    except (json.JSONDecodeError, OSError, PrivacyFilterOutputValidationError) as exc:
        fail(str(exc))
    print('privacy-filter output contract OK')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
