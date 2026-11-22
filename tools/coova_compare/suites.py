"""Explicit suite registry: add focused adapters instead of extending the runner."""

from corpus import fixture, offer_cases, option_cases
from framework import Case, Suite, Workload, require
from wire import compare_offers


def options():
    return [Case(case.name, "option", case.payload, case.code,
                 {"c_status": case.c_status, "rust_status": case.rust_status,
                  "classification": case.classification}) for case in option_cases()]


def compare_option(case, c, checked, rust):
    expected = case.expected
    require(c["status"] == checked["status"] == expected["c_status"], case.name + ": C status")
    require(c["value"] == checked["value"], case.name + ": sanitized C differs")
    require(rust["status"] == expected["rust_status"], case.name + ": Rust status")
    if expected["classification"] == "equivalent":
        require(c["value"] == rust["value"], case.name + ": lookup value differs")
    return {"classification": expected["classification"], "c": c, "rust": rust}


def offers():
    return [Case(name, "offer", data, expected={"broadcast_clear": name == "broadcast-flag-clear",
                 "rust_fixture": None if name == "broadcast-flag-clear" else
                 "offer-vlan-client-id" if name == "tagged-client-id" else "offer"})
            for name, data in offer_cases()]


def compare_offer(case, c, checked, rust):
    require(c["status"] == checked["status"] == rust["status"] == "encoded", case.name + ": encoding failed")
    require(c["frame"] == checked["frame"], case.name + ": sanitized C differs")
    result = compare_offers(bytes.fromhex(c["frame"]), bytes.fromhex(rust["frame"]),
                            broadcast_clear=case.expected["broadcast_clear"])
    expected = case.expected["rust_fixture"]
    if expected:
        require(bytes.fromhex(rust["frame"]) == fixture(expected), case.name + ": expected wire fixture differs")
    return result


def lookup_workloads():
    return (Workload("option-61-lookup", "option", fixture("discover-vlan-client-id")[46:], 61,
                     caveat="Valid input; Rust full validation is outside timing. Probe/compiler overhead is not matched."),)


def offer_workloads():
    return (Workload("tagged-offer", "offer", fixture("discover-vlan-client-id"),
                     caveat="Different options/padding; C includes capture copy. No C/Rust speedup ratio."),)


SUITES = (
    Suite("dhcp-options", 1, "Option lookup versus validated borrowed DHCP options; not daemon acceptance.",
          options, compare_option, lookup_workloads),
    Suite("dhcp-offers", 1, "Broadcast/unicast OFFER wire semantics with independent checksums and structure.",
          offers, compare_offer, offer_workloads, offer_workloads),
)


def select(names=None):
    requested = set(names or [suite.name for suite in SUITES])
    unknown = requested - {suite.name for suite in SUITES}
    if unknown:
        raise ValueError("Unknown suite: " + ", ".join(sorted(unknown)))
    return [suite for suite in SUITES if suite.name in requested]
