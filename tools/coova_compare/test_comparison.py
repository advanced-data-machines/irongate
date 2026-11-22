"""Guard the extraction and independent comparison oracle against false matches."""

import unittest

from corpus import fixture
from source import function
from wire import checksum, compare_offers, decode


def repair_udp(frame):
    data = bytearray(frame)
    udp = data[34:]
    udp[6:8] = bytes(2)
    pseudo = data[26:34] + bytes([0, 17]) + len(udp).to_bytes(2, "big")
    value = checksum(pseudo + udp) or 65535
    data[40:42] = value.to_bytes(2, "big")
    return bytes(data)


class ComparisonTests(unittest.TestCase):
    def test_extraction_preserves_text_and_ignores_braces_in_comments_and_strings(self):
        body = 'int target(int x) { /* } */ const char *s = "}"; if (x) { x++; } return x; }'
        source = '// intro\n' + body + '\nint unrelated(void) {return 0;}'
        line, extracted = function(source, "target")
        self.assertEqual((line, extracted), (2, body))

    def test_missing_original_function_fails_instead_of_substituting(self):
        with self.assertRaises(ValueError):
            function("int other(void) { return 0; }", "target")

    def test_known_fixture_decodes_and_identical_output_matches(self):
        frame = fixture("offer")
        decoded = decode(frame)
        self.assertTrue(decoded["udp_checksum_valid"])
        self.assertEqual(dict(decoded["options"])[53], "02")
        self.assertEqual(compare_offers(frame, frame)["differences"], [])

    def test_checksum_corruption_fails(self):
        data = bytearray(fixture("offer"))
        data[300] ^= 1
        with self.assertRaises(ValueError):
            decode(data)

    def test_valid_checksum_does_not_hide_wrong_transaction_id(self):
        frame = fixture("offer")
        modified = bytearray(frame)
        modified[46] ^= 1
        with self.assertRaises(AssertionError):
            compare_offers(frame, repair_udp(modified))

    def test_valid_checksum_does_not_hide_wrong_message_type(self):
        frame = fixture("offer")
        modified = bytearray(frame)
        modified[284] = 3
        with self.assertRaises(AssertionError):
            compare_offers(frame, repair_udp(modified))

    def test_vlan_mismatch_is_not_normalized_away(self):
        frame = fixture("offer-vlan-client-id")
        modified = bytearray(frame)
        modified[15] ^= 1
        with self.assertRaises(AssertionError):
            compare_offers(frame, bytes(modified))


if __name__ == "__main__":
    unittest.main()
