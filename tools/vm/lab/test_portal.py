"""Portal evidence must reject stale, missing or corrupted browser artifacts."""
import base64
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from tools.vm.lab import artifacts, evidence
from tools.vm.lab.test_host import valid


def portal():
    guest = valid()
    guest.update(schema=3, suite="local-consent-v1",
        browser={"namespace": "net:[2]", "transport": "devtools_pipe", "product": "Chrome/test"},
        portal_http={"replay":409,"origin":403,"wrong_action":409,"network_field":400,
            "expired":409,"stale_generation":409,"capacity":503},
        local_records={"retained":16,"durable":16,"capacity":16,
            "rows":[{"id":i,"run_id":"a"*32,"network":1,
                     "kind":"Granted" if i in (1,3,5,7) else "Revoked"} for i in range(1,17)]})
    guest["cases"] += [{"name":name,"passed":True} for name in evidence.PORTAL_CASES]
    return guest


class PortalEvidence(unittest.TestCase):
    def test_complete_browser_and_record_prefix(self):
        evidence.validate(portal(), "a"*32, portal=True)

    def test_wrong_namespace_record_scope_or_missing_durability_fails(self):
        for change in ("namespace", "durable", "run", "order", "codes", "case"):
            guest = portal()
            if change == "namespace": guest["browser"]["namespace"]="net:[1]"
            elif change == "durable": guest["local_records"]["durable"]=15
            elif change == "run": guest["local_records"]["rows"][0]["run_id"]="b"*32
            elif change == "order": guest["local_records"]["rows"].reverse()
            elif change == "codes": guest["portal_http"]["origin"]=303
            else: guest["cases"].pop()
            with self.subTest(change=change), self.assertRaises(ValueError):
                evidence.validate(guest, "a"*32, portal=True)

    def test_screenshot_chunks_require_exact_run_digest_and_complete_set(self):
        data = b"\x89PNG\r\n\x1a\nfixture"
        expected = {name:hashlib.sha256(data).hexdigest() for name in artifacts.NAMES}
        items = [{"run_id":"a"*32,"name":name,"part":0,"total":1,
                  "data":base64.b64encode(data).decode()} for name in artifacts.NAMES]
        with tempfile.TemporaryDirectory() as temporary:
            directory=Path(temporary)
            def write(rows):
                (directory/"evidence.log").write_text("\n".join("IG_LAB_ARTIFACT "+json.dumps(row) for row in rows))
            write(items)
            artifacts.extract(directory,"a"*32,expected)
            self.assertEqual((directory/artifacts.NAMES[0]).read_bytes(),data)
            for change in ("duplicate","missing","run","hash"):
                rows=copy.deepcopy(items); hashes=expected.copy()
                if change=="duplicate": rows.append(rows[0])
                elif change=="missing": rows.pop()
                elif change=="run": rows[0]["run_id"]="b"*32
                else: hashes[artifacts.NAMES[0]]="0"*64
                write(rows)
                with self.subTest(change=change), self.assertRaises(ValueError):
                    artifacts.extract(directory,"a"*32,hashes)


class TLSEvidence(unittest.TestCase):
    def fixture(self):
        guest=portal();guest.update(schema=4,suite="local-https-v1")
        guest["cases"] += [{"name":name,"passed":True}for name in evidence.TLS_CASES]
        guest["browser"].update(origin="https://10.50.10.1:8443",certificate_verification="normal")
        guest["tls_certificates"]={"browser":{"expired":"net::ERR_CERT_DATE_INVALID","wrong_name":"net::ERR_CERT_COMMON_NAME_INVALID","untrusted":"net::ERR_CERT_AUTHORITY_INVALID"},"python":{name:{"rejected":True}for name in ("expired","wrong_name","untrusted")}}
        guest["tls_pressure"]={"deadline":{"closed":12,"elapsed_seconds":2.1},"oversized":{"rejected":16},"transport":{"scheme":"https","tls_timeouts":2,"tls_failures":16,"queue_rejections":1},"memory_before":{"VmRSS":1000,"VmHWM":1000},"memory_after":{"VmRSS":1000,"VmHWM":1000}}
        return guest

    def test_verified_https_evidence_requires_full_portal_contract(self):
        evidence.validate(self.fixture(),"a"*32,https=True)
        guest=self.fixture();guest["local_records"]["durable"]=0
        with self.assertRaises(ValueError):evidence.validate(guest,"a"*32,https=True)

    def test_unverified_certificates_deadlines_or_missing_memory_cannot_pass(self):
        for change in ("browser","python","deadline","memory","origin","verification"):
            guest=self.fixture()
            if change=="browser":guest["tls_certificates"]["browser"]["untrusted"]=""
            elif change=="python":guest["tls_certificates"]["python"]["expired"]["rejected"]=False
            elif change=="deadline":guest["tls_pressure"]["deadline"]["elapsed_seconds"]=10
            elif change=="memory":guest["tls_pressure"]["memory_before"]={}
            elif change=="origin":guest["browser"]["origin"]="http://10.50.10.1:8080"
            else:guest["browser"]["certificate_verification"]="disabled"
            with self.subTest(change=change),self.assertRaises(ValueError):
                evidence.validate(guest,"a"*32,https=True)
