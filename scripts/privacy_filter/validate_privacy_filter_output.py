#!/usr/bin/env python3
"""Validate the bounded Privacy Filter CLI spike JSON contract."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

REQUIRED_TOP_LEVEL_KEYS = {"summary", "masked_text", "spans"}
REQUIRED_SUMMARY_KEYS = {"input_char_count", "detected_span_count", "category_counts"}
REQUIRED_SPAN_KEYS = {"label", "start", "end", "preview"}


def _fail(message: str) -> int:
    print(f"privacy-filter contract validation failed: {message}", file=sys.stderr)
    return 1


def _validate_document(document: Any) -> list[str]:
    errors: list[str] = []

    if not isinstance(document, dict):
        return ["top-level JSON value must be an object"]

    missing_top = REQUIRED_TOP_LEVEL_KEYS - document.keys()
    if missing_top:
        errors.append(f"missing top-level keys: {sorted(missing_top)}")

    summary = document.get("summary")
    if not isinstance(summary, dict):
        errors.append("summary must be an object")
    else:
        missing_summary = REQUIRED_SUMMARY_KEYS - summary.keys()
        if missing_summary:
            errors.append(f"missing summary keys: {sorted(missing_summary)}")
        if not isinstance(summary.get("input_char_count"), int) or summary.get("input_char_count", -1) < 0:
            errors.append("summary.input_char_count must be a non-negative integer")
        if not isinstance(summary.get("detected_span_count"), int) or summary.get("detected_span_count", -1) < 0:
            errors.append("summary.detected_span_count must be a non-negative integer")
        if not isinstance(summary.get("category_counts"), dict):
            errors.append("summary.category_counts must be an object")
        else:
            for label, count in summary["category_counts"].items():
                if not isinstance(label, str) or not label:
                    errors.append("summary.category_counts keys must be non-empty strings")
                if not isinstance(count, int) or count < 0:
                    errors.append(f"summary.category_counts[{label!r}] must be a non-negative integer")

    if not isinstance(document.get("masked_text"), str):
        errors.append("masked_text must be a string")

    spans = document.get("spans")
    if not isinstance(spans, list):
        errors.append("spans must be an array")
    else:
        previous_start = -1
        for index, span in enumerate(spans):
            if not isinstance(span, dict):
                errors.append(f"spans[{index}] must be an object")
                continue
            missing_span = REQUIRED_SPAN_KEYS - span.keys()
            if missing_span:
                errors.append(f"spans[{index}] missing keys: {sorted(missing_span)}")
            label = span.get("label")
            start = span.get("start")
            end = span.get("end")
            preview = span.get("preview")
            if not isinstance(label, str) or not label:
                errors.append(f"spans[{index}].label must be a non-empty string")
            if not isinstance(start, int) or start < 0:
                errors.append(f"spans[{index}].start must be a non-negative integer")
            if not isinstance(end, int) or end < 0:
                errors.append(f"spans[{index}].end must be a non-negative integer")
            if isinstance(start, int) and isinstance(end, int) and start >= end:
                errors.append(f"spans[{index}] must use start-inclusive/end-exclusive offsets with start < end")
            if isinstance(start, int) and start < previous_start:
                errors.append("spans must be sorted by start offset")
            if isinstance(start, int):
                previous_start = start
            if not isinstance(preview, str) or not preview:
                errors.append(f"spans[{index}].preview must be a non-empty string")
            elif not (preview.startswith("[") and preview.endswith("]")):
                errors.append(f"spans[{index}].preview must be redacted or fixture-safe; expected bracketed label")

        if isinstance(summary, dict) and isinstance(summary.get("detected_span_count"), int):
            if summary["detected_span_count"] != len(spans):
                errors.append("summary.detected_span_count must equal len(spans)")

        if isinstance(summary, dict) and isinstance(summary.get("category_counts"), dict):
            actual_counts: dict[str, int] = {}
            for span in spans:
                if isinstance(span, dict) and isinstance(span.get("label"), str):
                    actual_counts[span["label"]] = actual_counts.get(span["label"], 0) + 1
            if summary["category_counts"] != actual_counts:
                errors.append("summary.category_counts must match span labels")

    return errors


def _read_json_text(argument: str) -> str:
    if argument == "-":
        return sys.stdin.read()
    return Path(argument).read_text(encoding="utf-8")


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        return _fail("usage: validate_privacy_filter_output.py OUTPUT_JSON_OR_-")

    source = argv[1]
    try:
        document = json.loads(_read_json_text(source))
    except FileNotFoundError:
        return _fail(f"file not found: {source}")
    except json.JSONDecodeError as exc:
        return _fail(f"invalid JSON: {exc}")

    errors = _validate_document(document)
    if errors:
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print("privacy-filter contract validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
