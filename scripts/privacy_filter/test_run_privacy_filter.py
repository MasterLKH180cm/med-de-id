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
VALIDATOR = REPO_ROOT / 'scripts' / 'privacy_filter' / 'validate_privacy_filter_output.py'
CORPUS_RUNNER = REPO_ROOT / 'scripts' / 'privacy_filter' / 'run_synthetic_corpus.py'
CORPUS_FIXTURE_DIR = REPO_ROOT / 'scripts' / 'privacy_filter' / 'fixtures' / 'corpus'


def load_runner_module():
    spec = importlib.util.spec_from_file_location('run_privacy_filter_under_test', RUNNER)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_validator_module():
    spec = importlib.util.spec_from_file_location('validate_privacy_filter_output_under_test', VALIDATOR)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def run_text(text):
    module = load_runner_module()
    return module.heuristic_detect(text)


def detect_pii(text):
    return run_text(text)


def run_privacy_filter_payload(text):
    return run_text(text)


class PrivacyFilterRunnerTests(unittest.TestCase):
    def test_fallback_detects_context_required_fax_numbers(self):
        text = 'Patient Jane Example fax 555-222-3333 and fax: (555) 444-5555.'
        payload = run_privacy_filter_payload(text)

        self.assertEqual(payload['summary']['category_counts'].get('FAX'), 2)
        self.assertEqual(payload['masked_text'].count('[FAX]'), 2)
        self.assertNotIn('555-222-3333', payload['masked_text'])
        self.assertNotIn('(555) 444-5555', payload['masked_text'])
        fax_spans = [span for span in payload['spans'] if span['label'] == 'FAX']
        self.assertEqual(len(fax_spans), 2)
        self.assertTrue(all(span['preview'] == '<redacted>' for span in fax_spans))
        self.assertFalse(payload['metadata']['network_api_called'])

    def test_fallback_does_not_classify_plain_phone_or_overlong_fax_as_fax(self):
        text = 'Phone 555-222-3333. fax 555-222-333333. ID555-222-3333'
        payload = run_privacy_filter_payload(text)

        self.assertNotIn('FAX', payload['summary']['category_counts'])
        self.assertNotIn('[FAX]', payload['masked_text'])

    def test_fallback_detects_fax_extension_as_single_fax_span(self):
        text = 'Please fax 555-123-4567 x890 today.'
        payload = run_privacy_filter_payload(text)

        self.assertEqual(payload['summary']['category_counts'].get('FAX'), 1)
        self.assertNotIn('PHONE', payload['summary']['category_counts'])
        self.assertIn('[FAX]', payload['masked_text'])
        self.assertNotIn('555-123-4567', payload['masked_text'])
        self.assertNotIn('x890', payload['masked_text'])
        fax_spans = [span for span in payload['spans'] if span['label'] == 'FAX']
        self.assertEqual(len(fax_spans), 1)
        self.assertEqual(fax_spans[0]['preview'], '<redacted>')

    def test_detects_phone_extensions_without_leaking_raw_values(self):
        text = 'Patient Jane Example call 555-123-4567 x890 or (555) 222-3333 ext. 44 for MRN-12345.'
        payload = detect_pii(text)

        self.assertEqual(payload['summary']['category_counts'].get('PHONE'), 2)
        self.assertEqual(payload['masked_text'].count('[PHONE]'), 2)
        rendered = json.dumps(payload, sort_keys=True)
        for raw_phone in ['555-123-4567 x890', '(555) 222-3333 ext. 44']:
            self.assertNotIn(raw_phone, rendered)
        phone_spans = [span for span in payload['spans'] if span['label'] == 'PHONE']
        self.assertEqual(len(phone_spans), 2)
        self.assertEqual([text[span['start']:span['end']] for span in phone_spans], ['555-123-4567 x890', '(555) 222-3333 ext. 44'])
        self.assertTrue(all(span['preview'] == '<redacted>' for span in phone_spans))

    def test_phone_extension_detector_rejects_embedded_or_unbounded_tokens(self):
        text = 'ID555-123-4567 555-123-4567ext 555-123-4567 x 123456 (555) 222-3333 extension 123456'
        payload = detect_pii(text)

        self.assertEqual(payload['summary']['category_counts'].get('PHONE'), 2)
        self.assertNotIn('ID555-123-4567', payload['masked_text'])
        for raw_phone in ['555-123-4567 x 123456', '(555) 222-3333 extension 123456']:
            self.assertNotIn(raw_phone, payload['masked_text'])
        self.assertEqual(payload['masked_text'].count('[PHONE]'), 2)

    def test_fallback_detects_contextual_vin_without_raw_previews(self):
        text = 'Patient Jane Example vehicle VIN 1HGCM82633A004352 for transport billing.'
        payload = detect_pii(text)
        validator = load_validator_module()

        self.assertEqual(payload['summary']['category_counts'].get('VIN'), 1)
        self.assertIn('[VIN]', payload['masked_text'])
        self.assertNotIn('1HGCM82633A004352', payload['masked_text'])
        vin_spans = [span for span in payload['spans'] if span['label'] == 'VIN']
        self.assertEqual(len(vin_spans), 1)
        self.assertEqual(text[vin_spans[0]['start']:vin_spans[0]['end']], '1HGCM82633A004352')
        self.assertEqual(vin_spans[0]['preview'], '<redacted>')
        self.assertNotIn('1HGCM82633A004352', json.dumps(payload, sort_keys=True))
        validator.validate_privacy_filter_output(payload)

    def test_fallback_does_not_detect_invalid_uncontextual_or_embedded_vin_like_tokens(self):
        text = ' '.join([
            '1HGCM82633A004352 appears without context.',
            'VIN 1HGCM82633A00435I uses forbidden I.',
            'VIN 1HGCM82633A00435O uses forbidden O.',
            'VIN 1HGCM82633A00435Q uses forbidden Q.',
            'VIN X1HGCM82633A004352Y is embedded.',
            'MRN 1HGCM82633A004352 stays bounded.',
        ])
        payload = detect_pii(text)

        self.assertNotIn('VIN', payload['summary']['category_counts'])
        self.assertNotIn('[VIN]', payload['masked_text'])

    def test_fallback_detects_mixed_case_contextual_vin_without_raw_previews(self):
        text = 'Patient Jane Example vin 1hgcm82633a004352 for transport billing.'
        payload = detect_pii(text)

        self.assertEqual(payload['summary']['category_counts'].get('VIN'), 1)
        self.assertIn('[VIN]', payload['masked_text'])
        self.assertNotIn('1hgcm82633a004352', payload['masked_text'])
        vin_spans = [span for span in payload['spans'] if span['label'] == 'VIN']
        self.assertEqual(len(vin_spans), 1)
        self.assertEqual(text[vin_spans[0]['start']:vin_spans[0]['end']], '1hgcm82633a004352')
        self.assertEqual(vin_spans[0]['preview'], '<redacted>')
        self.assertNotIn('1hgcm82633a004352', json.dumps(payload, sort_keys=True))

    def test_fallback_detects_contextual_driver_license_without_raw_previews(self):
        text = 'Patient Jane Example driver license D1234567 for transport billing.'
        payload = detect_pii(text)
        validator = load_validator_module()

        self.assertEqual(payload['summary']['category_counts'].get('DRIVER_LICENSE'), 1)
        self.assertIn('[DRIVER_LICENSE]', payload['masked_text'])
        self.assertNotIn('D1234567', payload['masked_text'])
        driver_license_spans = [span for span in payload['spans'] if span['label'] == 'DRIVER_LICENSE']
        self.assertEqual(len(driver_license_spans), 1)
        self.assertEqual(text[driver_license_spans[0]['start']:driver_license_spans[0]['end']], 'D1234567')
        self.assertEqual(driver_license_spans[0]['preview'], '<redacted>')
        self.assertNotIn('D1234567', json.dumps(payload, sort_keys=True))
        validator.validate_privacy_filter_output(payload)

    def test_fallback_does_not_detect_standalone_or_embedded_driver_license_like_tokens(self):
        text = ' '.join([
            'D1234567 appears without context.',
            'driver license XD1234567Y is embedded.',
            'MRN D1234567 stays medical-record context.',
            'ID D1234567 stays generic-ID context.',
            'driver license ABC123 is not a supported driver-license identifier.',
        ])
        payload = detect_pii(text)

        self.assertNotIn('DRIVER_LICENSE', payload['summary']['category_counts'])
        self.assertNotIn('[DRIVER_LICENSE]', payload['masked_text'])

    def test_fallback_does_not_detect_generic_license_contexts_as_driver_license(self):
        text = ' '.join([
            'medical license number A1234567 remains a professional credential.',
            'professional license A1234567 should not be classified as driver-license PHI.',
            'software license A1234567 should not be classified as driver-license PHI.',
        ])
        payload = detect_pii(text)

        self.assertNotIn('DRIVER_LICENSE', payload['summary']['category_counts'])
        self.assertNotIn('[DRIVER_LICENSE]', payload['masked_text'])

    def test_fallback_detects_ipv4_address_without_raw_previews(self):
        text = 'Patient Jane Example remote login from 192.168.10.42 for MRN-12345'
        payload = detect_pii(text)

        self.assertEqual(payload['summary']['category_counts'].get('IP_ADDRESS'), 1)
        self.assertIn('[IP_ADDRESS]', payload['masked_text'])
        rendered = json.dumps(payload, sort_keys=True)
        self.assertNotIn('192.168.10.42', rendered)
        ip_spans = [span for span in payload['spans'] if span['label'] == 'IP_ADDRESS']
        self.assertEqual(len(ip_spans), 1)
        self.assertEqual(ip_spans[0]['preview'], '<redacted>')

    def test_fallback_does_not_detect_invalid_or_embedded_ipv4_addresses(self):
        text = '999.168.10.42 host192.168.10.42 192.168.10.42extra and 1.2.3 are not IPs'
        payload = detect_pii(text)

        self.assertNotIn('IP_ADDRESS', payload['summary']['category_counts'])
        self.assertNotIn('[IP_ADDRESS]', payload['masked_text'])

    def test_detects_bounded_http_and_https_urls(self):
        expected_urls = [
            'https://portal.example.test',
            'https://portal.example.test/patient/123',
            'http://clinic.example.test/cb?token=abc',
        ]
        text = ' '.join([
            f'Portal {expected_urls[0]}',
            f'patient link {expected_urls[1]}',
            f'callback {expected_urls[2]}',
        ])
        payload = detect_pii(text)

        self.assertEqual(payload['summary']['category_counts'].get('URL'), 3)
        self.assertIn('[URL]', payload['masked_text'])
        for raw_url in expected_urls:
            self.assertNotIn(raw_url, payload['masked_text'])
        url_spans = [span for span in payload['spans'] if span['label'] == 'URL']
        self.assertEqual(len(url_spans), 3)
        self.assertEqual(
            [text[span['start']:span['end']] for span in url_spans],
            expected_urls,
        )
        self.assertTrue(all(span['preview'] == '<redacted>' for span in url_spans))

    def test_url_detector_rejects_unbounded_or_non_http_tokens(self):
        text = ' '.join([
            'invalid-tld https://portal.example.testextra/path',
            'short-host http://a/1',
            'non-http ftp://legacy.example.test',
            'embedded notehttps://portal.example.test/path',
        ])
        payload = detect_pii(text)

        self.assertNotIn('URL', payload['summary']['category_counts'])
        self.assertNotIn('[URL]', payload['masked_text'])

    def test_fallback_detects_contextual_insurance_ids_without_raw_previews(self):
        text = 'Patient Jane Example insurance ID ABC1234567 and member number MBR-7654321.'
        payload = detect_pii(text)
        labels = [span['label'] for span in payload['spans']]
        self.assertEqual(labels.count('INSURANCE_ID'), 2)
        self.assertIn('[INSURANCE_ID]', payload['masked_text'])
        self.assertEqual(payload['summary']['category_counts']['INSURANCE_ID'], 2)
        for span in payload['spans']:
            self.assertEqual(span['preview'], '<redacted>')
        self.assertNotIn('ABC1234567', json.dumps(payload))
        self.assertNotIn('MBR-7654321', json.dumps(payload))

    def test_fallback_detects_contextual_valid_dea_number_without_raw_previews(self):
        text = 'Patient Jane Example DEA AB1234563 for MRN-12345.'
        payload = detect_pii(text)
        validator = load_validator_module()

        self.assertEqual(payload['summary']['category_counts'].get('DEA_NUMBER'), 1)
        self.assertIn('[DEA_NUMBER]', payload['masked_text'])
        self.assertNotIn('AB1234563', payload['masked_text'])
        dea_spans = [span for span in payload['spans'] if span['label'] == 'DEA_NUMBER']
        self.assertEqual(len(dea_spans), 1)
        self.assertEqual(text[dea_spans[0]['start']:dea_spans[0]['end']], 'AB1234563')
        self.assertEqual(dea_spans[0]['preview'], '<redacted>')
        self.assertNotIn('AB1234563', json.dumps(payload, sort_keys=True))
        validator.validate_privacy_filter_output(payload)

    def test_fallback_does_not_detect_invalid_uncontextual_or_wrong_context_dea_like_tokens(self):
        text = 'AB1234563 no context. DEA AB1234564. MRN AB1234563. ID AB1234563. xAB1234563 DEA ZZAB1234563Y.'
        payload = detect_pii(text)

        self.assertNotIn('DEA_NUMBER', payload['summary']['category_counts'])
        self.assertNotIn('[DEA_NUMBER]', payload['masked_text'])

    def test_fallback_does_not_detect_standalone_or_embedded_insurance_like_tokens(self):
        text = 'Standalone ABC1234567 should not match; embedded XABC1234567 and MRN ABC1234567 stay bounded.'
        payload = detect_pii(text)
        labels = [span['label'] for span in payload['spans']]
        self.assertNotIn('INSURANCE_ID', labels)

    def test_fallback_detects_bounded_age_facility_npi_and_license_plate_without_raw_previews(self):
        text = 'Patient Jane Example age 87 at facility North Valley Hospital, NPI 1234567893, license plate ABC-1234.'
        payload = detect_pii(text)
        validator = load_validator_module()

        self.assertEqual(payload['summary']['category_counts'].get('AGE'), 1)
        self.assertEqual(payload['summary']['category_counts'].get('FACILITY'), 1)
        self.assertEqual(payload['summary']['category_counts'].get('NPI'), 1)
        self.assertEqual(payload['summary']['category_counts'].get('LICENSE_PLATE'), 1)
        self.assertIn('[AGE]', payload['masked_text'])
        self.assertIn('[FACILITY]', payload['masked_text'])
        self.assertIn('[NPI]', payload['masked_text'])
        self.assertIn('[LICENSE_PLATE]', payload['masked_text'])
        rendered = json.dumps(payload, sort_keys=True)
        for raw in ('87', 'North Valley Hospital', '1234567893', 'ABC-1234'):
            self.assertNotIn(raw, rendered)
        self.assertTrue(all(span['preview'] == '<redacted>' for span in payload['spans']))
        validator.validate_privacy_filter_output(payload)

    def test_fallback_does_not_overmatch_unbounded_age_facility_npi_or_plate_like_tokens(self):
        text = 'Age 130 is out of scope; North Valley Hospital lacks facility context; NPI 1234567890 fails checksum; ABC-1234 lacks plate context.'
        payload = detect_pii(text)
        labels = [span['label'] for span in payload['spans']]
        self.assertNotIn('AGE', labels)
        self.assertNotIn('FACILITY', labels)
        self.assertNotIn('NPI', labels)
        self.assertNotIn('LICENSE_PLATE', labels)

    def test_passport_numbers_are_masked_without_overmatching_embedded_tokens(self):
        text = 'Patient Jane Example passport X12345678 reference AX12345678 and X123456789'
        payload = run_text(text)

        self.assertEqual(payload['summary']['category_counts'].get('PASSPORT'), 1)
        self.assertIn('[PASSPORT]', payload['masked_text'])
        self.assertNotIn('passport X12345678', payload['masked_text'])
        self.assertIn('AX12345678', payload['masked_text'])
        self.assertIn('X123456789', payload['masked_text'])
        passport_spans = [span for span in payload['spans'] if span['label'] == 'PASSPORT']
        self.assertEqual(len(passport_spans), 1)
        self.assertEqual(passport_spans[0]['preview'], '<redacted>')

    def test_numeric_passport_is_masked_without_overmatching_numeric_boundaries(self):
        text = 'Patient Jane Example passport 123456789 reference 1234567890 123456789A A123456789 passport 123456789-01'
        payload = run_text(text)

        self.assertEqual(payload['summary']['category_counts'].get('PASSPORT'), 1)
        self.assertIn('[PASSPORT]', payload['masked_text'])
        self.assertIn('passport [PASSPORT]', payload['masked_text'])
        self.assertIn('1234567890', payload['masked_text'])
        self.assertIn('123456789A', payload['masked_text'])
        self.assertIn('A123456789', payload['masked_text'])
        self.assertIn('passport 123456789-01', payload['masked_text'])
        passport_spans = [span for span in payload['spans'] if span['label'] == 'PASSPORT']
        self.assertEqual(len(passport_spans), 1)
        self.assertEqual(passport_spans[0]['preview'], '<redacted>')

    def test_passport_payload_is_accepted_by_validator_contract(self):
        payload = run_text('Patient Jane Example passport 123456789\n')
        validator = load_validator_module()

        self.assertEqual(payload['summary']['category_counts'].get('PASSPORT'), 1)
        validator.validate_privacy_filter_output(payload)

    def test_mrn_and_id_numeric_values_are_not_detected_as_passports(self):
        payload = run_text('Patient Jane Example MRN-123456789 and ID-123456789\n')
        validator = load_validator_module()

        self.assertEqual(payload['summary']['category_counts'].get('MRN'), 1)
        self.assertEqual(payload['summary']['category_counts'].get('ID'), 1)
        self.assertNotIn('PASSPORT', payload['summary']['category_counts'])
        self.assertNotIn('[PASSPORT]', payload['masked_text'])
        validator.validate_privacy_filter_output(payload)

    def test_mrn_and_id_alphanumeric_values_are_not_detected_as_passports(self):
        payload = run_text('Patient Jane Example MRN-X12345678 and ID-X12345678; passport X12345678\n')
        validator = load_validator_module()

        self.assertEqual(payload['summary']['category_counts'].get('MRN'), 1)
        self.assertEqual(payload['summary']['category_counts'].get('ID'), 1)
        self.assertEqual(payload['summary']['category_counts'].get('PASSPORT'), 1)
        self.assertIn('[PASSPORT]', payload['masked_text'])
        self.assertIn('[MRN]', payload['masked_text'])
        self.assertIn('[ID]', payload['masked_text'])
        self.assertNotIn('MRN-X12345678', payload['masked_text'])
        self.assertNotIn('ID-X12345678', payload['masked_text'])
        self.assertNotIn('X12345678 and ID-X12345678', payload['masked_text'])
        self.assertNotIn('MRN-[PASSPORT]', payload['masked_text'])
        self.assertNotIn('ID-[PASSPORT]', payload['masked_text'])
        passport_spans = [span for span in payload['spans'] if span['label'] == 'PASSPORT']
        self.assertEqual(len(passport_spans), 1)
        validator.validate_privacy_filter_output(payload)

    def test_spaced_mrn_and_id_alphanumeric_values_are_not_detected_as_passports(self):
        payload = run_text('Patient Jane Example MRN X12345678 and ID X12345678; passport X12345678; X12345678\n')
        validator = load_validator_module()

        self.assertEqual(payload['summary']['category_counts'].get('MRN'), 1)
        self.assertEqual(payload['summary']['category_counts'].get('ID'), 1)
        self.assertEqual(payload['summary']['category_counts'].get('PASSPORT'), 2)
        self.assertIn('[PASSPORT]', payload['masked_text'])
        self.assertIn('[MRN]', payload['masked_text'])
        self.assertIn('[ID]', payload['masked_text'])
        self.assertNotIn('MRN X12345678', payload['masked_text'])
        self.assertNotIn('ID X12345678', payload['masked_text'])
        self.assertNotIn('MRN [PASSPORT]', payload['masked_text'])
        self.assertNotIn('ID [PASSPORT]', payload['masked_text'])
        passport_spans = [span for span in payload['spans'] if span['label'] == 'PASSPORT']
        self.assertEqual(len(passport_spans), 2)
        validator.validate_privacy_filter_output(payload)

    def test_passport_context_numeric_value_creates_exactly_one_passport_span(self):
        payload = run_text('Patient Jane Example passport 123456789\n')
        passport_spans = [span for span in payload['spans'] if span['label'] == 'PASSPORT']

        self.assertEqual(payload['summary']['category_counts'].get('PASSPORT'), 1)
        self.assertEqual(len(passport_spans), 1)
        self.assertEqual(passport_spans[0]['start'], len('Patient Jane Example passport '))
        self.assertEqual(passport_spans[0]['end'], len('Patient Jane Example passport 123456789'))

    def test_street_address_number_is_not_detected_as_numeric_passport(self):
        payload = run_text('Patient Jane Example lives at 123456789 Main Street\n')

        self.assertNotIn('PASSPORT', payload['summary']['category_counts'])
        self.assertNotIn('[PASSPORT]', payload['masked_text'])

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
        with tempfile.TemporaryDirectory() as tmp:
            output_path = Path(tmp) / 'privacy-filter-output.json'
            output_path.write_text(result.stdout, encoding='utf-8')
            validator = subprocess.run(
                [sys.executable, str(REPO_ROOT / 'scripts' / 'privacy_filter' / 'validate_privacy_filter_output.py'), str(output_path)],
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=5,
                check=False,
            )
        self.assertEqual(validator.returncode, 0, validator.stderr)
        payload = json.loads(result.stdout)
        self.assertEqual(payload['metadata']['engine'], 'fallback_synthetic_patterns')
        self.assertEqual(payload['metadata']['network_api_called'], False)
        self.assertEqual(payload['summary']['category_counts'].get('ZIP'), 2)
        self.assertIn('[ZIP]', payload['masked_text'])
        rendered = json.dumps(payload, sort_keys=True)
        self.assertNotIn('02139', rendered)
        self.assertNotIn('02139-4307', rendered)
        self.assertTrue(all(span['preview'] == '<redacted>' for span in payload['spans']))

    def test_stdin_mock_does_not_detect_id_numbers_as_zip(self):
        phi = 'Patient Jane Example ID 12345 and ID-67890\n'
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
        self.assertEqual(payload['summary']['category_counts'].get('ID'), 2)
        self.assertNotIn('ZIP', payload['summary']['category_counts'])
        self.assertNotIn('[ZIP]', payload['masked_text'])

    def test_stdin_mock_does_not_detect_address_street_number_as_zip(self):
        phi = 'Patient Jane Example lives at 12345 Main St\n'
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
        self.assertEqual(payload['summary']['category_counts'].get('ADDRESS'), 1)
        self.assertNotIn('ZIP', payload['summary']['category_counts'])
        with tempfile.TemporaryDirectory() as tmp:
            output_path = Path(tmp) / 'privacy-filter-output.json'
            output_path.write_text(result.stdout, encoding='utf-8')
            validator = subprocess.run(
                [sys.executable, str(REPO_ROOT / 'scripts' / 'privacy_filter' / 'validate_privacy_filter_output.py'), str(output_path)],
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=5,
                check=False,
            )
        self.assertEqual(validator.returncode, 0, validator.stderr)

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
