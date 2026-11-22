"""Bind only the two owned veth packet sockets and pass them through stdio."""

import os
from pathlib import Path
import socket
import subprocess
import sys
from guest_common import WORK, guard


def main():
    run_id = sys.argv[1]
    guard(run_id)
    if os.readlink("/proc/self/ns/net") == os.readlink("/proc/1/ns/net"):
        raise RuntimeError("Refusing packet sockets in the VM management namespace")
    sockets = []
    try:
        for interface in ("down0", "up0"):
            sock = socket.socket(socket.AF_PACKET, socket.SOCK_RAW, socket.htons(3))
            sock.bind((interface, 0))
            sock.setblocking(False)
            sockets.append(sock)
        with (WORK / "daemon.log").open("w") as log:
            proc = subprocess.Popen(["/opt/irongate-lab/irongated", "lab", run_id],
                                    stdin=sockets[0], stdout=sockets[1], stderr=log)
            (WORK / "daemon.pid").write_text(str(proc.pid))
            return proc.wait()
    finally:
        for sock in sockets:
            sock.close()


if __name__ == "__main__":
    sys.exit(main())
