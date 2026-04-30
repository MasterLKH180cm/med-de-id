#!/usr/bin/env python3
"""Run the text-only Privacy Filter mock over a small synthetic corpus.

The emitted report is intentionally aggregate-only: it does not include raw input,
masked text, span offsets, or previews from fixture content.
"""
import argparse
import json
import subprocess
import sys
from pathlib import Path


NON_GOALS = [
    'ocr',
    'visual_redaction',
    'image_pixel_redaction',
    'final_pdf_rewrite_export',
    'browser_ui',
    'desktop_ui',
]


def parse_args():
    parser = argparse.ArgumentParser(description='Run Privacy Filter over a text-only synthetic corpus.')
    parser.add_argument('--fixture-dir', required=True, help='Directory containing synthetic .txt fixtures')
    parser.add_argument('--output', required=True, help='Path to write the PHI-safe JSON aggregate report')
    parser.add_argument('--python-command', default=sys.executable, help='Python command used to invoke the runner')
    parser.add_argument(
        '--runner-path',
        default=str(Path(__file__).with_name('run_privacy_filter.py')),
        help='Path to scripts/privacy_filter/run_privacy_filter.py',
    )
    return parser.parse_args()


def run_fixture(python_command: str, runner_path: Path, fixture_path: Path) -> dict:
    result = subprocess.run(
        [python_command, str(runner_path), '--mock', str(fixture_path)],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=20,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(f'privacy filter runner failed for {fixture_path.name} with exit {result.returncode}')
    if result.stderr:
        raise RuntimeError(f'privacy filter runner wrote stderr for {fixture_path.name}')
    return json.loads(result.stdout)


def merge_counts(total: dict, counts: dict) -> None:
    for category, count in counts.items():
        total[str(category)] = total.get(str(category), 0) + int(count)


def build_report(fixture_dir: Path, python_command: str, runner_path: Path) -> dict:
    if not fixture_dir.is_dir():
        raise ValueError('fixture directory is not a directory')
    fixtures = sorted(fixture_dir.glob('*.txt'))
    if not fixtures:
        raise ValueError('no .txt fixtures found')
    category_counts = {}
    fixture_reports = []
    engine = None

    for fixture in fixtures:
        payload = run_fixture(python_command, runner_path, fixture)
        metadata = payload.get('metadata', {}) if isinstance(payload, dict) else {}
        if engine is None:
            engine = metadata.get('engine')
        summary = payload.get('summary', {}) if isinstance(payload, dict) else {}
        counts = summary.get('category_counts', {}) if isinstance(summary, dict) else {}
        merge_counts(category_counts, counts)
        fixture_reports.append(
            {
                'fixture': fixture.name,
                'detected_span_count': int(summary.get('detected_span_count', 0)),
                'category_counts': {str(k): int(v) for k, v in sorted(counts.items())},
            }
        )

    return {
        'engine': engine or 'fallback_synthetic_patterns',
        'scope': 'text_only_synthetic_corpus',
        'fixture_count': len(fixtures),
        'category_counts': {str(k): int(v) for k, v in sorted(category_counts.items())},
        'fixtures': fixture_reports,
        'non_goals': NON_GOALS,
    }


def main():
    args = parse_args()
    fixture_dir = Path(args.fixture_dir)
    runner_path = Path(args.runner_path)
    output_path = Path(args.output)
    try:
        report = build_report(fixture_dir, args.python_command, runner_path)
    except ValueError as exc:
        print(f'error: {exc}', file=sys.stderr)
        return 2
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
