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
    if output.exists():
        output.unlink()


def fail(output: Path, message: str) -> int:
    remove_stale_output(output)
    print(message, file=sys.stderr)
    return 1


def build_report(fixture_dir: Path, output: Path) -> int:
    remove_stale_output(output)

    if not fixture_dir.is_dir():
        return fail(output, f"fixture dir does not exist or is not a directory: {fixture_dir}")

    fixture_paths = sorted(fixture_dir.glob("*.txt"))
    if not fixture_paths:
        return fail(output, f"fixture dir contains no .txt fixtures: {fixture_dir}")

    fixtures = []
    total_char_count = 0
    for index, fixture_path in enumerate(fixture_paths, start=1):
        normalized = normalize_whitespace(fixture_path.read_text(encoding="utf-8"))
        if not normalized:
            return fail(output, f"fixture is empty after whitespace normalization: {fixture_path}")
        char_count = len(normalized)
        total_char_count += char_count
        fixtures.append(
            {
                "id": f"fixture_{index:03d}",
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

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
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
