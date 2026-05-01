#!/usr/bin/env python3
"""Bounded synthetic-fixture OCR runner for the PP-OCRv5 mobile spike.

This runner is intentionally scoped to printed text line extraction for local
spike plumbing. The candidate recognizer tracked by this slice is
PP-OCRv5_mobile_rec, but this code does not verify model weights or claim real
OCR quality. When PaddleOCR is absent, it fails honestly instead of silently
using fixture text unless --mock is explicitly requested.
"""

import argparse
import importlib
import json
import sys
from pathlib import Path
from typing import Iterable, Sequence

CANDIDATE_RECOGNIZER = "PP-OCRv5_mobile_rec"
ENGINE = "PP-OCRv5-mobile-bounded-spike"
MOCK_ENGINE_STATUS = "deterministic_synthetic_fixture_fallback"
REAL_ENGINE_STATUS = "local_paddleocr_execution"
SCOPE = "printed_text_line_extraction_only"
PRIVACY_FILTER_CONTRACT = "text_only_normalized_input"
REDACTED_SOURCE = "<redacted>"
NON_GOALS = sorted(
    {
        "visual_redaction",
        "final_pdf_rewrite_export",
        "handwriting_recognition",
        "full_page_detection_or_segmentation",
        "complete_ocr_pipeline",
    }
)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    parser.add_argument("input_path")
    parser.add_argument(
        "--mock",
        action="store_true",
        help="Use fixture-backed mock extraction for plumbing only",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit bounded OCR extraction contract JSON instead of raw text",
    )
    return parser


def validate_input_path(input_path: Path) -> None:
    if not input_path.exists():
        raise ValueError("OCR input path does not exist")
    if not input_path.is_file():
        raise ValueError("OCR input path must be a file")


def normalize_text(text: str) -> str:
    return " ".join(text.split())


def build_extraction_contract(extracted_text: str, engine_status: str) -> dict:
    normalized_text = normalize_text(extracted_text)
    return {
        "candidate": CANDIDATE_RECOGNIZER,
        "engine": ENGINE,
        "engine_status": engine_status,
        "scope": SCOPE,
        "source": REDACTED_SOURCE,
        "extracted_text": extracted_text,
        "normalized_text": normalized_text,
        "ready_for_text_pii_eval": bool(normalized_text),
        "privacy_filter_contract": PRIVACY_FILTER_CONTRACT,
        "non_goals": NON_GOALS,
    }


def emit_output(extracted_text: str, engine_status: str, json_output: bool) -> None:
    if json_output:
        sys.stdout.write(
            json.dumps(
                build_extraction_contract(extracted_text, engine_status),
                indent=2,
                sort_keys=True,
            )
        )
        sys.stdout.write("\n")
        return
    sys.stdout.write(extracted_text)


def read_mock_text(input_path: Path) -> str | None:
    expected = input_path.parent / "synthetic_printed_phi_expected.txt"
    if expected.exists():
        return expected.read_text(encoding="utf-8")
    print("mock expected text fixture not found", file=sys.stderr)
    return None


def run_mock(input_path: Path, json_output: bool) -> int:
    extracted_text = read_mock_text(input_path)
    if extracted_text is None:
        return 2
    emit_output(extracted_text, MOCK_ENGINE_STATUS, json_output)
    return 0


def load_paddleocr_class():
    try:
        module = importlib.import_module("paddleocr")
    except Exception:
        print(
            "PaddleOCR is not installed locally. Re-run with --mock for plumbing only, or install the OCR stack first.",
            file=sys.stderr,
        )
        return None
    return getattr(module, "PaddleOCR", None)


def create_engine(paddleocr_class):
    """Create a bounded PaddleOCR-like engine without forcing downloads in tests.

    Kwargs are deliberately conservative and document the intended mobile OCR
    candidate. PaddleOCR APIs vary across versions, so retry with no kwargs if a
    local installation rejects them.
    """
    kwargs = {
        "use_angle_cls": False,
        "lang": "en",
        "show_log": False,
        # Metadata/default candidate for this bounded spike; not verification.
        "rec_model_name": CANDIDATE_RECOGNIZER,
    }
    try:
        return paddleocr_class(**kwargs)
    except TypeError:
        return paddleocr_class()


def iter_text_fragments(node) -> Iterable[str]:
    if isinstance(node, str):
        yield node
        return
    if isinstance(node, tuple) and node and isinstance(node[0], str):
        yield node[0]
        return
    if isinstance(node, dict):
        for key in ("text", "transcription", "rec_text"):
            value = node.get(key)
            if isinstance(value, str):
                yield value
        rec_texts = node.get("rec_texts")
        if isinstance(rec_texts, list):
            for value in rec_texts:
                if isinstance(value, str):
                    yield value
        return
    if isinstance(node, (list, tuple)):
        for item in node:
            yield from iter_text_fragments(item)


def normalize_ocr_result(result) -> str:
    lines = [text for text in iter_text_fragments(result) if text]
    if not lines:
        return ""
    return "\n".join(lines) + "\n"


def run_real(input_path: Path, json_output: bool) -> int:
    paddleocr_class = load_paddleocr_class()
    if paddleocr_class is None:
        return 3

    engine = create_engine(paddleocr_class)
    result = engine.ocr(str(input_path))
    emit_output(normalize_ocr_result(result), REAL_ENGINE_STATUS, json_output)
    return 0


def main(argv: Sequence[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    input_path = Path(args.input_path)
    try:
        validate_input_path(input_path)
    except ValueError as error:
        print(str(error), file=sys.stderr)
        return 2
    if args.mock:
        return run_mock(input_path, args.json)
    return run_real(input_path, args.json)


if __name__ == "__main__":
    raise SystemExit(main())
