"""Independent AF_PACKET observation in one owned guest namespace."""

import json
import select
import socket
import sys
import time
from guest_common import namespace_guard

namespace_guard()
interface, destination, marker = sys.argv[1:]
destination, marker = bytes.fromhex(destination), bytes.fromhex(marker)
with socket.socket(socket.AF_PACKET, socket.SOCK_RAW, socket.htons(3)) as sock:
    sock.bind((interface, 0))
    print(json.dumps({"ready": True}), flush=True)
    if sys.stdin.readline().strip() != "capture":
        raise SystemExit("Missing capture barrier")
    print(json.dumps({"armed": True}), flush=True)
    deadline, received = time.monotonic() + 0.5, 0
    while time.monotonic() < deadline:
        if select.select([sock], [], [], max(0, deadline - time.monotonic()))[0]:
            frame = sock.recv(4096)
            if frame[:6] == destination and marker in frame[14:]:
                received += 1
    print(json.dumps({"received": received}), flush=True)
