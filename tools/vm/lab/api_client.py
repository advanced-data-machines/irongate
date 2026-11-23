"""Fixed-origin HTTP client inside the subscriber namespace; never logs tokens."""
import http.client
import json
import socket
import sys
from guest_common import namespace_guard

namespace_guard()
slow=None
for line in sys.stdin:
    request=json.loads(line)
    try:
        if request.get("slow"):
            slow=socket.create_connection(("10.50.10.1",8080),timeout=2)
            slow.sendall(b"GET / HTTP/1.1\r\nHost: ")
            result={"status":200}
        else:
            headers={"Accept":"application/json"}
            if request.get("method")=="POST":
                headers.update({"Origin":request.get("origin","http://10.50.10.1:8080"),"Content-Type":"application/x-www-form-urlencoded"})
            conn=http.client.HTTPConnection("10.50.10.1",8080,timeout=3)
            conn.request(request.get("method","GET"),request.get("path","/status"),request.get("body"),headers)
            reply=conn.getresponse();body=reply.read(16384)
            result={"status":reply.status,"headers":dict(reply.getheaders())}
            if reply.getheader("Content-Type","").startswith("application/json"):result["data"]=json.loads(body)
            conn.close()
        print(json.dumps(result),flush=True)
    except Exception as error:print(json.dumps({"status":0,"error":type(error).__name__}),flush=True)
if slow:slow.close()
