"""Generate fresh guest-only test PKI; trust one CA in the guest user's NSS DB."""
from pathlib import Path
import base64
import json
from datetime import datetime,timedelta,timezone
from guest_common import WORK,command,guard


def prepare(run_id):
    guard(run_id)
    directory=WORK/"tls"
    directory.mkdir(mode=0o700)
    def openssl(*args):
        command("openssl",*args,timeout=15)
    for ca in ("ca","untrusted"):
        openssl("req","-x509","-newkey","ec","-pkeyopt","ec_paramgen_curve:P-256","-nodes",
            "-keyout",str(directory/f"{ca}.key"),"-out",str(directory/f"{ca}.pem"),"-days","2",
            "-subj",f"/CN=IronGate lab {run_id} {ca}","-addext","basicConstraints=critical,CA:TRUE",
            "-addext","keyUsage=critical,keyCertSign,cRLSign")
    for name,san,days,ca in (("server","IP:10.50.10.1","1","ca"),
            ("wrong-name","IP:10.50.10.2","1","ca"),("expired","IP:10.50.10.1","-1","ca"),
            ("untrusted-server","IP:10.50.10.1","1","untrusted")):
        prefix=directory/name
        openssl("req","-new","-newkey","ec","-pkeyopt","ec_paramgen_curve:P-256","-nodes",
            "-keyout",str(prefix)+".key","-out",str(prefix)+".csr","-subj","/CN=IronGate test portal")
        ext=directory/f"{name}.ext"
        ext.write_text(f"basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=serverAuth\nsubjectAltName={san}\n")
        validity=["-days",days]
        if name=="expired":
            now=datetime.now(timezone.utc)
            validity=["-not_before",(now-timedelta(days=2)).strftime("%Y%m%d%H%M%SZ"),"-not_after",(now-timedelta(days=1)).strftime("%Y%m%d%H%M%SZ")]
        openssl("x509","-req","-in",str(prefix)+".csr","-CA",str(directory/f"{ca}.pem"),
            "-CAkey",str(directory/f"{ca}.key"),"-CAcreateserial","-out",str(prefix)+".pem",
            *validity,"-extfile",str(ext))
    openssl("x509","-in",str(directory/"server.pem"),"-outform","DER","-out",str(directory/"server.der"))
    openssl("pkcs8","-topk8","-nocrypt","-in",str(directory/"server.key"),"-outform","DER","-out",str(directory/"server.key.der"))
    for path in directory.iterdir():
        path.chmod(0o600)
    # One explicit trust anchor in Chromium's managed policy inside this guest.
    # No certificate-error override, host mount or workstation trust change.
    openssl("x509","-in",str(directory/"ca.pem"),"-outform","DER","-out",str(directory/"ca.der"))
    policies=Path("/etc/chromium/policies/managed")
    policies.mkdir(parents=True,mode=0o700,exist_ok=True)
    policy=policies/"irongate-lab.json"
    policy.write_text(json.dumps({"CACertificates":[base64.b64encode((directory/"ca.der").read_bytes()).decode()]}))
    policy.chmod(0o600)
