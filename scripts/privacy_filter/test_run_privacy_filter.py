#!/usr/bin/env python3
import importlib.util
import io
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
    def test_stdin_mock_reads_stdin_emits_contract_and_detects_phi(self):
        phi = 'Patient Jane Example has MRN-12345\n'
        result = subprocess.run(
            [sys.executable, str(RUNNER), '--stdin', '--mock'],
            input=phi,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=5,
            check=False,
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, '')
        payload = json.loads(result.stdout)
        self.assertEqual(payload['metadata']['engine'], 'fallback_synthetic_patterns')
        self.assertEqual(payload['summary']['input_char_count'], len(phi))
        self.assertGreaterEqual(payload['summary']['category_counts'].get('NAME', 0), 1)
        self.assertGreaterEqual(payload['summary']['category_counts'].get('MRN', 0), 1)
        self.assertIn('[NAME]', payload['masked_text'])
        self.assertIn('[MRN]', payload['masked_text'])

    def test_stdin_mock_detects_ssn_without_phi_previews(self):
        phi = 'Patient Jane Example has SSN 123-45-6789 for intake\n'
        result = subprocess.run(
            [sys.executable, str(RUNNER), '--stdin', '--mock'],
            input=phi,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=5,
            check=False,
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, '')
        payload = json.loads(result.stdout)
        self.assertEqual(payload['metadata']['engine'], 'fallback_synthetic_patterns')
        self.assertEqual(payload['metadata']['network_api_called'], False)
        self.assertEqual(payload['summary']['category_counts'].get('SSN'), 1)
        self.assertIn('[SSN]', payload['masked_text'])
        rendered = json.dumps(payload, sort_keys=True)
        self.assertNotIn('123-45-6789', rendered)
        self.assertTrue(all(span['preview'] == '<redacted>' for span in payload['spans']))

    def test_stdin_mock_does_not_detect_embedded_ssn_like_tokens(self):
        phi = 'Codes ID123-45-6789 abc123-45-6789 123-45-6789-extra remain ordinary text\n'
        result = subprocess.run(
            [sys.executable, str(RUNNER), '--stdin', '--mock'],
            input=phi,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=5,
            check=False,
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, '')
        payload = json.loads(result.stdout)
        self.assertNotIn('SSN', payload['summary']['category_counts'])
        self.assertNotIn('[SSN]', payload['masked_text'])

    def test_stdin_mock_detects_zip_codes_without_phi_previews(self):
        phi = 'Patient Jane Example lives in ZIP 02139 and alternate 02139-4307\n'
        result = subprocess.run(
            [sys.executable, str(RUNNER), '--stdin', '--mock'],
            input=phi,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=5,
            check=False,
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, '')
        payload = json.loads(result.stdout)
        self.assertEqual(payload['metadata']['engine'], 'fallback_synthetic_patterns')
        self.assertEqual(payload['metadata']['network_api_called'], False)
        self.assertEqual(payload['summary']['category_counts'].get('ZIP'), 2)
        self.assertIn('[ZIP]', payload['masked_text'])
        rendered = json.dumps(payload, sort_keys=True)
        self.assertNotIn('02139', rendered)
        self.assertNotIn('02139-4307', rendered)
        self.assertTrue(all(span['preview'] == '<redacted>' for span in payload['spans']))

    def test_stdin_mock_does_not_detect_embedded_zip_like_tokens(self):
        phi = 'Codes A02139 02139B 02139-4307-extra and ID02139 remain ordinary text\n'
        result = subprocess.run(
            [sys.executable, str(RUNNER), '--stdin', '--mock'],
            input=phi,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=5,
            check=False,
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, '')
        payload = json.loads(result.stdout)
        self.assertNotIn('ZIP', payload['summary']['category_counts'])
        self.assertNotIn('[ZIP]', payload['masked_text'])

    def test_stdin_rejects_oversized_input_without_stdout_or_phi(self):
        phi_prefix = 'Patient Jane Example has MRN-12345\n'
        result = subprocess.run(
            [sys.executable, str(RUNNER), '--stdin', '--mock'],
            input=phi_prefix + ('x' * (1024 * 1024 + 1)),
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=5,
            check=False,
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, '')
        self.assertIn('stdin input exceeds 1048576 byte limit', result.stderr)
        self.assertNotIn('Jane Example', result.stderr)
        self.assertNotIn('MRN-12345', result.stderr)

    def test_positional_input_plus_stdin_is_rejected_phi_safely(self):
        with tempfile.TemporaryDirectory() as tmp:
            input_path = Path(tmp) / 'input.txt'
            input_path.write_text('Patient Jane Example has MRN-12345\n', encoding='utf-8')
            result = subprocess.run(
                [sys.executable, str(RUNNER), str(input_path), '--stdin', '--mock'],
                input='Patient John Sample has MRN-67890\n',
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=5,
                check=False,
            )

        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, '')
        self.assertIn('exactly one input source is required', result.stderr)
        self.assertNotIn('Jane Example', result.stderr)
        self.assertNotIn('MRN-12345', result.stderr)
        self.assertNotIn('John Sample', result.stderr)
        self.assertNotIn('MRN-67890', result.stderr)

    def test_explicit_opf_with_stdin_uses_subprocess_stdin_not_phi_argv(self):
        module = load_runner_module()
        phi = 'Patient Jane Example has MRN-12345\n'
        raw_opf = json.dumps({'masked_text': 'Patient [NAME] has [MRN]\n', 'spans': []})
        stdout = io.StringIO()
        stderr = io.StringIO()
        stdin = io.StringIO(phi)

        with mock.patch.object(module.sys, 'argv', ['run_privacy_filter.py', '--stdin', '--use-opf']), \
             mock.patch.object(module.shutil, 'which', return_value='/tmp/opf'), \
             mock.patch.object(module, 'run_opf_with_stdin', return_value=raw_opf) as run_opf, \
             mock.patch.object(module.sys, 'stdin', stdin), \
             mock.patch.object(module.sys, 'stdout', stdout), \
             mock.patch.object(module.sys, 'stderr', stderr):
            module.main()

        self.assertEqual(stderr.getvalue(), '')
        run_opf.assert_called_once_with('/tmp/opf', phi)
        self.assertNotIn(phi, run_opf.call_args.args[0])
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload['metadata']['engine'], 'openai_privacy_filter_opf')

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

    def test_explicit_opf_subprocess_path_is_verified_with_local_fake_binary(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            recorder = tmp_path / 'opf-stdin.txt'
            fake_opf = tmp_path / 'opf'
            fake_opf.write_text(
                '#!/usr/bin/env python3\n'
                'import json, pathlib, sys\n'
                f'pathlib.Path({str(recorder)!r}).write_text(sys.stdin.read(), encoding="utf-8")\n'
                'print(json.dumps({"masked_text":"Patient [NAME] has [MRN]",'
                '"spans":[{"label":"NAME","start":8,"end":20,"preview":"Jane Example"},'
                '{"label":"MRN","start":25,"end":34,"preview":"MRN-12345"}]}))\n',
                encoding='utf-8',
            )
            fake_opf.chmod(0o755)
            env = os.environ.copy()
            env['PATH'] = f'{tmp_path}{os.pathsep}{env.get("PATH", "")}'
            phi = 'Patient Jane Example has MRN-12345\n'

            result = subprocess.run(
                [sys.executable, str(RUNNER), '--stdin', '--use-opf'],
                input=phi,
                env=env,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=5,
                check=False,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stderr, '')
            self.assertEqual(recorder.read_text(encoding='utf-8'), phi)
            payload = json.loads(result.stdout)
            self.assertEqual(payload['metadata']['engine'], 'openai_privacy_filter_opf')
            self.assertEqual(payload['metadata']['network_api_called'], False)
            self.assertEqual(payload['summary']['category_counts'], {'MRN': 1, 'NAME': 1})
            rendered = json.dumps(payload, sort_keys=True)
            self.assertNotIn('Jane Example', rendered)
            self.assertNotIn('MRN-12345', rendered)
            self.assertTrue(all(span['preview'] == '<redacted>' for span in payload['spans']))

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

    def test_explicit_opf_canonical_json_is_normalized_without_phi_previews(self):
        module = load_runner_module()
        phi = 'Patient Jane Example has MRN-12345\n'
        raw_opf = json.dumps({
            'masked_text': 'Patient [NAME] has [MRN]\n',
            'spans': [
                {'label': 'MRN', 'start': 25, 'end': 34, 'preview': 'MRN-12345'},
                {'label': 'NAME', 'start': 8, 'end': 20, 'preview': 'Jane Example'},
            ],
        })

        with tempfile.TemporaryDirectory() as tmp:
            input_path = Path(tmp) / 'input.txt'
            input_path.write_text(phi, encoding='utf-8')
            stdout = io.StringIO()
            stderr = io.StringIO()
            with mock.patch.object(module.sys, 'argv', ['run_privacy_filter.py', '--use-opf', str(input_path)]), \
                 mock.patch.object(module.shutil, 'which', return_value='/tmp/opf'), \
                 mock.patch.object(module, 'run_opf_with_stdin', return_value=raw_opf), \
                 mock.patch.object(module.sys, 'stdout', stdout), \
                 mock.patch.object(module.sys, 'stderr', stderr):
                module.main()

        self.assertEqual(stderr.getvalue(), '')
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload['masked_text'], '[NAME] [MRN]')
        self.assertEqual(payload['summary']['category_counts'], {'MRN': 1, 'NAME': 1})
        self.assertEqual([span['label'] for span in payload['spans']], ['NAME', 'MRN'])
        normalized = json.dumps(payload, sort_keys=True)
        self.assertNotIn('Jane Example', normalized)
        self.assertNotIn('MRN-12345', normalized)
        self.assertTrue(all(span['preview'] == '<redacted>' for span in payload['spans']))

    def test_explicit_opf_alternate_entities_shape_is_normalized_without_phi_previews(self):
        module = load_runner_module()
        phi = 'Patient Jane Example email jane@example.test\n'
        raw_opf = json.dumps({
            'text': 'Patient [NAME] email [EMAIL]\n',
            'entities': [
                {'type': 'EMAIL', 'begin': '27', 'finish': '44', 'preview': 'jane@example.test'},
                {'category': 'NAME', 'begin': '8', 'stop': '20', 'preview': 'Jane Example'},
            ],
        })

        with tempfile.TemporaryDirectory() as tmp:
            input_path = Path(tmp) / 'input.txt'
            input_path.write_text(phi, encoding='utf-8')
            stdout = io.StringIO()
            stderr = io.StringIO()
            with mock.patch.object(module.sys, 'argv', ['run_privacy_filter.py', '--use-opf', str(input_path)]), \
                 mock.patch.object(module.shutil, 'which', return_value='/tmp/opf'), \
                 mock.patch.object(module, 'run_opf_with_stdin', return_value=raw_opf), \
                 mock.patch.object(module.sys, 'stdout', stdout), \
                 mock.patch.object(module.sys, 'stderr', stderr):
                module.main()

        self.assertEqual(stderr.getvalue(), '')
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload['masked_text'], '[NAME] [EMAIL]')
        self.assertEqual(payload['summary']['category_counts'], {'EMAIL': 1, 'NAME': 1})
        self.assertEqual([span['label'] for span in payload['spans']], ['NAME', 'EMAIL'])
        normalized = json.dumps(payload, sort_keys=True)
        self.assertNotIn('Jane Example', normalized)
        self.assertNotIn('jane@example.test', normalized)
        self.assertTrue(all(span['preview'] == '<redacted>' for span in payload['spans']))

    def test_explicit_opf_reconstructs_masked_text_without_raw_text_passthrough(self):
        module = load_runner_module()
        phi = 'Patient Jane Example has SSN 123-45-6789\n'
        raw_opf = json.dumps({
            'text': phi,
            'masked_text': phi,
            'spans': [
                {'label': 'NAME', 'start': 8, 'end': 20, 'preview': 'Jane Example'},
                {'label': 'SSN', 'start': 29, 'end': 40, 'preview': '123-45-6789'},
            ],
        })

        payload = module.normalize_opf_json(raw_opf, len(phi))
        self.assertEqual(payload['masked_text'], '[NAME] [SSN]')
        rendered = json.dumps(payload, sort_keys=True)
        self.assertNotIn('Jane Example', rendered)
        self.assertNotIn('123-45-6789', rendered)

    def test_explicit_opf_rejects_non_object_spans_without_phi_leak(self):
        module = load_runner_module()
        phi = 'Patient Jane Example has rare token 123-45-6789\n'
        raw_opf = json.dumps({'masked_text': phi, 'spans': ['Jane Example']})

        with tempfile.TemporaryDirectory() as tmp:
            input_path = Path(tmp) / 'input.txt'
            input_path.write_text(phi, encoding='utf-8')
            stdout = io.StringIO()
            stderr = io.StringIO()
            with mock.patch.object(module.sys, 'argv', ['run_privacy_filter.py', '--use-opf', str(input_path)]), \
                 mock.patch.object(module.shutil, 'which', return_value='/tmp/opf'), \
                 mock.patch.object(module, 'run_opf_with_stdin', return_value=raw_opf), \
                 mock.patch.object(module.sys, 'stdout', stdout), \
                 mock.patch.object(module.sys, 'stderr', stderr):
                with self.assertRaises(SystemExit) as raised:
                    module.main()

        self.assertEqual(raised.exception.code, 4)
        self.assertEqual(stdout.getvalue(), '')
        self.assertIn('opf returned non-JSON output', stderr.getvalue())
        self.assertNotIn('Jane Example', stderr.getvalue())
        self.assertNotIn('123-45-6789', stderr.getvalue())

    def test_explicit_opf_rejects_unsupported_category_without_phi_leak(self):
        module = load_runner_module()
        phi = 'Patient Jane Example has rare token 123-45-6789\n'
        raw_opf = json.dumps({
            'masked_text': 'Patient [ALIEN] has rare token [SSN]\n',
            'spans': [
                {'label': 'ALIEN', 'start': 8, 'end': 20, 'preview': 'Jane Example'},
                {'label': 'SSN', 'start': 36, 'end': 47, 'preview': '123-45-6789'},
            ],
        })

        with tempfile.TemporaryDirectory() as tmp:
            input_path = Path(tmp) / 'input.txt'
            input_path.write_text(phi, encoding='utf-8')
            stdout = io.StringIO()
            stderr = io.StringIO()
            with mock.patch.object(module.sys, 'argv', ['run_privacy_filter.py', '--use-opf', str(input_path)]), \
                 mock.patch.object(module.shutil, 'which', return_value='/tmp/opf'), \
                 mock.patch.object(module, 'run_opf_with_stdin', return_value=raw_opf), \
                 mock.patch.object(module.sys, 'stdout', stdout), \
                 mock.patch.object(module.sys, 'stderr', stderr):
                with self.assertRaises(SystemExit) as raised:
                    module.main()

        self.assertEqual(raised.exception.code, 4)
        self.assertEqual(stdout.getvalue(), '')
        self.assertIn('opf returned non-JSON output', stderr.getvalue())
        self.assertNotIn('Jane Example', stderr.getvalue())
        self.assertNotIn('123-45-6789', stderr.getvalue())


