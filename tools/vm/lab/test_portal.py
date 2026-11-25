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

class RadiusEvidence(unittest.TestCase):
    def fixture(self):
        guest=TLSEvidence().fixture();guest.update(schema=5,suite="radius-login-v1")
        guest["cases"]=[c for c in guest["cases"]if c["name"]not in evidence.PORTAL_CASES]+[{"name":name,"passed":True}for name in evidence.RADIUS_CASES]
        guest["radius_faults"]={"http":{"bad_auth":502,"bad_ma":502,"missing_ma":502,"wrong_id":502,"length":502,"wrong_source":503,"timeout":503,"challenge":422}}
        guest["radius_late"]={"http":409,"old_generation":1,"new_generation":2}
        guest["radius_auth"]={"health":"responsive","access_accepts":3,"access_rejects":1,"accounting":"not_implemented"}
        guest["radius_packages"]=["freeradius=test-version"]
        guest["local_records"]={"capacity":16,"durable":6,"retained":6,"scope":"local_audit_not_radius_accounting","rows":[{"id":i,"network":1,"run_id":"a"*32,"provider":"radius","kind":kind}for i,kind in enumerate(["Granted","Revoked","Granted","Expired","Granted","Revoked"],1)]}
        return guest

    def test_complete_login_contract_requires_tls_and_scoped_local_audit(self):
        evidence.validate(self.fixture(),"a"*32,radius=True)
        for change in ("tls","source","stale","accounting","identity","backend"):
            guest=self.fixture()
            if change=="tls":guest["browser"]["certificate_verification"]="disabled"
            elif change=="source":guest["local_records"]["rows"][0]["provider"]="consent"
            elif change=="stale":guest["radius_late"]["new_generation"]=1
            elif change=="accounting":guest["radius_auth"]["accounting"]="healthy"
            elif change=="identity":guest["local_records"]["rows"][0]["run_id"]="b"*32
            else:guest["radius_packages"]=[]
            with self.subTest(change=change),self.assertRaises(ValueError):evidence.validate(guest,"a"*32,radius=True)


class AccountingEvidence(unittest.TestCase):
    def fixture(self):
        guest=TLSEvidence().fixture();guest.update(schema=6,suite="radius-accounting-v1",radius_packages=["freeradius=test-version"])
        guest["cases"]=[c for c in guest["cases"]if c["name"]not in evidence.PORTAL_CASES]+[{"name":name,"passed":True}for name in evidence.ACCOUNTING_CASES]
        expected={"input":64*1028,"output":256*1228,"overflow":False}
        guest["accounting_traffic"]={"count_point":"successfully_submitted_ip_octets","expected":expected,"up":{"ip_octets":expected["input"],"packets":64},"down":{"ip_octets":expected["output"],"packets":256}}
        guest["accounting_idle"]={"current_octets":{"input":0,"output":0,"overflow":False},"acknowledged":2}
        guest["accounting_outage"]={"new_login_http":503,"logout_seconds":.1,"before_logout":{"state":"unavailable"},"after_logout":{"retained_lifecycles":1,"pending":1}}
        guest["accounting_recovery"]={"state":"responsive","retained_lifecycles":0,"last_acknowledged_octets":expected,"storage":"memory","crash_replay":False,"failed_attempts":1,"acknowledged":8}
        guest["accounting_rows"]=[{"session_id":"unique-id","kind":kind,"input":expected["input"],"output":expected["output"],"identity_matches":True,"delay":2,"seconds":2}for kind in ("Start","Interim-Update","Stop")]
        guest["accounting_final_rows"]=copy.deepcopy(guest["accounting_rows"])+[{"session_id":"second-id","kind":kind,"identity_matches":True}for kind in ("Start","Stop")]
        guest["accounting_final"]={"retained_lifecycles":0,"state":"responsive","acknowledged":10}
        return guest

    def test_complete_accounting_contract(self):
        evidence.validate(self.fixture(),"a"*32,accounting=True)

    def test_contradictory_usage_identity_outage_or_crash_claim_cannot_pass(self):
        for change in ("counter","identity","delay","outage","crash","idle","backend","case","reused"):
            guest=self.fixture()
            if change=="counter":guest["accounting_rows"][-1]["input"]+=1
            elif change=="identity":guest["accounting_rows"][-1]["session_id"]="other-session"
            elif change=="delay":guest["accounting_rows"][-1]["delay"]=0
            elif change=="outage":guest["accounting_outage"]["after_logout"]["pending"]=0
            elif change=="crash":guest["accounting_recovery"]["crash_replay"]=True
            elif change=="idle":guest["accounting_idle"]["current_octets"]["input"]=1
            elif change=="backend":guest["radius_packages"]=[]
            elif change=="reused":guest["accounting_final_rows"][-1]["session_id"]="unique-id"
            else:guest["cases"].pop()
            with self.subTest(change=change),self.assertRaises(ValueError):evidence.validate(guest,"a"*32,accounting=True)
