"""TLS fault injection only in the owned subscriber namespace."""
import json
import socket
import ssl
import sys
import time
from guest_common import namespace_guard
import portal_config

namespace_guard()
mode=sys.argv[1]
if mode=="certificates":
    results={}
    for name,port in (("expired",8444),("wrong_name",8445),("untrusted",8446)):
        try:
            with socket.create_connection(("10.50.10.1",port),timeout=2) as sock:
                with portal_config.context().wrap_socket(sock,server_hostname="10.50.10.1"):
                    raise RuntimeError("Invalid certificate was accepted")
        except ssl.SSLCertVerificationError as error:
            results[name]={"rejected":True,"code":error.verify_code,"reason":error.verify_message}
    print(json.dumps(results),flush=True)
elif mode=="pressure":
    sockets=[];started=time.monotonic()
    for _ in range(2):
        sock=socket.create_connection(("10.50.10.1",8443),timeout=3)
        sock.sendall(b"\x16\x03\x03\x40\x00")
        sockets.append(sock)
    # Two occupied workers plus enough arrivals to exceed the four-cell queue.
    for _ in range(10):
        sockets.append(socket.create_connection(("10.50.10.1",8443),timeout=3))
    print(json.dumps({"opened":12}),flush=True)
    for sock in sockets:
        try:
            if sock.recv(1)!=b"":raise RuntimeError("Stalled incomplete handshake was not closed")
        except ConnectionResetError:pass
        sock.close()
    elapsed=time.monotonic()-started
    if not 1.5<elapsed<3.5:raise RuntimeError("Handshake total deadline was not enforced")
    print(json.dumps({"closed":12,"elapsed_seconds":elapsed}),flush=True)
elif mode=="oversized":
    for _ in range(16):
        with socket.create_connection(("10.50.10.1",8443),timeout=3) as sock:
            sock.sendall(b"\x16\x03\x03\xff\xff"+bytes(16))
            reply=sock.recv(256)
            if reply and reply[0]!=21:raise RuntimeError("Oversized TLS record was not rejected with an alert")
    print(json.dumps({"rejected":16}),flush=True)
else:raise RuntimeError("Unknown probe mode")
