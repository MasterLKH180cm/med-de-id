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
    parser.add_argument("image_paths", nargs="*")
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
    parser.add_argument(
        "--manifest",
        help="Path to a JSON manifest describing documents and page image paths",
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


def safe_document_id(value: object) -> str | None:
    if not isinstance(value, str):
        return None
    stripped = value.strip()
    if not stripped or len(stripped) > 128:
        return None
    if any(ord(ch) < 32 or ord(ch) == 127 for ch in stripped):
        return None
    return stripped


def load_manifest_pages(manifest_path: str) -> tuple[list[dict], list[dict]] | None:
    try:
        with open(manifest_path, "r", encoding="utf-8") as handle:
            payload = json.load(handle)
    except (OSError, json.JSONDecodeError):
        return None

    documents = payload.get("documents") if isinstance(payload, dict) else None
    if not isinstance(documents, list):
        return None

    pages: list[dict] = []
    document_specs: list[dict] = []
    for document in documents:
        if not isinstance(document, dict):
            return None
        document_id = safe_document_id(document.get("document_id"))
        document_pages = document.get("pages")
        if document_id is None or not isinstance(document_pages, list):
            return None
        document_specs.append({"document_id": document_id, "page_count": len(document_pages)})
        for page in document_pages:
            if not isinstance(page, dict):
                return None
            page_number = page.get("page_number")
            image_path = page.get("image_path")
            if not isinstance(page_number, int) or isinstance(page_number, bool):
                return None
            if page_number < 1 or not isinstance(image_path, str) or not image_path:
                return None
            pages.append(
                {
                    "document_id": document_id,
                    "page_number": page_number,
                    "image_path": image_path,
                }
            )
    return pages, document_specs


def build_document_summaries(items: list[dict], document_specs: list[dict]) -> list[dict]:
    summaries = []
    for spec in document_specs:
        document_items = [
            item for item in items if item.get("document_id") == spec["document_id"]
        ]
        succeeded_count = sum(1 for item in document_items if item["status"] == "succeeded")
        summaries.append(
            {
                "document_id": spec["document_id"],
                "page_count": spec["page_count"],
                "succeeded_count": succeeded_count,
                "failed_count": len(document_items) - succeeded_count,
            }
        )
    return summaries


def build_summary(items: list[dict], document_specs: list[dict] | None = None) -> dict:
    successes = [item for item in items if item["status"] == "succeeded"]
    failed_count = len(items) - len(successes)
    engine_status_counts = Counter(item["engine_status"] for item in successes)
    real_model_quality_verified = (
        bool(successes)
        and len(successes) == len(items)
        and all(item["engine_status"] == REAL_ENGINE_STATUS for item in successes)
    )
    summary = {
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
    if document_specs is not None:
        summary["document_count"] = len(document_specs)
        summary["documents"] = build_document_summaries(items, document_specs)
    return summary


def main(argv: Sequence[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.manifest and args.image_paths:
        sys.stderr.write("OCR manifest cannot be combined with positional image paths\n")
        return 2
    if not args.manifest and not args.image_paths:
        sys.stderr.write("OCR batch requires image paths or manifest\n")
        return 2

    document_specs = None
    if args.manifest:
        manifest = load_manifest_pages(args.manifest)
        if manifest is None:
            sys.stderr.write("OCR manifest is malformed\n")
            return 2
        manifest_pages, document_specs = manifest
        items = []
        for index, page in enumerate(manifest_pages):
            item = run_one(args.runner_path, page["image_path"], args.mock, index)
            item["document_id"] = page["document_id"]
            item["page_number"] = page["page_number"]
            items.append(item)
    else:
        items = [
            run_one(args.runner_path, image_path, args.mock, index)
            for index, image_path in enumerate(args.image_paths)
        ]
    sys.stdout.write(json.dumps(build_summary(items, document_specs), indent=2, sort_keys=True))
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
