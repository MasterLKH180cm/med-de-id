#!/usr/bin/env python3
"""PHI-safe aggregate OCR batch quality runner.

Runs the bounded single-image OCR runner once per input and emits aggregate text
metrics only. This intentionally never copies source paths, filenames, raw OCR
text, or normalized OCR text into the aggregate output.
"""

import argparse
import json
import subprocess
import sys
from collections import Counter
from typing import Sequence

CANDIDATE_RECOGNIZER = "PP-OCRv5_mobile_rec"
ENGINE = "PP-OCRv5-mobile-bounded-spike"
REAL_ENGINE_STATUS = "local_paddleocr_execution"
QUALITY_SCOPE_REAL = "local_real_ocr_execution_aggregate_text_metrics"
QUALITY_SCOPE_FIXTURE_OR_MIXED = "fixture_or_mixed_execution_aggregate_text_metrics"
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
    parser.add_argument("image_paths", nargs="+")
    parser.add_argument(
        "--runner-path",
        default="scripts/ocr_eval/run_small_ocr.py",
        help="Path to the local single-image OCR runner",
    )
    parser.add_argument(
        "--mock",
        action="store_true",
        help="Ask the single-image runner to use explicit fixture-backed mock OCR",
    )
    return parser


def safe_error_message(returncode: int, stderr: str) -> str:
    for line in stderr.splitlines():
        stripped = line.strip()
        if stripped in {
            "OCR input path does not exist",
            "OCR input path must be a file",
            "mock expected text fixture not found",
        }:
            return stripped
        if stripped.startswith("PaddleOCR is not installed locally"):
            return "PaddleOCR is not installed locally"
    return "OCR runner failed" if returncode else "OCR runner emitted invalid JSON"


def run_one(runner_path: str, image_path: str, mock: bool, index: int) -> dict:
    command = [sys.executable, runner_path, "--json", image_path]
    if mock:
        command.insert(3, "--mock")

    completed = subprocess.run(
        command,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != 0:
        return {
            "index": index,
            "status": "failed",
            "error_code": completed.returncode,
            "error": safe_error_message(completed.returncode, completed.stderr),
        }

    try:
        payload = json.loads(completed.stdout)
    except json.JSONDecodeError:
        return {
            "index": index,
            "status": "failed",
            "error_code": 1,
            "error": safe_error_message(0, completed.stderr),
        }

    return {
        "index": index,
        "status": "succeeded",
        "engine_status": payload.get("engine_status", "unknown"),
        "line_count": int(payload.get("line_count") or 0),
        "normalized_char_count": int(payload.get("normalized_char_count") or 0),
    }


def build_summary(items: list[dict]) -> dict:
    successes = [item for item in items if item["status"] == "succeeded"]
    failed_count = len(items) - len(successes)
    engine_status_counts = Counter(item["engine_status"] for item in successes)
    real_model_quality_verified = (
        bool(successes)
        and len(successes) == len(items)
        and all(item["engine_status"] == REAL_ENGINE_STATUS for item in successes)
    )
    return {
        "artifact": "ocr_batch_quality_summary",
        "schema_version": 1,
        "candidate": CANDIDATE_RECOGNIZER,
        "engine": ENGINE,
        "input_count": len(items),
        "succeeded_count": len(successes),
        "failed_count": failed_count,
        "engine_status_counts": dict(sorted(engine_status_counts.items())),
        "total_line_count": sum(item.get("line_count", 0) for item in successes),
        "total_normalized_char_count": sum(
            item.get("normalized_char_count", 0) for item in successes
        ),
        "real_model_quality_verified": real_model_quality_verified,
        "quality_scope": QUALITY_SCOPE_REAL
        if real_model_quality_verified
        else QUALITY_SCOPE_FIXTURE_OR_MIXED,
        "items": items,
        "network_api_called": False,
        "non_goals": NON_GOALS,
    }


def main(argv: Sequence[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    items = [
        run_one(args.runner_path, image_path, args.mock, index)
        for index, image_path in enumerate(args.image_paths)
    ]
    sys.stdout.write(json.dumps(build_summary(items), indent=2, sort_keys=True))
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
