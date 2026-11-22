"""Independent synthetic untagged ARP and routed TCP fixtures for the VM lab."""

from pathlib import Path
import runpy
import struct

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "crates/irongate-proto/tests/fixtures"
legacy = runpy.run_path(str(ROOT / "scripts/generate-protocol-fixtures.py"))
checksum = legacy["checksum"]
client = bytes.fromhex("020000000002")
gateway = bytes.fromhex("020000000001")
wan = bytes.fromhex("020000000101")
upstream = bytes.fromhex("020000000102")
source, target = bytes([10, 50, 10, 100]), bytes([192, 0, 2, 80])
server = bytes([10, 50, 10, 1])


def tcp_frame(ttl, src_mac, dst_mac):
    tcp = struct.pack("!HHIIBBHHH", 40000, 80, 0x12345678, 0, 0x50, 2, 64240, 0, 0)
    pseudo = source + target + bytes([0, 6]) + struct.pack("!H", len(tcp))
    tcp = tcp[:16] + struct.pack("!H", checksum(pseudo + tcp)) + tcp[18:]
    ip = struct.pack("!BBHHHBBH4s4s", 0x45, 0, 40, 0x1234, 0x4000, ttl, 6, 0, source, target)
    ip = ip[:10] + struct.pack("!H", checksum(ip)) + ip[12:]
    return dst_mac + src_mac + bytes.fromhex("0800") + ip + tcp


def main():
    arp = struct.pack("!HHBBH", 1, 0x0800, 6, 4, 1) + client + source + bytes(6) + server
    answer = struct.pack("!HHBBH", 1, 0x0800, 6, 4, 2) + gateway + server + client + source
    selecting = legacy["message"](kind=3, extra=[(50, source), (54, server)])
    fixtures = {"arp-request": bytes([255] * 6) + client + bytes.fromhex("0806") + arp,
                "arp-reply": client + gateway + bytes.fromhex("0806") + answer,
                "routed-input": tcp_frame(64, client, gateway),
                "routed-output": tcp_frame(63, wan, upstream),
                "lab-discover": legacy["frame"](legacy["message"]()),
                "lab-selecting": legacy["frame"](selecting)}
    fixtures["lab-discover-client-id"] = legacy["frame"](legacy["message"](client_id=True))
    for name, kind, ciaddr, options in [
        ("lab-renew", 3, source, []), ("lab-release", 7, source, [(54, server)]),
        ("lab-decline", 4, bytes(4), [(50, source), (54, server)]),
        ("lab-reboot", 3, bytes(4), [(50, source)]),
        ("lab-select-other", 3, bytes(4), [(50, source), (54, bytes([10,50,10,2]))]),
    ]:
        fixtures[name] = legacy["frame"](legacy["message"](kind=kind, ciaddr=ciaddr, extra=options),
                                          source=ciaddr, destination=server if ciaddr != bytes(4) else bytes([255]*4))
    for name, data in fixtures.items():
        OUT.joinpath(name + ".hex").write_text("\n".join(data[i:i + 32].hex() for i in range(0, len(data), 32)) + "\n")


if __name__ == "__main__":
    main()
