"""One synthetic guest frame or real IPv6 datagram; never run on the host."""

import socket
import sys
from guest_common import namespace_guard

namespace_guard()
mode, value = sys.argv[1:]
if mode == "frame":
    with socket.socket(socket.AF_PACKET, socket.SOCK_RAW) as sock:
        sock.bind(("eth0", 0))
        sock.send(bytes.fromhex(value))
elif mode == "ipv6":
    with socket.socket(socket.AF_INET6, socket.SOCK_DGRAM) as sock:
        sock.sendto(bytes.fromhex(value), ("fd00:50:2::80", 9001))
else:
    raise SystemExit("Unsupported injection mode")