class PrivacyFilterSyntheticCorpusTests(unittest.TestCase):
    def test_missing_fixture_dir_fails_without_report(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            output_path = tmp_path / 'corpus-report.json'
            missing_dir = tmp_path / 'missing-fixtures'
            result = subprocess.run(
                [
                    sys.executable,
                    str(CORPUS_RUNNER),
                    '--fixture-dir',
                    str(missing_dir),
                    '--output',
                    str(output_path),
                ],
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=10,
                check=False,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(result.stdout, '')
            self.assertIn('fixture directory is not a directory', result.stderr)
            self.assertFalse(output_path.exists())

    def test_empty_fixture_dir_fails_without_report(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            fixture_dir = tmp_path / 'empty-fixtures'
            fixture_dir.mkdir()
            output_path = tmp_path / 'corpus-report.json'
            result = subprocess.run(
                [
                    sys.executable,
                    str(CORPUS_RUNNER),
                    '--fixture-dir',
                    str(fixture_dir),
                    '--output',
                    str(output_path),
                ],
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=10,
                check=False,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(result.stdout, '')
            self.assertIn('no .txt fixtures found', result.stderr)
            self.assertFalse(output_path.exists())

    def test_invalid_fixture_output_fails_without_report(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            fixture_dir = tmp_path / 'fixtures'
            fixture_dir.mkdir()
            (fixture_dir / 'sample.txt').write_text('synthetic fixture text\n', encoding='utf-8')
            output_path = tmp_path / 'corpus-report.json'
            fake_runner = tmp_path / 'fake_runner.py'
            fake_runner.write_text(
                'import json, sys\n'
                'json.dump({"summary": {"detected_span_count": 0}}, sys.stdout)\n',
                encoding='utf-8',
            )

            result = subprocess.run(
                [
                    sys.executable,
                    str(CORPUS_RUNNER),
                    '--fixture-dir',
                    str(fixture_dir),
                    '--output',
                    str(output_path),
                    '--runner-path',
                    str(fake_runner),
                ],
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=10,
                check=False,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(result.stdout, '')
            self.assertIn('invalid privacy filter output for sample.txt', result.stderr)
            self.assertIn('missing top-level key: masked_text', result.stderr)
            self.assertNotIn('synthetic fixture text', result.stderr)
            self.assertFalse(output_path.exists())

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
        self.assertEqual(report['total_detected_span_count'], sum(report['category_counts'].values()))
        for category in ('NAME', 'MRN', 'EMAIL', 'PHONE'):
            self.assertGreaterEqual(report['category_counts'].get(category, 0), 1)

        raw_report = json.dumps(report, sort_keys=True)
        for phi in (
            'Jane Example',
            'MRN-12345',
            'jane@example.test',
            '555-111-2222',
            'John Sample',
            'john.sample@example.test',
            '555-333-4444',
            'MRN 67890',
        ):
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
