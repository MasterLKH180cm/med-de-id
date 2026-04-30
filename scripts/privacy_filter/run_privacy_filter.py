#!/usr/bin/env python3
"""Local Privacy Filter CLI spike runner.

This script intentionally does not call network APIs. In --engine auto it may detect a
future local Privacy Filter package, but this spike currently normalizes through the
explicit deterministic synthetic-pattern fallback so local verification remains truthful.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import re
import sys
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

ENGINE_FALLBACK = "fallback_synthetic_patterns"
ENGINE_AUTO = "auto"

PATTERNS: tuple[tuple[str, re.Pattern[str]], ...] = (
    ("EMAIL", re.compile(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b")),
    ("PHONE", re.compile(r"\b(?:\d{3}[-.]?){2}\d{4}\b")),
    ("MRN", re.compile(r"\bMRN[-: ]?[A-Z0-9-]+\b", re.IGNORECASE)),
    ("DATE", re.compile(r"\b\d{4}-\d{2}-\d{2}\b")),
    ("NAME", re.compile(r"(?m)^Patient:[ \t]*([A-Z][A-Za-z]+(?:[ \t]+[A-Z][A-Za-z]+)+)\b")),
)


@dataclass(frozen=True)
class Span:
    label: str
    start: int
    end: int

    @property
    def preview(self) -> str:
        return f"[{self.label}]"

    def as_contract(self) -> dict[str, object]:
        return {
            "label": self.label,
            "start": self.start,
            "end": self.end,
            "preview": self.preview,
        }


def _select_engine(requested_engine: str) -> str:
    if requested_engine == ENGINE_FALLBACK:
        return ENGINE_FALLBACK

    # Do not import or call any service in this spike. This spec check documents the
    # intended future extension point while preserving safe deterministic local behavior.
    importlib.util.find_spec("openai_privacy_filter")
    return ENGINE_FALLBACK


def _overlaps_existing(start: int, end: int, spans: Iterable[Span]) -> bool:
    return any(start < span.end and end > span.start for span in spans)


def _detect_with_fallback(text: str) -> list[Span]:
    spans: list[Span] = []
    for label, pattern in PATTERNS:
        for match in pattern.finditer(text):
            if label == "NAME" and match.lastindex:
                start, end = match.span(1)
            else:
                start, end = match.span(0)
            if not _overlaps_existing(start, end, spans):
                spans.append(Span(label=label, start=start, end=end))
    return sorted(spans, key=lambda span: (span.start, span.end, span.label))


def _masked_text(text: str, spans: list[Span]) -> str:
    chunks: list[str] = []
    cursor = 0
    for span in spans:
        chunks.append(text[cursor:span.start])
        chunks.append(span.preview)
        cursor = span.end
    chunks.append(text[cursor:])
    return "".join(chunks)


def _build_contract(text: str, engine: str) -> dict[str, object]:
    spans = _detect_with_fallback(text)
    counts = Counter(span.label for span in spans)
    return {
        "summary": {
            "input_char_count": len(text),
            "detected_span_count": len(spans),
            "category_counts": dict(sorted(counts.items())),
        },
        "masked_text": _masked_text(text, spans),
        "spans": [span.as_contract() for span in spans],
        "metadata": {
            "engine": engine,
            "network_api_called": False,
            "preview_policy": "redacted_bracket_labels_only",
        },
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Run the local Privacy Filter CLI spike on a text file.")
    parser.add_argument("input_path", help="UTF-8 text input file path")
    parser.add_argument("--engine", choices=(ENGINE_AUTO, ENGINE_FALLBACK), default=ENGINE_AUTO)
    args = parser.parse_args(argv[1:])

    input_path = Path(args.input_path)
    try:
        text = input_path.read_text(encoding="utf-8")
    except FileNotFoundError:
        print(f"privacy-filter runner failed: file not found: {input_path}", file=sys.stderr)
        return 1
    except UnicodeDecodeError as exc:
        print(f"privacy-filter runner failed: input must be UTF-8 text: {exc}", file=sys.stderr)
        return 1

    engine = _select_engine(args.engine)
    json.dump(_build_contract(text, engine), sys.stdout, indent=2)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
