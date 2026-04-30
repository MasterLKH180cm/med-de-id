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
normalized_text = norm(text)
obj = {
    'source': Path(args.source).name,
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
Path(args.output).write_text(json.dumps(obj, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(args.output)
