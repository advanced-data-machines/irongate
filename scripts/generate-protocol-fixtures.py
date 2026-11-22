"""Regenerate synthetic wire fixtures independently of the Rust implementation.

Uses only Python's standard library. Documentation addresses and locally
administered synthetic MACs; no captures, credentials or subscriber data.
"""

from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parents[1] / "crates/irongate-proto/tests/fixtures"


def checksum(data):
    data += b"\0" * (len(data) % 2)
    total = sum(struct.unpack("!" + "H" * (len(data) // 2), data))
    while total >> 16:
        total = (total & 65535) + (total >> 16)
    return (~total) & 65535


def frame(payload, reply=False, vlan=False, checked=True, source=None, destination=None,
          destination_mac=None):
    source = source if source is not None else bytes([192, 0, 2, 1]) if reply else bytes(4)
    destination = destination if destination is not None else bytes([255] * 4)
    ports = (67, 68) if reply else (68, 67)
    udp = struct.pack("!HHHH", *ports, len(payload) + 8, 0) + payload
    if checked:
        pseudo = source + destination + bytes([0, 17]) + struct.pack("!H", len(udp))
        value = checksum(pseudo + udp) or 65535
        udp = udp[:6] + struct.pack("!H", value) + udp[8:]
    ip = struct.pack("!BBHHHBBH4s4s", 0x45, 0, len(udp) + 20, 0, 0, 64, 17, 0,
                     source, destination)
    ip = ip[:10] + struct.pack("!H", checksum(ip)) + ip[12:]
    mac = bytes.fromhex("020000000001" if reply else "020000000002")
    eth = (destination_mac if destination_mac is not None else bytes([255] * 6)) + mac
    eth += bytes.fromhex("8100a0640800" if vlan else "0800")
    return eth + ip + udp


def message(reply=False, kind=None, client_id=False, ciaddr=bytes(4), broadcast=True,
            extra=()):
    data = bytearray(240)
    data[:3] = bytes([2 if reply else 1, 1, 6])
    data[4:8] = bytes.fromhex("12345678")
    data[10:12] = bytes.fromhex("8000" if broadcast else "0000")
    data[12:16] = ciaddr
    data[28:34] = bytes.fromhex("020000000002")
    data[236:240] = bytes.fromhex("63825363")
    kind = kind if kind is not None else 2 if reply else 1
    data.extend([53, 1, kind])
    if reply and kind != 6:
        data[16:20] = bytes([192, 0, 2, 100])
        options = [(54, bytes([192, 0, 2, 1])), (51, struct.pack("!I", 3600)),
                   (1, bytes([255, 255, 255, 0])), (3, bytes([192, 0, 2, 1])),
                   (6, bytes([192, 0, 2, 53]))]
        for code, value in options:
            data.extend(bytes([code, len(value)]) + value)
    if reply and kind == 6:
        data.extend(bytes([54, 4, 192, 0, 2, 1]))
    for code, value in extra:
        data.extend(bytes([code, len(value)]) + value)
    if client_id:
        data.extend(bytes.fromhex("3d0701020000000002"))
    data.append(255)
    data.extend(bytes(max(0, 300 - len(data))))
    return bytes(data)


def main():
    fixtures = {
        "discover": frame(message()),
        "discover-zero-checksum": frame(message(), checked=False),
        "discover-vlan-client-id": frame(message(client_id=True), vlan=True),
        "request": frame(message(kind=3)),
        "offer": frame(message(reply=True), reply=True),
        "offer-vlan-client-id": frame(message(reply=True, client_id=True), reply=True, vlan=True),
    }
    address = bytes([192, 0, 2, 100])
    server = bytes([192, 0, 2, 1])
    client_mac = bytes.fromhex("020000000002")
    for vlan, suffix in [(False, ""), (True, "-vlan-client-id")]:
        common = dict(client_id=vlan)
        selecting = dict(kind=3, extra=[(50, address), (54, server)], **common)
        fixtures["selecting" + suffix] = frame(message(**selecting), vlan=vlan)
        fixtures["ack" + suffix] = frame(message(reply=True, kind=5, **common), reply=True, vlan=vlan)
        fixtures["nak" + suffix] = frame(message(reply=True, kind=6, **common), reply=True, vlan=vlan)
        fixtures["renew" + suffix] = frame(
            message(kind=3, ciaddr=address, broadcast=False, **common), vlan=vlan,
            source=address, destination=server, destination_mac=bytes.fromhex("020000000001"))
        fixtures["ack-renew" + suffix] = frame(
            message(reply=True, kind=5, ciaddr=address, broadcast=False, **common),
            reply=True, vlan=vlan, destination=address, destination_mac=client_mac)
        fixtures["nak-renew" + suffix] = frame(
            message(reply=True, kind=6, broadcast=False, **common), reply=True, vlan=vlan)
    fixtures["selecting-unicast"] = frame(
        message(kind=3, extra=[(50, address), (54, server)], client_id=True, broadcast=False), vlan=True)
    fixtures["ack-unicast"] = frame(message(reply=True, kind=5, client_id=True, broadcast=False),
                                    reply=True, vlan=True, destination=address, destination_mac=client_mac)
    fixtures["discover-requested"] = frame(message(extra=[(50, address)]))
    fixtures["reboot"] = frame(message(kind=3, extra=[(50, address)]))
    fixtures["release"] = frame(message(kind=7, ciaddr=address, broadcast=False, extra=[(54, server)]),
                                source=address, destination=server,
                                destination_mac=bytes.fromhex("020000000001"))
    fixtures["decline"] = frame(message(kind=4, extra=[(50, address), (54, server)]))
    for name, data in fixtures.items():
        lines = [data[index:index + 32].hex() for index in range(0, len(data), 32)]
        ROOT.joinpath(name + ".hex").write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
