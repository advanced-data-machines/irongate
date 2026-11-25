"""One real FreeRADIUS service, owned entirely by this disposable guest."""
import json
import secrets
import subprocess
from guest_common import WORK,UPSTREAM,command,guard,spawn


def configure(run_id):
    guard(run_id)
    command("systemctl","stop","freeradius",check=False)
    command("systemctl","mask","--runtime","freeradius",check=False)
    root=WORK/"radius";root.mkdir(mode=0o700)
    secret=secrets.token_hex(32)
    (root/"secret").write_text(secret+"\n");(root/"secret").chmod(0o600)
    (root/"dictionary").write_text("")
    (root/"users").write_text('lab-user Cleartext-Password := "lab-pass"\n    Session-Timeout := 5\n\nunsupported-user Cleartext-Password := "lab-pass"\n    Session-Timeout := 5,\n    Filter-Id := "unsupported-policy"\n')
    config=f'''name = irongate-lab
prefix = /usr
localstatedir = /var
logdir = {root}
raddbdir = {root}
dictdir = /usr/share/freeradius
pidfile = {root}/server.pid
max_request_time = 5
cleanup_delay = 1
max_requests = 16
hostname_lookups = no
log {{
 destination = stdout
 file = {WORK}/radius-server.log
 auth = no
 auth_badpass = no
 auth_goodpass = no
}}
security {{
 user = root
 group = root
 allow_core_dumps = no
 max_attributes = 64
 reject_delay = 0
 status_server = no
}}
client irongate {{
 ipaddr = 192.0.2.1
 secret = "{secret}"
 require_message_authenticator = yes
 limit_proxy_state = yes
}}
modules {{
 files {{
  filename = {root}/users
 }}
 pap {{
 }}
}}
server default {{
 listen {{
  type = auth
  ipaddr = 192.0.2.80
  port = 1812
 }}
 authorize {{
  files
  pap
 }}
 authenticate {{
  Auth-Type PAP {{
   pap
  }}
 }}
}}
'''
    if (WORK/"mode").read_text().strip()=="accounting":
        config=config.replace("modules {", f"modules {{\n detail {{\n  filename = {root}/accounting-detail\n  permissions = 0600\n  locking = yes\n }}")
        config=config.replace("server default {", "server default {\n listen {\n  type = acct\n  ipaddr = 192.0.2.80\n  port = 1813\n }\n accounting {\n  detail\n }")
        (root/"users").write_text((root/"users").read_text().replace("Session-Timeout := 5", "Session-Timeout := 30"))
    (root/"radiusd.conf").write_text(config)
    for path in root.iterdir():path.chmod(0o600)


def start(children):
    # Validate without -X/debug output: credentials and shared secrets must not
    # appear in retained command or daemon logs.
    checked=command("freeradius","-C","-l","stdout","-d",str(WORK/"radius"),namespace=UPSTREAM,check=False)
    if checked.returncode:
        secret=(WORK/"radius/secret").read_text().strip()
        detail=(checked.stdout+checked.stderr).replace(secret,"<redacted>").replace("lab-pass","<test-password>")
        raise RuntimeError("FreeRADIUS startup validation failed: "+detail[-2500:])
    log=(WORK/"radius-process.log").open("w")
    try:process=spawn("freeradius","-f","-l","stdout","-d",str(WORK/"radius"),namespace=UPSTREAM,stdout=log,stderr=log)
    finally:log.close()
    children.append(process)
    return process
