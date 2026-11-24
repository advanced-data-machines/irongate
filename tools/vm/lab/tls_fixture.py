"""Deliberately invalid certificates on owned guest-only test listeners."""
import http.server
import ssl
import sys
from guest_common import WORK,namespace_guard

namespace_guard()
port,name=sys.argv[1:]
class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        data=b"IronGate certificate negative fixture\n"
        self.send_response(200);self.send_header("Content-Length",str(len(data)));self.end_headers();self.wfile.write(data)
    def log_message(self,*args):pass
server=http.server.ThreadingHTTPServer(("10.50.10.1",int(port)),Handler)
context=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
context.load_cert_chain(WORK/f"tls/{name}.pem",WORK/f"tls/{name}.key")
server.socket=context.wrap_socket(server.socket,server_side=True)
server.serve_forever()
