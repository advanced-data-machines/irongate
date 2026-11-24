"""Verified browser certificates and TLS pressure with independent packet probes."""
import json
import select
import subprocess
import time
from pathlib import Path
from guest_common import WORK,CLIENT,GATEWAY,command,spawn
from browser import Browser
from portal_scenarios import Api
from scenarios import control,fresh,require


def memory(pid):
    fields={}
    for line in Path(f"/proc/{pid}/status").read_text().splitlines():
        if line.startswith(("VmRSS:","VmHWM:")):
            name,value,*_=line.split();fields[name[:-1]]=int(value)*1024
    return fields


def run(children,report):
    for port,name in ((8444,"expired"),(8445,"wrong-name"),(8446,"untrusted-server")):
        log=(WORK/f"tls-fixture-{port}.log").open("w")
        try:
            children.append(spawn("python3","/opt/irongate-lab/tls_fixture.py",str(port),name,
                namespace=GATEWAY,stdout=log,stderr=log))
        finally:log.close()
    time.sleep(.2)
    browser=Browser(children,label="tls");api=Api(children)
    try:
        negatives={}
        for name,port,expected in (("expired",8444,"ERR_CERT_DATE_INVALID"),
                ("wrong_name",8445,"ERR_CERT_COMMON_NAME_INVALID"),("untrusted",8446,"ERR_CERT_AUTHORITY_INVALID")):
            reply=browser.call("Page.navigate",{"url":f"https://10.50.10.1:{port}/"})
            error=reply.get("errorText","")
            require(expected in error,f"Browser did not reject {name} certificate")
            negatives[name]=error
        report["tls_certificates"]={"browser":negatives,"python":json.loads(command("python3","/opt/irongate-lab/tls_probe.py","certificates",namespace=CLIENT).stdout)}
        require(not fresh(children,"10.50.10.1:8080"),"HTTPS mode exposed an HTTP fallback")
        report["cases"].append({"name":"tls_certificate_validation_and_no_http_fallback","passed":True})
        pid=int((WORK/"daemon.pid").read_text())
        baseline=memory(pid)
        pressure=spawn("python3","/opt/irongate-lab/tls_probe.py","pressure",namespace=CLIENT,
            stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
        children.append(pressure)
        require(bool(select.select([pressure.stdout],[],[],4)[0]),"TLS pressure did not open")
        require(json.loads(pressure.stdout.readline())["opened"]==12,"TLS pressure setup failed")
        started=time.monotonic();status=control("status")
        require(time.monotonic()-started<.5,"TLS handshakes blocked admission owner")
        require(not status["allowed"],"TLS handshake authorized transit")
        require(control("consent")["allowed"] and fresh(children),"TLS pressure blocked authorized transit")
        require(not control("logout")["allowed"],"TLS pressure blocked logout")
        require(bool(select.select([pressure.stdout],[],[],5)[0]),"TLS pressure did not expire")
        deadline=json.loads(pressure.stdout.readline());pressure.wait(timeout=3)
        require(pressure.returncode==0,"TLS pressure probe failed")
        oversized=json.loads(command("python3","/opt/irongate-lab/tls_probe.py","oversized",namespace=CLIENT).stdout)
        status=api.status()
        require(status["transport"]["tls_timeouts"]>=2,"TLS deadline diagnostics missing")
        require(status["transport"]["queue_rejections"]>=1,"TLS accepted queue overflow was not observable")
        require(status["transport"]["tls_failures"]>=16,"TLS malformed-record diagnostics missing")
        require(not status["allowed"],"TLS failure authorized transit")
        report["tls_pressure"]={"deadline":deadline,"oversized":oversized,"transport":status["transport"],
            "memory_before":baseline,"memory_after":memory(pid),"scope":"one_client_functional_measurement_not_capacity_qualification"}
        report["cases"].append({"name":"tls_handshake_deadline_pressure_and_memory","passed":True})
    finally:api.close();browser.close()
