#!/usr/bin/env python3
import argparse, json, sys
from pathlib import Path


def norm(s: str) -> str:
    return ' '.join(s.split())


def remove_output(path: Path) -> None:
    try:
        path.unlink()
    except FileNotFoundError:
        pass


def fail(message: str, output: Path) -> None:
    remove_output(output)
    print(message, file=sys.stderr)
    raise SystemExit(2)


ap = argparse.ArgumentParser()
ap.add_argument('--source', required=True)
ap.add_argument('--input', required=True)
ap.add_argument('--output', required=True)
args = ap.parse_args()

source_path = Path(args.source)
input_path = Path(args.input)
output_path = Path(args.output)
remove_output(output_path)

if not source_path.is_file():
    fail('OCR source file is missing', output_path)
if not input_path.is_file():
    fail('OCR input file is missing', output_path)

try:
    text = input_path.read_text(encoding='utf-8')
except OSError:
    fail('OCR input file is unreadable', output_path)

normalized_text = norm(text)
if not normalized_text:
    fail('OCR input text is empty', output_path)

obj = {
    'source': '<redacted>',
    'extracted_text': text.strip(),
    'normalized_text': normalized_text,
    'ready_for_text_pii_eval': bool(normalized_text),
    'candidate': 'PP-OCRv5_mobile_rec',
    'engine': 'PP-OCRv5-mobile-bounded-spike',
    'engine_status': 'deterministic_synthetic_fixture_fallback',
    'scope': 'printed_text_line_extraction_only',
    'privacy_filter_contract': 'text_only_normalized_input',
    'non_goals': [
        'visual_redaction',
        'final_pdf_rewrite_export',
        'handwriting_recognition',
        'full_page_detection_or_segmentation',
        'complete_ocr_pipeline',
    ],
}
try:
    output_path.write_text(json.dumps(obj, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
except OSError:
    fail('OCR handoff output write failed', output_path)
print(args.output)
