"""Shared synthetic corpus. Cases declare expected scope differences explicitly."""

from dataclasses import dataclass
from source import ROOT


@dataclass
class Case:
    name: str
    payload: bytes
    code: int
    rust_status: str
    c_status: str
    classification: str


def fixture(name):
    return bytes.fromhex((ROOT / f"crates/irongate-proto/tests/fixtures/{name}.hex").read_text())


def option_cases():
    header = fixture("discover")[42:282]
    def case(name, options, code=53, rust="found", c="found", classification="equivalent"):
        return Case(name, header + bytes(options), code, rust, c, classification)
    cases = [
        case("discover", [53, 1, 1, 255]),
        case("request", [53, 1, 3, 255]),
        case("client-id", [53, 1, 1, 61, 7, 1, 2, 0, 0, 0, 0, 2, 255], 61),
        case("unknown-option", [53, 1, 1, 200, 2, 10, 20, 255], 200),
        case("missing-option", [53, 1, 1, 255], 61, "missing", "missing"),
        case("pad-before-type", [0, 53, 1, 1, 255], c="missing", classification="suspected_c_lookup_defect"),
        case("missing-end", [53, 1, 1], rust="rejected", classification="validation_boundary_difference"),
        case("duplicate-type", [53, 1, 1, 53, 1, 3, 255], rust="rejected", classification="intentional_rust_restriction"),
        case("truncated-later-option", [53, 1, 1, 61, 7, 1], rust="rejected", classification="validation_boundary_difference"),
        case("truncated-target", [53, 1, 1, 61, 7, 1], 61, "rejected", "missing", "validation_boundary_difference"),
        case("overload", [53, 1, 1, 52, 1, 1, 255], rust="rejected", classification="intentional_rust_restriction"),
        case("unknown-message-type", [53, 1, 9, 255], rust="rejected", classification="intentional_rust_restriction"),
        case("garbage-after-end", [53, 1, 1, 255, 0, 61, 2, 1, 2, 255], 61,
             "rejected", "found", "suspected_c_lookup_defect"),
        case("empty-type", [53, 0, 255], rust="rejected", classification="validation_boundary_difference"),
    ]
    # Length boundary probes use real payload sizes, not zero-padded backing input.
    for length in (240, 241, 242, 243):
        cases.append(Case(f"prefix-{length}", fixture("discover")[42:42 + length], 53,
                          "rejected", "missing" if length < 243 else "found",
                          "validation_boundary_difference"))
    return cases


def offer_cases():
    cases = [("discover", fixture("discover")),
             ("tagged-client-id", fixture("discover-vlan-client-id")),
             ("zero-udp-checksum", fixture("discover-zero-checksum"))]
    # A valid client that clears B: Coova unicasts, while the pilot broadcasts.
    data = bytearray(fixture("discover-zero-checksum"))
    data[52:54] = b"\0\0"
    cases.append(("broadcast-flag-clear", bytes(data)))
    return cases
