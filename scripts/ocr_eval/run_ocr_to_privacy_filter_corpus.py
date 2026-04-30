#!/usr/bin/env python3
"""Bridge synthetic OCR corpus readiness into aggregate Privacy Filter evidence."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

ARTIFACT = "ocr_to_privacy_filter_corpus_bridge"
SUMMARY_ARTIFACT = "ocr_to_privacy_filter_corpus"
OCR_CANDIDATE = "PP-OCRv5_mobile_rec"
OCR_ENGINE = "PP-OCRv5-mobile-bounded-spike"
SCOPE = "printed_text_extraction_to_text_pii_detection_only"
PRIVACY_FILTER_CONTRACT = "text_only_normalized_input"
SAFE_CATEGORIES = {"NAME", "MRN", "EMAIL", "PHONE", "ID"}
NON_GOALS = sorted({
    "visual_redaction",
    "image_pixel_redaction",
    "final_pdf_rewrite_export",
    "handwriting_recognition",
    "browser_ui",
    "desktop_ui",
})
GENERIC_FAILURE = "ocr_to_privacy_filter_corpus bridge failed"
STALE_CLEANUP_FAILURE = "ocr_to_privacy_filter_corpus stale output cleanup failed"


def remove_stale_output(output: Path) -> None:
    try:
        if output.exists():
            output.unlink()
    except OSError as exc:
        raise RuntimeError(STALE_CLEANUP_FAILURE) from exc


def fail(output: Path, message: str) -> int:
    try:
        remove_stale_output(output)
    except RuntimeError:
        print(STALE_CLEANUP_FAILURE, file=sys.stderr)
        return 1
    print(message, file=sys.stderr)
    return 1


def fixture_id(index: int) -> str:
    return f"fixture_{index:03d}"


def normalize_ocr_text(text: str) -> str:
    # Keep the bridge text-only while normalizing common label punctuation from OCR handoff fixtures.
    return " ".join(text.replace("Patient:", "Patient ").split())


def run_json(argv: list[str]) -> tuple[int, str, str]:
    proc = subprocess.run(argv, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30, check=False)
    return proc.returncode, proc.stdout, proc.stderr


def load_ocr_texts(ocr_report: dict[str, object], fixture_dir: Path) -> list[tuple[str, str, bool]]:
    fixtures_obj = ocr_report.get("fixtures")
    if not isinstance(fixtures_obj, list):
        raise ValueError("OCR report missing fixtures")
    fixture_paths = sorted(fixture_dir.glob("*.txt"))
    if len(fixture_paths) != len(fixtures_obj):
        raise ValueError("OCR report fixture count mismatch")

    loaded: list[tuple[str, str, bool]] = []
    for index, fixture in enumerate(fixtures_obj, start=1):
        if not isinstance(fixture, dict):
            raise ValueError("OCR report fixture is invalid")
        ready = fixture.get("ready_for_text_pii_eval") is True
        raw_text = fixture.get("normalized_text") or fixture.get("extracted_text") or fixture.get("text")
        if not isinstance(raw_text, str):
            raw_text = fixture_paths[index - 1].read_text(encoding="utf-8")
        normalized = normalize_ocr_text(raw_text)
        if not normalized:
            raise ValueError("OCR fixture text is empty")
        loaded.append((fixture_id(index), normalized, ready))
    return loaded


def validate_privacy_report(report: object) -> tuple[int, dict[str, int], str]:
    if not isinstance(report, dict):
        raise ValueError("privacy filter output is not an object")
    summary = report.get("summary")
    metadata = report.get("metadata")
    if not isinstance(summary, dict) or not isinstance(metadata, dict):
        raise ValueError("privacy filter report missing summary or metadata")
    if metadata.get("network_api_called") is not False:
        raise ValueError("privacy filter network_api_called must be false")
    engine = metadata.get("engine")
    if not isinstance(engine, str) or not engine:
        raise ValueError("privacy filter engine missing")
    detected = summary.get("detected_span_count")
    if not isinstance(detected, int) or detected < 0:
        raise ValueError("privacy filter detected_span_count invalid")
    raw_counts = summary.get("category_counts")
    if not isinstance(raw_counts, dict):
        raise ValueError("privacy filter category_counts missing")
    counts: dict[str, int] = {}
    for label, count in raw_counts.items():
        if label not in SAFE_CATEGORIES or not isinstance(count, int) or count < 0:
            raise ValueError("privacy filter category_counts invalid")
        counts[label] = count
    return detected, counts, engine


def build_report(fixture_dir: Path, ocr_runner_path: Path, privacy_runner_path: Path, output: Path) -> int:
    try:
        remove_stale_output(output)
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            ocr_output = tmp_path / "ocr-report.json"
            code, stdout, stderr = run_json([
                sys.executable,
                str(ocr_runner_path),
                "--fixture-dir",
                str(fixture_dir),
                "--output",
                str(ocr_output),
            ])
            if code != 0:
                return fail(output, "OCR corpus runner failed")
            if stdout or stderr:
                # Do not forward child output; it may include fixture implementation detail.
                pass
            ocr_report = json.loads(ocr_output.read_text(encoding="utf-8"))
            texts = load_ocr_texts(ocr_report, fixture_dir)

            fixtures: list[dict[str, object]] = []
            category_counts: dict[str, int] = {}
            total_detected = 0
            privacy_engine: str | None = None
            for safe_id, text, ready in texts:
                input_path = tmp_path / f"{safe_id}.txt"
                pf_output_path = tmp_path / f"{safe_id}-privacy.json"
                input_path.write_text(text, encoding="utf-8")
                code, pf_stdout, _pf_stderr = run_json([sys.executable, str(privacy_runner_path), str(input_path)])
                if code != 0:
                    return fail(output, f"privacy filter failed for {safe_id}")
                pf_output_path.write_text(pf_stdout, encoding="utf-8")
                detected, counts, engine = validate_privacy_report(json.loads(pf_stdout))
                if privacy_engine is None:
                    privacy_engine = engine
                elif privacy_engine != engine:
                    raise ValueError("privacy filter engine changed across fixtures")
                total_detected += detected
                for label, count in counts.items():
                    category_counts[label] = category_counts.get(label, 0) + count
                fixtures.append({"fixture": safe_id, "ready_for_text_pii_eval": ready, "detected_span_count": detected})

            report = {
                "artifact": ARTIFACT,
                "ocr_candidate": OCR_CANDIDATE,
                "ocr_engine": OCR_ENGINE,
                "scope": SCOPE,
                "privacy_filter_engine": privacy_engine or "unknown",
                "privacy_filter_contract": PRIVACY_FILTER_CONTRACT,
                "fixture_count": len(fixtures),
                "ready_fixture_count": sum(1 for fixture in fixtures if fixture["ready_for_text_pii_eval"]),
                "privacy_filter_detected_span_count": total_detected,
                "category_counts": {key: category_counts[key] for key in sorted(category_counts)},
                "privacy_filter_category_counts": {key: category_counts[key] for key in sorted(category_counts)},
                "fixtures": fixtures,
                "non_goals": NON_GOALS,
            }
            output.parent.mkdir(parents=True, exist_ok=True)
            output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            print(json.dumps({"artifact": SUMMARY_ARTIFACT, "report_written": True}, sort_keys=True))
            return 0
    except RuntimeError as exc:
        message = STALE_CLEANUP_FAILURE if str(exc) == STALE_CLEANUP_FAILURE else GENERIC_FAILURE
        return fail(output, message)
    except Exception:
        return fail(output, GENERIC_FAILURE)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture-dir", required=True, type=Path)
    parser.add_argument("--ocr-runner-path", required=True, type=Path)
    parser.add_argument("--privacy-runner-path", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    return build_report(args.fixture_dir, args.ocr_runner_path, args.privacy_runner_path, args.output)


if __name__ == "__main__":
    raise SystemExit(main())
