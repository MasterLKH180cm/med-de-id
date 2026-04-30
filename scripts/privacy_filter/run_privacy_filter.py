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


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('input_path')
    ap.add_argument('--mock', action='store_true', help='Use bounded heuristic/mock detection for contract plumbing only')
    ap.add_argument('--use-opf', action='store_true', help='Explicitly invoke local opf via stdin; ambient opf auto-use is disabled')
    args = ap.parse_args()
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
        obj = json.loads(raw)
    except Exception:
        print('opf returned non-JSON output; run with --mock or adapt parser to actual local opf version.', file=sys.stderr)
        raise SystemExit(4)
    print(json.dumps({
        'summary': {
            'input_char_count': len(text),
            'detected_span_count': len(obj.get('spans', [])) if isinstance(obj, dict) else 0,
            'category_counts': {
                str(s.get('label', 'UNKNOWN')): sum(1 for x in obj.get('spans', []) if str(x.get('label', 'UNKNOWN')) == str(s.get('label', 'UNKNOWN')))
                for s in obj.get('spans', [])
            } if isinstance(obj, dict) else {},
        },
        'masked_text': obj.get('masked_text', '<missing>') if isinstance(obj, dict) else '<missing>',
        'spans': [
            {
                'label': str(s.get('label', 'UNKNOWN')),
                'start': int(s.get('start', 0)),
                'end': int(s.get('end', 0)),
                'preview': '<redacted>'
            }
            for s in obj.get('spans', [])
        ] if isinstance(obj, dict) else [],
        'metadata': real_opf_metadata(),
    }, ensure_ascii=False, indent=2))

if __name__ == '__main__':
    main()
