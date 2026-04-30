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
CANDIDATE = "PP-OCRv5_mobile_rec"
SUMMARY_ARTIFACT = "ocr_handoff_corpus_readiness_summary"
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


def fail(output: Path, message: str, summary_output: Path | None = None) -> int:
    remove_stale_output(output)
    if summary_output is not None:
        remove_stale_output(summary_output)
    print(message, file=sys.stderr)
    return 1


def fixture_id(index: int) -> str:
    return f"fixture_{index:03d}"


def read_fixture_text(
    fixture_path: Path, index: int, output: Path, summary_output: Path | None = None
) -> tuple[str | None, int | None]:
    try:
        with fixture_path.open("rb") as handle:
            data = handle.read(MAX_FIXTURE_BYTES + 1)
    except OSError:
        fail(output, f"{fixture_id(index)} could not be read", summary_output)
        return None, 1

    if len(data) > MAX_FIXTURE_BYTES:
        fail(output, f"{fixture_id(index)} exceeds maximum fixture size", summary_output)
        return None, 1

    try:
        return data.decode("utf-8"), None
    except UnicodeDecodeError:
        fail(output, f"{fixture_id(index)} is not valid UTF-8", summary_output)
        return None, 1


def build_summary(report: dict[str, object]) -> dict[str, object]:
    return {
        "artifact": SUMMARY_ARTIFACT,
        "candidate": CANDIDATE,
        "engine": ENGINE,
        "scope": SCOPE,
        "privacy_filter_contract": PRIVACY_FILTER_CONTRACT,
        "fixture_count": report["fixture_count"],
        "ready_fixture_count": report["ready_fixture_count"],
        "all_fixtures_ready_for_text_pii_eval": report["fixture_count"] == report["ready_fixture_count"],
        "total_char_count": report["total_char_count"],
        "non_goals": NON_GOALS,
    }


def build_report(fixture_dir: Path, output: Path, summary_output: Path | None = None) -> int:
    remove_stale_output(output)
    if summary_output is not None:
        remove_stale_output(summary_output)

    try:
        fixture_dir_exists = fixture_dir.is_dir()
    except OSError:
        fixture_dir_exists = False
    if not fixture_dir_exists:
        return fail(output, "fixture dir does not exist or is not a directory", summary_output)

    try:
        fixture_paths = sorted(fixture_dir.glob("*.txt"))
    except OSError:
        return fail(output, "fixture dir could not be read", summary_output)
    if not fixture_paths:
        return fail(output, "fixture dir contains no .txt fixtures", summary_output)
    if len(fixture_paths) > MAX_FIXTURE_COUNT:
        return fail(output, "fixture corpus exceeds maximum fixture count", summary_output)

    fixtures = []
    total_char_count = 0
    for index, fixture_path in enumerate(fixture_paths, start=1):
        text, error_code = read_fixture_text(fixture_path, index, output, summary_output)
        if error_code is not None:
            return error_code
        assert text is not None
        normalized = normalize_whitespace(text)
        if not normalized:
            return fail(output, f"{fixture_id(index)} is empty after whitespace normalization", summary_output)
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
        return fail(output, "report output could not be written", summary_output)

    if summary_output is not None:
        try:
            summary_output.parent.mkdir(parents=True, exist_ok=True)
            summary_output.write_text(
                json.dumps(build_summary(report), indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
        except OSError:
            return fail(output, "summary output could not be written", summary_output)
    return 0


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture-dir", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--summary-output", type=Path)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    return build_report(args.fixture_dir, args.output, args.summary_output)


if __name__ == "__main__":
    raise SystemExit(main())
