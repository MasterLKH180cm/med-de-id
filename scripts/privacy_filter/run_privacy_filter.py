#!/usr/bin/env python3
import argparse, json, re, shutil, subprocess, sys
from pathlib import Path

EMAIL_RE = re.compile(r'[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}')
PHONE_RE = re.compile(r'(?<!\d)(?:\+\d{1,3}-)?\d{3}-\d{3}-\d{4}(?!\d)')
MRN_RE = re.compile(r'\bMRN[- ]?\d+\b', re.I)
ID_RE = re.compile(r'\bID[- ]?\d+\b', re.I)
PERSON_RE = re.compile(r'\bPatient\s+([A-Z][a-z]+\s+[A-Z][a-z]+)')
OPF_TIMEOUT_SECONDS = 15
OPF_OUTPUT_MAX_BYTES = 1024 * 1024
STDIN_INPUT_MAX_BYTES = 1024 * 1024


def add_span(spans, label, start, end):
    spans.append({'label': label, 'start': start, 'end': end, 'preview': '<redacted>'})


def heuristic_detect(text: str):
    spans = []
    for m in PERSON_RE.finditer(text):
        add_span(spans, 'NAME', m.start(1), m.end(1))
    for m in EMAIL_RE.finditer(text):
        add_span(spans, 'EMAIL', m.start(), m.end())
    for m in PHONE_RE.finditer(text):
        add_span(spans, 'PHONE', m.start(), m.end())
    for m in MRN_RE.finditer(text):
        add_span(spans, 'MRN', m.start(), m.end())
    for m in ID_RE.finditer(text):
        add_span(spans, 'ID', m.start(), m.end())
    spans.sort(key=lambda s: (s['start'], s['end']))
    counts = {}
    masked_parts = []
    pos = 0
    for s in spans:
        counts[s['label']] = counts.get(s['label'], 0) + 1
        if s['start'] < pos:
            continue
        masked_parts.append(text[pos:s['start']])
        masked_parts.append(f'[{s["label"]}]')
        pos = s['end']
    masked_parts.append(text[pos:])
    return {
        'summary': {
            'input_char_count': len(text),
            'detected_span_count': len(spans),
            'category_counts': counts,
        },
        'masked_text': ''.join(masked_parts),
        'spans': spans,
        'metadata': fallback_metadata(),
    }


def fallback_metadata():
    return {
        'engine': 'fallback_synthetic_patterns',
        'network_api_called': False,
        'preview_policy': 'redacted_placeholders_only',
    }


def real_opf_metadata():
    return {
        'engine': 'openai_privacy_filter_opf',
        'network_api_called': False,
        'preview_policy': 'redacted_placeholders_only',
    }


def _coerce_int(value, default=0):
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def _span_label(span):
    if not isinstance(span, dict):
        return 'UNKNOWN'
    return str(span.get('label') or span.get('type') or span.get('category') or 'UNKNOWN')


def _span_start(span):
    if not isinstance(span, dict):
        return 0
    return _coerce_int(span.get('start', span.get('begin', 0)))


def _span_end(span):
    if not isinstance(span, dict):
        return 0
    return _coerce_int(span.get('end', span.get('finish', span.get('stop', 0))))


def normalize_opf_json(raw: str, input_char_count: int):
    obj = json.loads(raw)
    if not isinstance(obj, dict):
        obj = {}
    raw_spans = obj.get('spans')
    if not isinstance(raw_spans, list):
        raw_spans = obj.get('entities')
    if not isinstance(raw_spans, list):
        raw_spans = []

    spans = [
        {
            'label': _span_label(span),
            'start': _span_start(span),
            'end': _span_end(span),
            'preview': '<redacted>',
        }
        for span in raw_spans
    ]
    spans.sort(key=lambda span: (span['start'], span['end'], span['label']))

    counts = {}
    for span in spans:
        counts[span['label']] = counts.get(span['label'], 0) + 1
    category_counts = {key: counts[key] for key in sorted(counts)}

    masked_text = obj.get('masked_text', obj.get('text', '<missing>'))
    if not isinstance(masked_text, str):
        masked_text = '<missing>'

    return {
        'summary': {
            'input_char_count': input_char_count,
            'detected_span_count': len(spans),
            'category_counts': category_counts,
        },
        'masked_text': masked_text,
        'spans': spans,
        'metadata': real_opf_metadata(),
    }


def run_opf_with_stdin(opf: str, text: str) -> str:
    proc = subprocess.Popen(
        [opf, '--format', 'json'],
        text=True,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    try:
        stdout, stderr = proc.communicate(input=text, timeout=OPF_TIMEOUT_SECONDS)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.communicate()
        print('opf timed out; run with --mock or inspect local opf configuration.', file=sys.stderr)
        raise SystemExit(3)
    if len(stdout.encode('utf-8', errors='replace')) > OPF_OUTPUT_MAX_BYTES or len(stderr.encode('utf-8', errors='replace')) > OPF_OUTPUT_MAX_BYTES:
        print('opf output exceeded limit; run with --mock or inspect local opf configuration.', file=sys.stderr)
        raise SystemExit(3)
    if proc.returncode != 0:
        print('opf failed; run with --mock or inspect local opf configuration.', file=sys.stderr)
        raise SystemExit(proc.returncode or 3)
    return stdout


def read_bounded_stdin_text() -> str:
    stdin_buffer = getattr(sys.stdin, 'buffer', None)
    if stdin_buffer is not None:
        data = stdin_buffer.read(STDIN_INPUT_MAX_BYTES + 1)
        if len(data) > STDIN_INPUT_MAX_BYTES:
            print('stdin input exceeds 1048576 byte limit', file=sys.stderr)
            raise SystemExit(2)
        try:
            return data.decode('utf-8')
        except UnicodeDecodeError:
            print('stdin input must be valid UTF-8 text', file=sys.stderr)
            raise SystemExit(2)

    text = sys.stdin.read(STDIN_INPUT_MAX_BYTES + 1)
    if len(text.encode('utf-8')) > STDIN_INPUT_MAX_BYTES:
        print('stdin input exceeds 1048576 byte limit', file=sys.stderr)
        raise SystemExit(2)
    return text


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('input_path', nargs='?')
    ap.add_argument('--stdin', action='store_true', help='Read UTF-8 text from stdin instead of a file path')
    ap.add_argument('--mock', action='store_true', help='Use bounded heuristic/mock detection for contract plumbing only')
    ap.add_argument('--use-opf', action='store_true', help='Explicitly invoke local opf via stdin; ambient opf auto-use is disabled')
    args = ap.parse_args()
    if (args.input_path is None) == (not args.stdin):
        ap.error('exactly one input source is required')
    if args.stdin:
        text = read_bounded_stdin_text()
    else:
        text = Path(args.input_path).read_text(encoding='utf-8')
    if args.mock or not args.use_opf:
        print(json.dumps(heuristic_detect(text), ensure_ascii=False, indent=2))
        return
    opf = shutil.which('opf')
    if opf is None:
        print(json.dumps(heuristic_detect(text), ensure_ascii=False, indent=2))
        return
    raw = run_opf_with_stdin(opf, text)
    try:
        output = normalize_opf_json(raw, len(text))
    except Exception:
        print('opf returned non-JSON output; run with --mock or adapt parser to actual local opf version.', file=sys.stderr)
        raise SystemExit(4)
    print(json.dumps(output, ensure_ascii=False, indent=2))

if __name__ == '__main__':
    main()
