"""One real backend, enforced short policy and independently labeled wire faults."""
import json
import os
import select
import subprocess
import time
from pathlib import Path
from guest_common import WORK,CLIENT,UPSTREAM,spawn
from browser import Browser
from portal_scenarios import Api,dhcp
from scenarios import require,control,Probe,fresh
import radius_backend


def login(api,user="lab-user",password="lab-pass",expected=303):
    action=api.action()["action"]
    require(action["kind"]=="login","RADIUS profile offered local consent")
    reply=api.call(login={"token":action["token"],"username":user,"password":password})
    require(reply["status"]==expected,f"Unexpected login HTTP status: {reply['status']} (expected {expected})")
    return reply


def mode(value):
    (WORK/"radius/fault.json").write_text(json.dumps({"mode":value}))


def stop(process):
    if process.poll() is None:process.terminate()
    process.wait(timeout=5)


def fault_server(children):
    log=(WORK/"radius-fault.log").open("w")
    try:process=spawn("python3","/opt/irongate-lab/radius_fault.py",namespace=UPSTREAM,stdout=log,stderr=log)
    finally:log.close()
    children.append(process);time.sleep(.1);return process


def run(children,report):
    api=Api(children);browser=Browser(children,label="radius")
    try:
        ns=os.readlink(f"/proc/{browser.proc.pid}/ns/net")
        require(ns==report["topology"]["namespaces"][CLIENT],"Browser is outside subscriber namespace")
        report["browser"]={"product":browser.version,"namespace":ns,"transport":"devtools_pipe","origin":"https://10.50.10.1:8443","certificate_verification":"normal","sandbox":"disabled_in_disposable_guest"}
        browser.navigate();require(browser.evaluate("window.isSecureContext && location.protocol==='https:'"),"Browser has no verified HTTPS context")
        browser.screenshot("portal-before.png");require(not fresh(children),"Unauthenticated subscriber had transit")
        consumed=browser.evaluate("document.querySelector('[name=token]').value")
        browser.evaluate("document.querySelector('[name=username]').value='lab-user';document.querySelector('[name=password]').value='lab-pass';true")
        browser.submit();browser.wait("document.body.dataset.allowed==='true'")
        status=api.status();require(status["policy_source"]=="radius" and status["authentication"]["last_outcome"]=="access_accept","Admission did not come from validated RADIUS")
        live=Probe(children);require(live.request() and fresh(children),"RADIUS Accept did not grant real transit")
        browser.screenshot("portal-connected.png")
        browser.submit();browser.wait("document.body.dataset.allowed==='false'")
        require(not live.request() and not fresh(children),"RADIUS logout left traffic authorized");live.close()
        browser.screenshot("portal-logged-out.png")
        report["cases"].append({"name":"browser_radius_accept_and_logout","passed":True})
        login(api,password="wrong-password",expected=401)
        status=api.status();require(not status["allowed"] and status["authentication"]["health"]=="responsive" and status["authentication"]["last_outcome"]=="access_reject","A valid Reject was reported as an outage")
        login(api);expiring=Probe(children);require(expiring.request(),"Policy expiry baseline failed")
        time.sleep(5.2)
        require(not api.status()["allowed"] and not expiring.request() and not fresh(children),"Session-Timeout did not revoke actual access");expiring.close()
        report["cases"].append({"name":"radius_reject_and_session_timeout","passed":True})
        login(api,user="unsupported-user",expected=422)
        require(not api.status()["allowed"],"Unsupported backend policy granted unrestricted access")
        report["cases"].append({"name":"radius_unsupported_policy_denied","passed":True})
        action=api.action()["action"]
        require(api.post("consent",action["token"],expected=403)==403,"RADIUS fell back to public consent")
        require(api.call(login={"token":consumed,"username":"lab-user","password":"lab-pass"})["status"]==409,"Consumed browser login action replayed")
        report["cases"].append({"name":"radius_no_consent_fallback_or_replay","passed":True})
        backend=next(p for p in children if p.pid==report["radius_process"]);stop(backend)
        mode("timeout");fault=fault_server(children)
        codes={};validated_before=api.status()["authentication"]["validated_at"]
        for value,expected in (("bad_auth",502),("bad_ma",502),("missing_ma",502),("wrong_id",502),("length",502),("wrong_source",503),("timeout",503)):
            mode(value);codes[value]=login(api,expected=expected)["status"]
            status=api.status();require(not status["allowed"] and status["authentication"]["validated_at"]==validated_before,"Invalid/absent reply authorized access or refreshed valid health")
        mode("challenge");codes["challenge"]=login(api,expected=422)["status"]
        require(not api.status()["allowed"],"Unsupported challenge granted access")
        report["radius_faults"]={"fixture":"synthetic_wire_faults_after_real_freeradius_interop","http":codes}
        report["cases"].append({"name":"radius_protocol_faults_deny_and_report","passed":True})
        mode("late");old_generation=api.status()["generation"];late=Api(children)
        try:
            action=api.action()["action"]
            late.proc.stdin.write(json.dumps({"login":{"token":action["token"],"username":"lab-user","password":"lab-pass"}})+"\n");late.proc.stdin.flush()
            deadline=time.monotonic()+2
            while time.monotonic()<deadline:
                observed=json.loads((WORK/"radius/fault-observed.json").read_text())
                if observed["mode"]=="late":break
                time.sleep(.02)
            require(observed["mode"]=="late","Late-result fixture did not receive login")
            control("logout");dhcp(release=True);dhcp()
            require(bool(select.select([late.proc.stdout],[],[],4)[0]),"Late login did not complete")
            reply=json.loads(late.proc.stdout.readline());require(reply["status"]==409,"Late valid backend result was not rejected as stale")
            status=api.status();require(status["generation"]!=old_generation and not status["allowed"],"Old authentication authorized reused lease/session")
            report["radius_late"]={"old_generation":old_generation,"new_generation":status["generation"],"http":reply["status"]}
        finally:late.close()
        report["cases"].append({"name":"radius_late_result_cannot_authorize_reused_lease","passed":True})
        stop(fault);backend=radius_backend.start(children);time.sleep(.2)
        login(api);require(fresh(children),"Real backend recovery did not restore login")
        action=api.action()["action"];api.post("logout",action["token"])
        require(not fresh(children),"Recovery logout failed")
        report["radius_auth"]=api.status()["authentication"]
        require(report["radius_auth"]["access_accepts"]==3 and report["radius_auth"]["health"]=="responsive","Authentication health did not recover")
        report["cases"].append({"name":"radius_backend_recovery","passed":True})
        deadline=time.monotonic()+4
        while time.monotonic()<deadline:
            status=api.status()
            if status["records"]["pending"]==0:break
            time.sleep(.05)
        rows=[json.loads(line)for line in Path("/var/lib/irongate-lab/local-records.jsonl").read_text().splitlines()]
        require(len(rows)==6 and all(row["provider"]=="radius" for row in rows),"Local records do not match RADIUS policy lifecycles")
        report["local_records"]={"capacity":16,"retained":6,"durable":status["records"]["durable"],"rows":rows,"scope":"local_audit_not_radius_accounting"}
    except Exception:
        try:report["browser_failure_text"]=browser.evaluate("document.body.innerText")[:2000]
        except Exception:pass
        raise
    finally:api.close();browser.close()
