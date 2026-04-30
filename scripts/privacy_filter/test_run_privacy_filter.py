#!/usr/bin/env python3
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
RUNNER = REPO_ROOT / 'scripts' / 'privacy_filter' / 'run_privacy_filter.py'


class PrivacyFilterRunnerFailureTests(unittest.TestCase):
    def test_opf_failure_stderr_is_phi_safe_and_generic(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            input_path = tmp_path / 'input.txt'
            input_path.write_text('Patient Jane Example has MRN-12345\n', encoding='utf-8')
            fake_opf = tmp_path / 'opf'
            fake_opf.write_text(
                '#!/bin/sh\n'
                'printf "%s\\n" "raw failure leaked Jane Example MRN-12345" >&2\n'
                'exit 7\n',
                encoding='utf-8',
            )
            fake_opf.chmod(0o755)
            env = os.environ.copy()
            env['PATH'] = f'{tmp_path}{os.pathsep}{env.get("PATH", "")}'

            result = subprocess.run(
                [sys.executable, str(RUNNER), str(input_path)],
                env=env,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=5,
                check=False,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn('opf failed; run with --mock or inspect local opf configuration.', result.stderr)
            self.assertNotIn('Jane Example', result.stderr)
            self.assertNotIn('MRN-12345', result.stderr)
            self.assertNotIn('Jane Example', result.stdout)
            self.assertNotIn('MRN-12345', result.stdout)


if __name__ == '__main__':
    unittest.main()
