"""No QEMU, privileges, sockets, downloads or network mutations in these tests."""

import base64
import importlib.util
import json
from pathlib import Path
import tempfile
import sys
from types import SimpleNamespace
import unittest
from unittest.mock import patch
from tools.vm.lab import evidence, host
from tools.vm.lab import guest_common


def valid():
    return {"schema": 1, "scope": "isolated_packet_lab", "run_id": "a" * 32, "status": "passed",
            "cases": [{"name": name, "passed": True} for name in sorted(evidence.CASES)],
            "cleanup_errors": [], "management_interfaces": ["lo"], "management_namespace": "net:[1]",
            "topology": {"namespaces": {"iglab-client": "net:[2]", "iglab-gateway": "net:[3]", "iglab-upstream": "net:[4]"}},
            "crash_guard": {"kernel_ip_forward": 1, "forward_drop_packets": 2}}


class Evidence(unittest.TestCase):
    def test_complete_matching_evidence_is_accepted(self):
        evidence.validate(valid(), "a" * 32)

    def test_failed_stale_or_incomplete_results_cannot_pass(self):
        for key, value in [("schema", 2), ("run_id", "b" * 32), ("status", "failed"),
                           ("scope", "other"), ("cleanup_errors", ["failed"]), ("management_interfaces", ["lo", "eth0"])]:
            guest = valid()
            guest[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                evidence.validate(guest, "a" * 32)

    def test_missing_duplicate_failed_or_unknown_case_is_rejected(self):
        for change in ("missing", "duplicate", "failed", "unknown"):
            guest = valid()
            if change == "missing": guest["cases"].pop()
            elif change == "duplicate": guest["cases"][0] = guest["cases"][1]
            elif change == "failed": guest["cases"][0]["passed"] = False
            else: guest["cases"][0]["name"] = "invented"
            with self.subTest(change=change), self.assertRaises(ValueError):
                evidence.validate(guest, "a" * 32)

    def test_management_namespace_or_unexercised_guard_is_rejected(self):
        guest = valid()
        guest["topology"]["namespaces"]["iglab-client"] = "net:[1]"
        with self.assertRaises(ValueError): evidence.validate(guest, "a" * 32)
        guest = valid()
        guest["crash_guard"]["forward_drop_packets"] = 0
        with self.assertRaises(ValueError): evidence.validate(guest, "a" * 32)


class SeedAndCommand(unittest.TestCase):
    def test_demo_has_no_external_nic_or_host_share_and_seed_is_read_only(self):
        command = host.qemu(Path("/tmp/example"))
        self.assertEqual(command[-2:], ["-nic", "none"])
        self.assertNotIn("-netdev", command)
        self.assertNotIn("-virtfs", command)
        self.assertNotIn("-fsdev", command)
        self.assertTrue(any("seed.img,format=raw,if=virtio,readonly=on" in arg for arg in command))

    def test_preparation_network_is_qemu_userspace_only(self):
        command = host.qemu(Path("/tmp/example"), prepare=True)
        self.assertIn("user,id=prepnet", command)
        self.assertFalse(any("tap" in arg or "bridge" in arg for arg in command))

    def test_artifacts_are_encoded_into_guest_seed_without_host_directory_mounts(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            (directory / "payload").mkdir()
            (directory / "payload/irongated").write_bytes(b"\0synthetic binary\xff")
            with patch.object(host.subprocess, "run") as run:
                host.seed(directory, "a" * 32, "test")
            data = json.loads((directory / "user-data").read_text().split("\n", 1)[1])
            artifact = next(item for item in data["write_files"] if item["path"].endswith("/irongated"))
            self.assertEqual(base64.b64decode(artifact["content"]), b"\0synthetic binary\xff")
            script = next(item["content"] for item in data["write_files"] if item["path"].endswith("start.sh"))
            self.assertNotIn("mount", script)
            self.assertIn("systemctl poweroff", script)
            self.assertIn("/opt/irongate-lab/guest.py", script)
            self.assertEqual(run.call_args.args[0][0], "cloud-localds")

    def test_absent_result_is_an_error_instead_of_stale_success(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            (directory / "serial.log").write_text("guest never completed\n")
            with self.assertRaises(RuntimeError): host.result(directory, "IG_LAB_RESULT ")


class NamespaceOwnership(unittest.TestCase):
    def module(self):
        spec = importlib.util.spec_from_file_location("guest_topology_test", host.SOURCE / "topology.py")
        module = importlib.util.module_from_spec(spec)
        with patch.dict(sys.modules, {"guest_common": guest_common}):
            spec.loader.exec_module(module)
        return module

    def test_empty_iproute_namespace_output_is_accepted_without_using_real_commands(self):
        module = self.module()
        calls = []

        def fake(*args, namespace=None, **kwargs):
            calls.append((args, namespace))
            text = ""
            if args[:3] == ("nft", "-j", "list"): text = '{"nftables":[]}'
            elif args[0] == "readlink": text = namespace
            elif args[:2] == ("sysctl", "-n"): text = "0"
            return SimpleNamespace(stdout=text, stderr="", returncode=0)

        created = []
        with patch.object(module, "command", side_effect=fake):
            result = module.prepare(created)
        self.assertEqual(created, module.NAMES)
        self.assertEqual(len(set(result["namespaces"].values())), 3)
        self.assertTrue(all(namespace in module.NAMES for args, namespace in calls if args[0] == "sysctl"))

    def test_existing_namespace_is_never_reused_or_deleted(self):
        module = self.module()
        created = []
        with patch.object(module, "command", return_value=SimpleNamespace(stdout='[{"name":"iglab-client"}]')) as command:
            with self.assertRaises(RuntimeError): module.prepare(created)
        self.assertEqual(created, [])
        self.assertEqual(command.call_count, 1)  # Only the read-only inventory.


if __name__ == "__main__":
    unittest.main()
