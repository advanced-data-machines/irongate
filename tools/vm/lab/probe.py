"""Real client TCP socket, optionally retained across logout or daemon death."""

import json
import socket
import sys
from guest_common import namespace_guard

namespace_guard()
connection = None
stream = None
for line in sys.stdin:
    target = line.strip()
    try:
        host, port = target.split(":")
        if connection is None:
            connection = socket.create_connection((host, int(port)), timeout=0.8)
            connection.settimeout(0.8)
            stream = connection.makefile("rb")
        connection.sendall(b"GET / HTTP/1.1\r\nHost: origin.test\r\n\r\n")
        status = stream.readline(256)
        if not status.startswith(b"HTTP/1.1 200 "):
            raise RuntimeError("Unexpected HTTP status")
        length = None
        for _ in range(32):
            header = stream.readline(1024)
            if header == b"\r\n":
                break
            if header.lower().startswith(b"content-length:"):
                length = int(header.split(b":", 1)[1])
        if length is None or length > 1024:
            raise RuntimeError("Unbounded/missing fixture length")
        body = stream.read(length)
        if body != b"irongate controlled origin\n":
            raise RuntimeError("Unexpected fixture body")
        print(json.dumps({"reachable": True}), flush=True)
    except Exception as error:
        print(json.dumps({"reachable": False, "error": type(error).__name__}), flush=True)
        break
if connection:
    connection.close()
