#!/usr/bin/env python3
import argparse, json, sys
from pathlib import Path

REQUIRED_KEYS = {
    'source',
    'extracted_text',
    'normalized_text',
    'ready_for_text_pii_eval',
    'candidate',
    'engine',
    'engine_status',
    'scope',
    'privacy_filter_contract',
    'non_goals',
}
EXPECTED_NON_GOALS = {
    'visual_redaction',
    'final_pdf_rewrite_export',
    'handwriting_recognition',
    'full_page_detection_or_segmentation',
    'complete_ocr_pipeline',
}

def fail(msg: str):
    print(msg, file=sys.stderr)
    raise SystemExit(1)

def norm(s: str) -> str:
    return ' '.join(s.split())

ap = argparse.ArgumentParser(description='Validate OCR handoff contract JSON')
ap.add_argument('path', help='path to OCR handoff JSON')
args = ap.parse_args()

obj = json.loads(Path(args.path).read_text(encoding='utf-8'))
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
if obj['normalized_text'] != norm(obj['extracted_text']):
    fail('normalized_text must equal whitespace-normalized extracted_text')
if obj['ready_for_text_pii_eval'] is not bool(obj['normalized_text']):
    fail('ready_for_text_pii_eval must equal bool(normalized_text)')
if obj['candidate'] != 'PP-OCRv5_mobile_rec':
    fail('candidate must be PP-OCRv5_mobile_rec')
if obj['engine'] != 'PP-OCRv5-mobile-bounded-spike':
    fail('engine must be PP-OCRv5-mobile-bounded-spike')
if obj['engine_status'] not in {'deterministic_synthetic_fixture_fallback', 'ppocrv5_local_inference'}:
    fail('engine_status must truthfully identify fallback or local inference')
if obj['scope'] != 'printed_text_line_extraction_only':
    fail('scope must be printed_text_line_extraction_only')
if obj['privacy_filter_contract'] != 'text_only_normalized_input':
    fail('privacy_filter_contract must be text_only_normalized_input')
if not isinstance(obj['non_goals'], list) or not all(isinstance(item, str) for item in obj['non_goals']):
    fail('non_goals must be a list of strings')
missing_non_goals = EXPECTED_NON_GOALS - set(obj['non_goals'])
if missing_non_goals:
    fail(f'missing required non_goals: {sorted(missing_non_goals)}')
print('ocr handoff contract OK')
