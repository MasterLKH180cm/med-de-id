#!/usr/bin/env python3
import sys
from pathlib import Path

def norm(s: str) -> str:
    return ' '.join(s.split())

actual = norm(Path(sys.argv[1]).read_text(encoding='utf-8'))
expected = norm(Path(sys.argv[2]).read_text(encoding='utf-8'))
if actual == expected:
    print('small-ocr output matches expected text')
    raise SystemExit(0)
print('small-ocr output mismatch', file=sys.stderr)
print('EXPECTED:', expected, file=sys.stderr)
print('ACTUAL  :', actual, file=sys.stderr)
raise SystemExit(1)
