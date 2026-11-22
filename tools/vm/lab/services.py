"""Controlled HTTP and exact-name DNS fixtures, not production servers."""

import http.server
import socketserver
import struct
import sys
import threading
from guest_common import namespace_guard


class Http(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def handle(self):
        try:
            super().handle()
        except (ConnectionResetError, BrokenPipeError):
            pass  # Expected when revocation/crash tests abandon an open client.

    def do_GET(self):
        body = b"irongate controlled origin\n"
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass


class Dns(socketserver.BaseRequestHandler):
    def handle(self):
        query, sock = self.request
        question = b"\x06origin\x04test\0\0\x01\0\x01"
        if len(query) != 12 + len(question) or query[4:12] != b"\0\x01" + bytes(6) or query[12:] != question:
            return
        answer = query[:2] + bytes.fromhex("81800001000100000000") + question
        answer += bytes.fromhex("c00c000100010000001e0004c0000250")
        sock.sendto(answer, self.client_address)


if __name__ == "__main__":
    namespace_guard()
    host, port = sys.argv[1], int(sys.argv[2])
    if port == 80:
        dns = socketserver.ThreadingUDPServer((host, 53), Dns)
        threading.Thread(target=dns.serve_forever, daemon=True).start()
    http.server.ThreadingHTTPServer((host, port), Http).serve_forever()
