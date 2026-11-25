"""Real accounting sink, independent octet workload and bounded outage recovery."""
import json
import os
import select
import subprocess
import time
from guest_common import WORK,CLIENT,UPSTREAM,command,spawn
from portal_scenarios import Api
from radius_scenarios import login
from browser import Browser
from scenarios import require,fresh
import accounting_backend as backend


def wait(api,predicate,message,seconds=15):
    deadline=time.monotonic()+seconds
    while time.monotonic()<deadline:
        status=api.status()
        if predicate(status):return status
        time.sleep(.05)
    raise RuntimeError(message+": "+json.dumps(status["accounting"]))


def workload(children,receiver_ns,sender_ns,address,count,length,interval):
    receiver=spawn("python3","/opt/irongate-lab/accounting_traffic.py","receive",address,str(count),str(length),"0",namespace=receiver_ns,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
    children.append(receiver)
    require(bool(select.select([receiver.stdout],[],[],4)[0])and receiver.stdout.readline().strip()=="ready","Octet receiver did not become ready")
    sent=json.loads(command("python3","/opt/irongate-lab/accounting_traffic.py","send",address,str(count),str(length),str(interval),namespace=sender_ns).stdout)
    require(bool(select.select([receiver.stdout],[],[],6)[0]),"Octet receiver did not complete")
    line=receiver.stdout.readline();receiver.wait(timeout=3)
    require(receiver.returncode==0 and bool(line),"Octet receiver failed")
    received=json.loads(line);require(sent==received,"Independent UDP workload lost packets")
    return received


def run(children,report):
    api=Api(children);browser=Browser(children,label="accounting");dropping=False
    try:
        ns=os.readlink(f"/proc/{browser.proc.pid}/ns/net")
        require(ns==report["topology"]["namespaces"][CLIENT],"Browser namespace mismatch")
        report["browser"]={"product":browser.version,"namespace":ns,"transport":"devtools_pipe","origin":"https://10.50.10.1:8443","certificate_verification":"normal"}
        browser.navigate();browser.screenshot("portal-before.png")
        browser.evaluate("document.querySelector('[name=username]').value='lab-user';document.querySelector('[name=password]').value='lab-pass';true")
        browser.submit();browser.wait("document.body.dataset.allowed==='true'");browser.screenshot("portal-connected.png")
        wait(api,lambda s:s["accounting"]["acknowledged"]>=1,"Start was not acknowledged")
        # Native status/TLS traffic must not become subscriber transit usage.
        before=api.status()["accounting"]["current_octets"]
        time.sleep(2.2)
        idle=wait(api,lambda s:s["accounting"]["acknowledged"]>=2,"Idle Interim was not acknowledged")
        require(idle["accounting"]["current_octets"]==before=={"input":0,"output":0,"overflow":False},"Native traffic leaked into session usage")
        report["accounting_idle"]=idle["accounting"]
        report["cases"].append({"name":"accounting_start_and_idle_interim","passed":True})
        # 64 packets at about 64 kbit/s, then a larger reverse burst.
        up=workload(children,UPSTREAM,CLIENT,"192.0.2.80",64,1000,.125)
        down=workload(children,CLIENT,UPSTREAM,"10.50.10.100",256,1200,.001)
        expected={"input":up["ip_octets"],"output":down["ip_octets"],"overflow":False}
        status=api.status();require(status["accounting"]["current_octets"]==expected,"Owner directional octets differ from independent workload")
        wait(api,lambda s:s["accounting"]["last_acknowledged_octets"]==expected,"Final traffic Interim was not acknowledged")
        report["accounting_traffic"]={"up":up,"down":down,"expected":expected,"count_point":"successfully_submitted_ip_octets"}
        report["cases"].append({"name":"accounting_directional_octets_low_rate_and_burst","passed":True})
        backend.drop_responses(True);dropping=True
        failed=wait(api,lambda s:s["accounting"]["state"]=="unavailable","Accounting outage remained invisible",seconds=5)
        require(failed["allowed"] and failed["authentication"]["health"]=="responsive","Accounting outage corrupted existing authorization or auth health")
        before=time.monotonic();action=api.action()["action"];api.post("logout",action["token"])
        elapsed=time.monotonic()-before
        require(elapsed<1 and not api.status()["allowed"] and not fresh(children),"Accounting outage delayed traffic revocation")
        browser.navigate();browser.screenshot("portal-logged-out.png")
        backlog=api.status()["accounting"];require(backlog["retained_lifecycles"]==1 and backlog["pending"]>=1,"Stop obligation was lost")
        login(api,expected=503)
        require(not api.status()["allowed"],"Known accounting outage admitted another session")
        report["accounting_outage"]={"before_logout":failed["accounting"],"after_logout":backlog,"logout_seconds":elapsed,"new_login_http":503}
        report["cases"].append({"name":"accounting_outage_retains_stop_and_logout_is_immediate","passed":True})
        backend.drop_responses(False);dropping=False
        recovered=wait(api,lambda s:s["accounting"]["retained_lifecycles"]==0 and s["accounting"]["state"]=="responsive","Retained Stop did not recover",seconds=20)
        require(recovered["accounting"]["last_acknowledged_octets"]==expected,"Recovery changed cumulative usage")
        rows=backend.rows();sessions={row["session_id"]for row in rows};require(len(sessions)==1 and all(row["identity_matches"]for row in rows),"Backend lifecycle identity mismatch")
        kinds=[row["kind"]for row in rows];require(kinds[0]=="Start" and kinds[-1]=="Stop" and "Interim-Update"in kinds,"Backend Start/Interim/Stop missing")
        require(all(row["input"]==expected["input"] and row["output"]==expected["output"]for row in rows if row["kind"]=="Stop"),"Backend final octets differ from independent workload")
        require(rows[-1]["delay"]>=1,"Recovery did not report export delay")
        report["accounting_rows"]=rows;report["accounting_recovery"]=recovered["accounting"]
        report["cases"].append({"name":"accounting_backend_identity_totals_and_recovery","passed":True})
        login(api);require(fresh(children),"Recovered sink did not permit new admission")
        action=api.action()["action"];api.post("logout",action["token"])
        final=wait(api,lambda s:s["accounting"]["retained_lifecycles"]==0,"Second lifecycle did not drain")
        rows=backend.rows();require(len({r["session_id"]for r in rows})==2,"Reauthentication reused accounting identity")
        report["accounting_final_rows"]=rows
        report["accounting_final"]=final["accounting"];report["cases"].append({"name":"accounting_reauthentication_uses_new_identity","passed":True})
    finally:
        if dropping:backend.drop_responses(False)
        api.close();browser.close()
