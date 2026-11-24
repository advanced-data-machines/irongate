"""Browser and API admission proof from the real client namespace."""

import json
import os
import select
import signal
import subprocess
import time
from pathlib import Path
from guest_common import CLIENT,WORK,command,spawn
from browser import Browser
import portal_config
from scenarios import Probe,control,fresh,require


class Api:
    def __init__(self,children):
        self.proc=spawn("python3","/opt/irongate-lab/api_client.py",namespace=CLIENT,
            stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
        children.append(self.proc)

    def call(self,**request):
        self.proc.stdin.write(json.dumps(request)+"\n");self.proc.stdin.flush()
        require(bool(select.select([self.proc.stdout],[],[],6)[0]),"Portal API client timed out")
        line=self.proc.stdout.readline();require(bool(line),"Portal API client exited")
        return json.loads(line)

    def action(self):
        reply=self.call(path="/api/action")
        require(reply["status"]==200,"Action handoff failed")
        require(reply["headers"].get("Cache-Control")=="no-store","Token response was cacheable")
        return reply["data"]

    def post(self,action,token,expected=303,**options):
        reply=self.call(method="POST",path="/"+action,body="token="+token,**options)
        require(reply["status"]==expected,f"Portal action returned unexpected HTTP status (expected {expected}, got {reply['status']})")
        return reply["status"]

    def status(self):
        reply=self.call(path="/status");require(reply["status"]==200,"Portal status failed")
        return reply["data"]

    def close(self):
        self.proc.stdin.close();self.proc.wait(timeout=5)


def dhcp(release=False):
    args=["busybox","udhcpc","-f","-n","-i","eth0","-t","5","-T","2","-s","/opt/irongate-lab/client_lease.py"]
    if not release:
        command(*args,"-q",namespace=CLIENT,timeout=20)
        return
    # Keep the DHCP client alive until bound, then explicitly request RELEASE.
    # A -q client exits immediately after binding, before its release-on-exit path.
    marker=WORK/"client-lease.json"
    previous=marker.stat().st_mtime_ns
    with (WORK/"release-client.log").open("w") as log:
        process=spawn(*args,namespace=CLIENT,stdout=log,stderr=log)
        try:
            deadline=time.monotonic()+15
            while marker.stat().st_mtime_ns==previous:
                require(process.poll() is None and time.monotonic()<deadline,"Release client did not bind")
                time.sleep(.05)
            process.send_signal(signal.SIGUSR2)
            time.sleep(.2)
        finally:
            if process.poll() is None:process.terminate()
            process.wait(timeout=5)


def run(children,report):
    api=Api(children);browser=Browser(children)
    statuses={};report["portal_http"]=statuses
    try:
        namespace=os.readlink(f"/proc/{browser.proc.pid}/ns/net")
        require(namespace==report["topology"]["namespaces"][CLIENT],"Browser bypasses subscriber namespace")
        report["browser"]={"origin":portal_config.origin(),"certificate_verification":"normal","product":browser.version,"namespace":namespace,"transport":"devtools_pipe","sandbox":"disabled_in_disposable_guest"}
        browser.navigate();browser.wait("document.body.dataset.allowed==='false'")
        if portal_config.secure():
            require(browser.evaluate("location.protocol==='https:' && window.isSecureContext"),"Browser is not a verified secure context")
            report["tls_status"]=api.status()["transport"]
        browser.screenshot("portal-before.png")
        require(not fresh(children),"Browser client was admitted before consent")
        consumed=browser.evaluate("document.querySelector('[name=token]').value")
        browser.submit();browser.wait("document.body.dataset.allowed==='true'")
        require(fresh(children),"Browser consent did not grant actual transit")
        live=Probe(children);require(live.request(),"Browser admission existing TCP baseline failed")
        browser.screenshot("portal-connected.png")
        statuses["replay"]=api.post("consent",consumed,expected=409)
        browser.submit();browser.wait("document.body.dataset.allowed==='false'")
        require(not live.request(),"Existing TCP survived browser logout");live.close()
        require(not fresh(children),"Fresh TCP survived browser logout")
        browser.screenshot("portal-logged-out.png")
        report["cases"].append({"name":"browser_consent_and_logout","passed":True})

        action=api.action()["action"]
        statuses["origin"]=api.post("consent",action["token"],expected=403,origin="http://attacker.invalid")
        rejected=api.call(method="POST",path="/consent",body="token="+action["token"]+"&network=2")
        statuses["network_field"]=rejected["status"];require(rejected["status"]==400,"Portal accepted a client-selected network")
        statuses["wrong_action"]=api.post("logout",action["token"],expected=409)
        api.post("consent",action["token"],expected=409)
        action=api.action()["action"];api.post("consent",action["token"])
        require(api.status()["allowed"],"API consent did not authorize")
        api.post("consent",action["token"],expected=409)
        action=api.action()["action"];api.post("logout",action["token"])
        report["cases"].append({"name":"portal_one_use_origin_and_action_rejection","passed":True})

        expired=api.action()["action"]
        time.sleep(61)
        statuses["expired"]=api.post("consent",expired["token"],expected=409)
        stale=api.action();old_generation=stale["generation"]
        dhcp(release=True);dhcp()
        require(api.status()["generation"]!=old_generation,"DHCP did not reuse the session generation")
        statuses["stale_generation"]=api.post("consent",stale["action"]["token"],expected=409)
        report["cases"].append({"name":"portal_expired_and_stale_generation_actions","passed":True})

        browser.navigate();browser.submit();browser.wait("document.body.dataset.allowed==='true'")
        expiring=Probe(children);require(expiring.request(),"Portal expiry baseline failed")
        time.sleep(31)
        browser.wait("document.body.dataset.allowed==='false'")
        require(not api.status()["allowed"],"Portal admission failed to expire")
        require(not expiring.request(),"Existing TCP survived portal expiry");expiring.close()
        require(not fresh(children),"Fresh TCP survived portal expiry")
        report["cases"].append({"name":"portal_expiry_blocks_existing_and_new_tcp","passed":True})

        api.call(slow=True)
        started=time.monotonic();status=api.status()
        require(time.monotonic()-started<1,"Slow HTTP connection stalled independent status")
        require(not status["allowed"],"Slow HTTP connection changed policy")
        report["cases"].append({"name":"portal_slow_client_does_not_block_control","passed":True})

        action=api.action()["action"];api.post("consent",action["token"])
        require(api.status()["allowed"],"Journal pressure baseline was not authorized")
        control("record-pressure")
        action=api.action()["action"];require(action["kind"]=="logout","Pressure removed logout action")
        api.post("logout",action["token"])
        require(not fresh(children),"Log pressure prevented revocation")
        action=api.action();require(not action["admission_available"],"Full journal still admitted sessions")
        statuses["capacity"]=api.post("consent",action["action"]["token"],expected=503)
        require(not api.status()["allowed"],"Full journal granted access")
        deadline=time.monotonic()+5
        while time.monotonic()<deadline:
            status=api.status()
            if status["records"]["pending"]==0:break
            time.sleep(.05)
        require(status["records"]["durable"]==16,"Journal did not persist its reserved stop")
        rows=[json.loads(line)for line in Path("/var/lib/irongate-lab/local-records.jsonl").read_text().splitlines()]
        require([row["id"]for row in rows]==list(range(1,17)),"Journal prefix is missing/duplicated")
        require(rows[-1]["kind"]=="Revoked","Reserved final termination record was lost")
        require(sum(row["kind"]=="Granted"for row in rows)==4,"Grant records do not match real portal admissions")
        require(all(row["run_id"]==report["run_id"] and row["network"]==1 for row in rows),"Journal scope mismatch")
        report["local_records"]={"capacity":16,"retained":16,"durable":16,"rows":rows,
            "crash_semantics":"only_acknowledged_prefix_is_promised; unacknowledged_tail_and_crash_stop_may_be_missing"}
        report["cases"].append({"name":"portal_record_pressure_preserves_logout","passed":True})
    except Exception:
        # Rendered text excludes hidden bearer inputs; retain useful browser
        # failures without logging request bodies, tokens or full DOM markup.
        try:report["browser_failure_text"]=browser.evaluate("document.body.innerText")[0:2000]
        except Exception:pass
        raise
    finally:
        api.close();browser.close()
