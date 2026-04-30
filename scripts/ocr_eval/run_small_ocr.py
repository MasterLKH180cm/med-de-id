#!/usr/bin/env python3
import argparse, shutil, subprocess, sys
from pathlib import Path

ap = argparse.ArgumentParser()
ap.add_argument('input_path')
ap.add_argument('--mock', action='store_true', help='Use fixture-backed mock extraction for plumbing only')
args = ap.parse_args()
input_path = Path(args.input_path)
if args.mock:
    expected = input_path.parent / 'synthetic_printed_phi_expected.txt'
    if expected.exists():
        sys.stdout.write(expected.read_text(encoding='utf-8'))
        raise SystemExit(0)
    print('mock expected text fixture not found', file=sys.stderr)
    raise SystemExit(2)

python = shutil.which('python') or shutil.which('python3')
if python is None:
    print('python not found', file=sys.stderr)
    raise SystemExit(2)

try:
    import paddleocr  # type: ignore
except Exception:
    print('PaddleOCR is not installed locally. Re-run with --mock for plumbing only, or install the OCR stack first.', file=sys.stderr)
    raise SystemExit(3)

print('Real PaddleOCR path is intentionally not wired yet in this first bounded spike; use --mock for plumbing or implement model invocation next.', file=sys.stderr)
raise SystemExit(4)
