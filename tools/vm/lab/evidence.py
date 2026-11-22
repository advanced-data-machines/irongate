"""Fail closed on incomplete, stale or contradictory live-scenario reports."""

CASES = frozenset({
    "real_busybox_dhcp", "userspace_arp_and_native_local_tcp", "unauthorized_transit_denied",
    "authorized_bidirectional_tcp_and_dns", "logout_blocks_existing_and_new_tcp",
    "expiry_blocks_existing_and_new_tcp", "crash_guard_with_kernel_routing_and_fixed_neighbors",
    "authorized_spoofed_mac_denied", "authorized_spoofed_ip_denied", "authorized_ipv6_userspace_denied",
    "ipv6_kernel_guard_after_crash",
})


def validate(guest, run_id):
    if (guest.get("schema") != 2 or guest.get("suite") != "untagged-acceptance-v1" or guest.get("scope") != "isolated_packet_lab"
            or guest.get("run_id") != run_id or guest.get("status") != "passed"):
        raise ValueError("Guest result identity/status does not match this run")
    cases = guest.get("cases", [])
    if (not isinstance(cases, list) or len(cases) != len(CASES)
            or any(not isinstance(case, dict) or case.get("passed") is not True for case in cases)
            or {case.get("name") for case in cases} != CASES):
        raise ValueError("Missing, duplicate, unknown or failed scenario")
    if guest.get("cleanup_errors") != [] or guest.get("management_interfaces") != ["lo"]:
        raise ValueError("Cleanup failed or management networking is not NIC-free")
    namespaces = guest.get("topology", {}).get("namespaces", {})
    management = guest.get("management_namespace")
    if (set(namespaces) != {"iglab-client", "iglab-gateway", "iglab-upstream"}
            or len(set(namespaces.values())) != 3 or not management or management in namespaces.values()):
        raise ValueError("Namespace isolation evidence is missing/contradictory")
    guard = guest.get("crash_guard", {})
    if guard.get("kernel_ip_forward") != 1 or guard.get("forward_drop_packets", 0) < 1:
        raise ValueError("Persistent guard was not independently exercised")
    probes = guest.get("security_probes", {})
    if guest.get("security_authorized") != {"before": True, "after": True}:
        raise ValueError("Spoof probes did not retain live authorized state")
    baseline = probes.get("baseline", {})
    if baseline.get("ingress_received", 0) < 1 or baseline.get("upstream_received", 0) < 1:
        raise ValueError("Raw-frame positive control is missing")
    for name in ("authorized_spoofed_mac_denied", "authorized_spoofed_ip_denied", "authorized_ipv6_userspace_denied"):
        probe = probes.get(name, {})
        if probe.get("ingress_received", 0) < 1 or probe.get("upstream_received") != 0:
            raise ValueError("Bypass was not observed at ingress and blocked at egress")
    ipv6 = guest.get("ipv6_guard", {})
    if (ipv6.get("kernel_forwarding") != 1 or ipv6.get("forward_drop_delta", 0) < 1
            or ipv6.get("ingress_received", 0) < 1 or ipv6.get("upstream_received") != 0):
        raise ValueError("IPv6 kernel bypass guard was not independently exercised")
