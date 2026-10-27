"""Byte preservation, native admission and fail-closed cursor publication."""
import copy
import hashlib
import json
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch

import current_checkpoint_store as cp
import run_current_care as runner


def encoded(payload):
    return b'ALIFE-CURRENT-1\n' + struct.pack('<Q', len(payload)) + hashlib.sha256(payload).digest() + payload


def payload(pending=b'[30,31]'):
    # Deliberately retain non-Python float lexemes and strings containing seeds.
    return (b'{"schema_version":1,"seed":20,"founder_seed_base":7,"seeds":'
            b'{"historical_freshness":"unknown","excluded_ranges":[[1,20],[30,31]],"pending_range":' + pending +
            b'},"handoff":{"weights":[-0.0,1e-30,0.10000000149011612],"actor_adam":[1.25],'
            b'"value_adam":[4.5],"step":18446744073709551615},'
            b'"life":{"world_json":"\\\"pending_range\\\":[30,31]","assets_base64":{"x":"AQID"}}}')


class CursorBytesTests(unittest.TestCase):
    def test_large_learned_payload_is_not_decoded_as_cursor_metadata(self):
        # Only the small ASCII cursor belongs to this JSON parser. A complete
        # checkpoint may carry hundreds of MiB of unchanged learned/private data.
        before = payload().replace(b'"AQID"', b'"' + b'A' * (4 * 1024 * 1024) + b'"')
        decoded_lengths = []
        original_decode = json.JSONDecoder.raw_decode

        def observed_decode(decoder, text, idx=0):
            decoded_lengths.append(len(text))
            return original_decode(decoder, text, idx)

        with patch.object(json.JSONDecoder, 'raw_decode', observed_decode):
            result = runner.consumed_cursor_bytes(encoded(before), 30)
        self.assertEqual(result, encoded(before.replace(b'"pending_range":[30,31]',
                                                       b'"pending_range":[31,31]', 1)))
        self.assertTrue(decoded_lengths)
        self.assertLess(max(decoded_lengths), 1024)

    def test_only_seed_cursor_and_envelope_change(self):
        before = payload()
        result = runner.consumed_cursor_bytes(encoded(before), 30)
        expected = payload(b'[31,31]')
        self.assertEqual(result, encoded(expected))
        self.assertEqual(result[56:].split(b',"handoff":', 1)[1], before.split(b',"handoff":', 1)[1])
        self.assertEqual(runner.consumed_cursor_bytes(result, 31), encoded(payload(b'null')))

    def test_decimal_width_change_is_encoded_and_hashed_correctly(self):
        before = payload(b'[9,10]').replace(b'[30,31]],', b'[9,10]],')
        result = runner.consumed_cursor_bytes(encoded(before), 9)
        self.assertEqual(result, encoded(before.replace(b'"pending_range":[9,10]', b'"pending_range":[10,10]')))

    def test_wrong_reused_unreserved_or_noninteger_seed_rejected(self):
        for seed in (29, 31, 0, True, 30.0, 2**64):
            with self.subTest(seed=seed), self.assertRaisesRegex(ValueError, 'next owned seed'):
                runner.consumed_cursor_bytes(encoded(payload()), seed)
        with self.assertRaisesRegex(ValueError, 'next owned seed'):
            runner.consumed_cursor_bytes(encoded(payload(b'null')), 30)

    def test_corrupt_length_digest_trailing_bytes_and_limit_rejected(self):
        valid = encoded(payload())
        for damaged in (valid[:16] + b'\x00'*8 + valid[24:], valid[:24] + b'\x00'*32 + valid[56:], valid+b'!'):
            with self.assertRaisesRegex(ValueError, 'length/digest mismatch'):
                runner.consumed_cursor_bytes(damaged, 30)
        with patch.object(cp, 'MAX_BYTES', len(valid)-1), self.assertRaisesRegex(ValueError, 'length/digest mismatch'):
            runner.consumed_cursor_bytes(valid, 30)

    def test_noncanonical_or_unknown_encodings_request_native_fallback(self):
        self.assertIsNone(runner.consumed_cursor_bytes(b'legacy-or-future', 30))
        self.assertIsNone(runner.consumed_cursor_bytes(encoded(b' '+payload()), 30))
        spaced = payload().replace(b'"pending_range":[', b'"pending_range": [', 1)
        self.assertIsNone(runner.consumed_cursor_bytes(encoded(spaced), 30))
        spaced_handoff = payload().replace(b',"handoff":', b', "handoff":', 1)
        self.assertIsNone(runner.consumed_cursor_bytes(encoded(spaced_handoff), 30))

    def test_large_cursor_and_escaped_marker_remain_byte_exact(self):
        metadata = {'historical_freshness': 'unknown,"handoff":escaped',
                    'excluded_ranges': [[100 + i * 2, 100 + i * 2] for i in range(5000)],
                    'pending_range': [30, 31]}
        before = payload().split(b'"seeds":', 1)[0] + b'"seeds":'
        before += json.dumps(metadata, separators=(',', ':')).encode()
        before += b',"handoff":' + payload().split(b',"handoff":', 1)[1]
        result = runner.consumed_cursor_bytes(encoded(before), 30)
        expected = before.replace(b'"pending_range":[30,31]', b'"pending_range":[31,31]', 1)
        self.assertEqual(result, encoded(expected))

    def test_reordered_or_missing_handoff_requests_native_fallback(self):
        document = json.loads(payload())
        reordered = {name: document[name] for name in
                     ('schema_version', 'seed', 'founder_seed_base', 'seeds', 'life', 'handoff')}
        self.assertIsNone(runner.consumed_cursor_bytes(
            encoded(json.dumps(reordered, separators=(',', ':')).encode()), 30))

    def test_prefix_can_end_inside_unchanged_utf8_data(self):
        leading, rest = payload().split(b',"handoff":{', 1)
        start = len(leading.split(b'"seeds":', 1)[0]) + len(b'"seeds":')
        prefix = leading + b',"handoff":{"unchanged_text":"'
        padding = b'A' * (start + 255 - len(prefix))
        for character in ('é', '世', '🙂'):
            before = prefix + padding + character.encode() + b'",' + rest
            result = runner.consumed_cursor_bytes(encoded(before), 30)
            expected = before.replace(b'"pending_range":[30,31]', b'"pending_range":[31,31]', 1)
            self.assertEqual(result, encoded(expected))
        del reordered['handoff']
        self.assertIsNone(runner.consumed_cursor_bytes(
            encoded(json.dumps(reordered, separators=(',', ':')).encode()), 30))


class CursorAdmissionTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.source = self.root / cp.CURRENT
        self.source.write_bytes(encoded(payload()))
        self.candidate = self.root / 'candidate'
        self.report = {'kind': 'single-current-training-checkpoint', 'external_dependencies': [],
            'policy_version': 42, 'actor_optimizer_step': 100, 'value_optimizer_step': 200,
            'asset_sha256': 'immutable-founder', 'seeds': {'pending_range': [30, 31]}}
        self.calls = []
        outer = self
        class Store:
            current = outer.source
            def validate(self, path):
                outer.calls.append(path)
                report = copy.deepcopy(outer.report)
                report['seeds']['pending_range'] = json.loads(path.read_bytes()[56:])['seeds']['pending_range']
                return report
        self.store = Store()

    def test_complete_native_admission_precedes_candidate_return(self):
        result = runner.consume_current(self.store, {}, 30, self.candidate)
        self.assertEqual(self.calls, [self.source, self.candidate])
        self.assertEqual(result['seeds']['pending_range'], [31, 31])
        self.assertEqual(self.source.read_bytes(), encoded(payload()))
        self.assertEqual(self.candidate.read_bytes(), encoded(payload(b'[31,31]')))

    def test_native_rejection_leaves_original_unchanged(self):
        validate = self.store.validate
        def reject(path):
            if path == self.candidate:
                raise ValueError('native private-state rejection')
            return validate(path)
        with patch.object(self.store, 'validate', side_effect=reject), self.assertRaisesRegex(ValueError, 'native private-state'):
            runner.consume_current(self.store, {}, 30, self.candidate)
        self.assertEqual(self.source.read_bytes(), encoded(payload()))

    def test_changed_report_or_source_is_rejected(self):
        validate = self.store.validate
        def mutate_report(path):
            report = validate(path)
            if path == self.candidate:
                report['actor_optimizer_step'] += 1
            return report
        with patch.object(self.store, 'validate', side_effect=mutate_report), self.assertRaisesRegex(ValueError, 'learned-state identity'):
            runner.consume_current(self.store, {}, 30, self.candidate)
        self.candidate.unlink()
        def mutate_source(path):
            report = validate(path)
            if path == self.candidate:
                self.source.write_bytes(b'external-mutation')
            return report
        with patch.object(self.store, 'validate', side_effect=mutate_source), self.assertRaisesRegex(ValueError, 'changed during cursor admission'):
            runner.consume_current(self.store, {}, 30, self.candidate)

    def test_existing_output_is_never_overwritten(self):
        self.candidate.write_bytes(b'other-recoverable-work')
        with self.assertRaises(FileExistsError):
            runner.consume_current(self.store, {}, 30, self.candidate)
        self.assertEqual(self.candidate.read_bytes(), b'other-recoverable-work')

    def test_noncanonical_native_valid_source_uses_existing_owned_tool(self):
        self.source.write_bytes(encoded(b' '+payload()))
        with patch.object(runner, 'owned_tool', return_value={'native': True}) as native:
            self.assertEqual(runner.consume_current(self.store, {'tool': 'pinned'}, 30, self.candidate), {'native': True})
        native.assert_called_once_with(self.store, {'tool': 'pinned'}, '--consume-current', self.source, 30, self.candidate)
        self.assertFalse(self.candidate.exists())


if __name__ == '__main__':
    unittest.main()
