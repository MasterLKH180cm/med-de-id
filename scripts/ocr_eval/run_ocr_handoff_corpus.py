#!/usr/bin/env python3
"""Build a PHI-safe aggregate report for synthetic OCR handoff text fixtures."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ENGINE = "PP-OCRv5-mobile-bounded-spike"
SCOPE = "printed_text_line_extraction_only"
PRIVACY_FILTER_CONTRACT = "text_only_normalized_input"
MAX_FIXTURE_COUNT = 100
MAX_FIXTURE_BYTES = 1024 * 1024
NON_GOALS = sorted(
    {
        "visual_redaction",
        "final_pdf_rewrite_export",
        "handwriting_recognition",
        "full_page_detection_or_segmentation",
        "complete_ocr_pipeline",
    }
)


def normalize_whitespace(text: str) -> str:
    return " ".join(text.split())


def remove_stale_output(output: Path) -> None:
    try:
        if output.exists():
            output.unlink()
    except OSError:
        pass


def fail(output: Path, message: str) -> int:
    remove_stale_output(output)
    print(message, file=sys.stderr)
    return 1


def fixture_id(index: int) -> str:
    return f"fixture_{index:03d}"


def read_fixture_text(fixture_path: Path, index: int, output: Path) -> tuple[str | None, int | None]:
    try:
        with fixture_path.open("rb") as handle:
            data = handle.read(MAX_FIXTURE_BYTES + 1)
    except OSError:
        fail(output, f"{fixture_id(index)} could not be read")
        return None, 1

    if len(data) > MAX_FIXTURE_BYTES:
        fail(output, f"{fixture_id(index)} exceeds maximum fixture size")
        return None, 1

    try:
        return data.decode("utf-8"), None
    except UnicodeDecodeError:
        fail(output, f"{fixture_id(index)} is not valid UTF-8")
        return None, 1


def build_report(fixture_dir: Path, output: Path) -> int:
    remove_stale_output(output)

    try:
        fixture_dir_exists = fixture_dir.is_dir()
    except OSError:
        fixture_dir_exists = False
    if not fixture_dir_exists:
        return fail(output, "fixture dir does not exist or is not a directory")

    try:
        fixture_paths = sorted(fixture_dir.glob("*.txt"))
    except OSError:
        return fail(output, "fixture dir could not be read")
    if not fixture_paths:
        return fail(output, "fixture dir contains no .txt fixtures")
    if len(fixture_paths) > MAX_FIXTURE_COUNT:
        return fail(output, "fixture corpus exceeds maximum fixture count")

    fixtures = []
    total_char_count = 0
    for index, fixture_path in enumerate(fixture_paths, start=1):
        text, error_code = read_fixture_text(fixture_path, index, output)
        if error_code is not None:
            return error_code
        assert text is not None
        normalized = normalize_whitespace(text)
        if not normalized:
            return fail(output, f"{fixture_id(index)} is empty after whitespace normalization")
        char_count = len(normalized)
        total_char_count += char_count
        fixtures.append(
            {
                "id": fixture_id(index),
                "char_count": char_count,
                "ready_for_text_pii_eval": True,
            }
        )

    report = {
        "engine": ENGINE,
        "scope": SCOPE,
        "fixture_count": len(fixtures),
        "ready_fixture_count": sum(1 for fixture in fixtures if fixture["ready_for_text_pii_eval"]),
        "total_char_count": total_char_count,
        "fixtures": fixtures,
        "non_goals": NON_GOALS,
        "privacy_filter_contract": PRIVACY_FILTER_CONTRACT,
    }

    try:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    except OSError:
        return fail(output, "report output could not be written")
    return 0


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture-dir", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    return build_report(args.fixture_dir, args.output)


if __name__ == "__main__":
    raise SystemExit(main())
