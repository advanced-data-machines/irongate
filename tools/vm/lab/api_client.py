"""Fixed-origin HTTP client inside the subscriber namespace; never logs tokens."""
import http.client
import json
import socket
import urllib.parse
import sys
from guest_common import namespace_guard
import portal_config

namespace_guard()
slow=None
for line in sys.stdin:
    request=json.loads(line)
    try:
        if request.get("slow"):
            slow=socket.create_connection(("10.50.10.1",portal_config.port()),timeout=2)
            if portal_config.secure():slow=portal_config.context().wrap_socket(slow,server_hostname="10.50.10.1")
            slow.sendall(b"GET / HTTP/1.1\r\nHost: ")
            result={"status":200}
        else:
            if "login"in request:request.update(method="POST",path="/login",body=urllib.parse.urlencode(request["login"]))
            headers={"Accept":"application/json"}
            if request.get("method")=="POST":
                headers.update({"Origin":request.get("origin",portal_config.origin()),"Content-Type":"application/x-www-form-urlencoded"})
            conn=(http.client.HTTPSConnection("10.50.10.1",portal_config.port(),timeout=3,context=portal_config.context())if portal_config.secure() else http.client.HTTPConnection("10.50.10.1",portal_config.port(),timeout=3))
            conn.request(request.get("method","GET"),request.get("path","/status"),request.get("body"),headers)
            reply=conn.getresponse();body=reply.read(16384)
            result={"status":reply.status,"headers":dict(reply.getheaders())}
            if reply.getheader("Content-Type","").startswith("application/json"):result["data"]=json.loads(body)
            conn.close()
        print(json.dumps(result),flush=True)
    except Exception as error:print(json.dumps({"status":0,"error":type(error).__name__}),flush=True)
if slow:slow.close()
