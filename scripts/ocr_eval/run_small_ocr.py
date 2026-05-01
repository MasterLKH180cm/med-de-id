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
import sys
from pathlib import Path
from typing import Iterable, Sequence

CANDIDATE_RECOGNIZER = "PP-OCRv5_mobile_rec"


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    parser.add_argument("input_path")
    parser.add_argument(
        "--mock",
        action="store_true",
        help="Use fixture-backed mock extraction for plumbing only",
    )
    return parser


def validate_input_path(input_path: Path) -> None:
    if not input_path.exists():
        raise ValueError("OCR input path does not exist")
    if not input_path.is_file():
        raise ValueError("OCR input path must be a file")


def run_mock(input_path: Path) -> int:
    expected = input_path.parent / "synthetic_printed_phi_expected.txt"
    if expected.exists():
        sys.stdout.write(expected.read_text(encoding="utf-8"))
        return 0
    print("mock expected text fixture not found", file=sys.stderr)
    return 2


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


def run_real(input_path: Path) -> int:
    paddleocr_class = load_paddleocr_class()
    if paddleocr_class is None:
        return 3

    engine = create_engine(paddleocr_class)
    result = engine.ocr(str(input_path))
    sys.stdout.write(normalize_ocr_result(result))
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
        return run_mock(input_path)
    return run_real(input_path)


if __name__ == "__main__":
    raise SystemExit(main())
