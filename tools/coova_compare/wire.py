"""Independent strict decoding of emitted frames; no C/Rust parser reuse."""

import struct


def checksum(data):
    total = sum(int.from_bytes(data[i:i + 2].ljust(2, b"\0"), "big")
                for i in range(0, len(data), 2))
    while total >> 16:
        total = (total & 65535) + (total >> 16)
    return (~total) & 65535


def decode(data):
    if len(data) < 14:
        raise ValueError("Short Ethernet header")
    vlan = None
    link = 14
    if data[12:14] == bytes.fromhex("8100"):
        vlan = int.from_bytes(data[14:16], "big")
        link = 18
    if data[link - 2:link] != bytes.fromhex("0800") or data[link] != 0x45:
        raise ValueError("Expected Ethernet/IPv4 without IP options")
    ip = data[link:link + 20]
    total = int.from_bytes(ip[2:4], "big")
    if total < 28 or link + total != len(data) or ip[9] != 17:
        raise ValueError("Incorrect IP length or protocol")
    if checksum(ip):
        raise ValueError("Invalid IPv4 checksum")
    udp = data[link + 20:]
    sport, dport, length, check = struct.unpack("!HHHH", udp[:8])
    if length != len(udp):
        raise ValueError("Incorrect UDP length")
    pseudo = ip[12:20] + bytes([0, 17]) + length.to_bytes(2, "big")
    if not check or checksum(pseudo + udp):
        raise ValueError("Missing or invalid emitted UDP checksum")
    message = udp[8:]
    if len(message) < 240 or message[236:240] != bytes.fromhex("63825363"):
        raise ValueError("Invalid DHCP header")
    options = []
    cursor = 240
    while cursor < len(message):
        code = message[cursor]
        cursor += 1
        if code == 0:
            continue
        if code == 255:
            if any(message[cursor:]):
                raise ValueError("Non-PAD after END")
            break
        if cursor == len(message):
            raise ValueError("Missing option length")
        size = message[cursor]
        cursor += 1
        value = message[cursor:cursor + size]
        if len(value) != size:
            raise ValueError("Truncated option")
        options.append([code, value.hex()])
        cursor += size
    else:
        raise ValueError("Missing END")
    return {"frame_bytes": len(data), "ethernet_source": data[6:12].hex(),
            "ethernet_destination": data[:6].hex(), "vlan_tci": vlan,
            "ip_source": ip[12:16].hex(), "ip_destination": ip[16:20].hex(),
            "ttl": ip[8], "ip_bytes": total, "udp_bytes": length,
            "source_port": sport, "destination_port": dport,
            "bootp_header": message[:236].hex(), "dhcp_bytes": len(message),
            "padding_bytes": len(message) - cursor, "options": options,
            "ipv4_checksum_valid": True, "udp_checksum_valid": True}


def compare_offers(c_frame, rust_frame, broadcast_clear=False):
    c, rust = decode(c_frame), decode(rust_frame)
    differences = []
    permitted = {"frame_bytes", "ip_bytes", "udp_bytes", "dhcp_bytes", "padding_bytes", "ttl", "options"}
    if broadcast_clear:
        permitted.update(("ethernet_destination", "ip_destination"))
    for field in sorted(c.keys() | rust.keys()):
        if c.get(field) == rust.get(field):
            continue
        if field not in permitted:
            raise AssertionError(f"Unexpected shared-field mismatch: {field}")
        classification = "wire_layout_difference"
        if field == "ttl": classification = "policy_difference"
        if field in ("ethernet_destination", "ip_destination"):
            classification = "pilot_scope_difference"
        if field != "options":
            differences.append({"field": field, "c": c[field], "rust": rust[field],
                                "classification": classification})
    c_options, rust_options = dict(c["options"]), dict(rust["options"])
    if len(c_options) != len(c["options"]) or len(rust_options) != len(rust["options"]):
        raise AssertionError("Duplicate emitted options")
    for code in sorted(c_options.keys() | rust_options.keys()):
        if c_options.get(code) == rust_options.get(code): continue
        if code == 26 and rust_options.get(code) is None:
            classification = "missing_rust_functionality"
        elif code == 61 and c_options.get(code) is None:
            classification = "suspected_c_compatibility_defect"
        else:
            raise AssertionError(f"Unexpected DHCP option mismatch: {code}")
        differences.append({"option": code, "c": c_options.get(code), "rust": rust_options.get(code),
                            "classification": classification})
    if [code for code, _ in c["options"]] != [code for code, _ in rust["options"]]:
        differences.append({"field": "option_order", "classification": "wire_layout_difference",
                            "c": [code for code, _ in c["options"]],
                            "rust": [code for code, _ in rust["options"]]})
    return {"c": c, "rust": rust, "differences": differences,
            "exact_bytes_equal": c_frame == rust_frame, "shared_fields_equal": True}
