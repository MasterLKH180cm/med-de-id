#!/usr/bin/env python3
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


REPO_ROOT = Path(__file__).resolve().parents[2]
RUNNER = REPO_ROOT / 'scripts' / 'privacy_filter' / 'run_privacy_filter.py'
CORPUS_RUNNER = REPO_ROOT / 'scripts' / 'privacy_filter' / 'run_synthetic_corpus.py'
CORPUS_FIXTURE_DIR = REPO_ROOT / 'scripts' / 'privacy_filter' / 'fixtures' / 'corpus'


def load_runner_module():
    spec = importlib.util.spec_from_file_location('run_privacy_filter_under_test', RUNNER)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class PrivacyFilterRunnerFailureTests(unittest.TestCase):
    def test_ambient_opf_is_not_auto_used(self):
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

            self.assertEqual(result.returncode, 0)
            self.assertEqual(result.stderr, '')
            payload = json.loads(result.stdout)
            self.assertEqual(payload['metadata']['engine'], 'fallback_synthetic_patterns')
            self.assertNotIn('Jane Example', result.stderr)
            self.assertNotIn('MRN-12345', result.stderr)

    def test_explicit_opf_uses_stdin_not_phi_argv(self):
        module = load_runner_module()
        phi = 'Patient Jane Example has MRN-12345\n'

        class FakePopen:
            def __init__(self, argv, **kwargs):
                self.argv = argv
                self.kwargs = kwargs
                self.returncode = 0

            def communicate(self, input=None, timeout=None):
                self.input = input
                return ('{"masked_text":"[NAME] has [MRN]","spans":[]}', '')

            def kill(self):
                pass

        with mock.patch.object(module.subprocess, 'Popen', side_effect=FakePopen) as popen:
            raw = module.run_opf_with_stdin('/tmp/opf', phi)

        self.assertIn('masked_text', raw)
        argv = popen.call_args.args[0]
        self.assertNotIn(phi, argv)
        self.assertEqual(argv, ['/tmp/opf', '--format', 'json'])
        self.assertEqual(popen.call_args.kwargs['stdin'], module.subprocess.PIPE)


class PrivacyFilterSyntheticCorpusTests(unittest.TestCase):
    def test_synthetic_corpus_report_is_aggregate_and_phi_safe(self):
        with tempfile.TemporaryDirectory() as tmp:
            output_path = Path(tmp) / 'corpus-report.json'
            result = subprocess.run(
                [
                    sys.executable,
                    str(CORPUS_RUNNER),
                    '--fixture-dir',
                    str(CORPUS_FIXTURE_DIR),
                    '--output',
                    str(output_path),
                ],
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=10,
                check=False,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stderr, '')
            report = json.loads(output_path.read_text(encoding='utf-8'))

        self.assertEqual(report['engine'], 'fallback_synthetic_patterns')
        self.assertEqual(report['scope'], 'text_only_synthetic_corpus')
        self.assertEqual(report['fixture_count'], 2)
        for category in ('NAME', 'MRN', 'EMAIL', 'PHONE'):
            self.assertGreaterEqual(report['category_counts'].get(category, 0), 1)

        raw_report = json.dumps(report, sort_keys=True)
        for phi in ('Jane Example', 'MRN-12345', 'jane@example.test', '555-111-2222'):
            self.assertNotIn(phi, raw_report)

        self.assertEqual(
            sorted(report['non_goals']),
            sorted([
                'ocr',
                'visual_redaction',
                'image_pixel_redaction',
                'final_pdf_rewrite_export',
                'browser_ui',
                'desktop_ui',
            ]),
        )
        self.assertEqual([entry['fixture'] for entry in report['fixtures']], ['clinic_note.txt', 'contact_card.txt'])
        for entry in report['fixtures']:
            self.assertEqual(set(entry.keys()), {'fixture', 'detected_span_count', 'category_counts'})
            self.assertIsInstance(entry['detected_span_count'], int)
            self.assertIsInstance(entry['category_counts'], dict)


if __name__ == '__main__':
    unittest.main()
