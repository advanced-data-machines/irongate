"""Fail closed on incomplete, stale or contradictory live-scenario reports."""

CASES = frozenset({
    "real_busybox_dhcp", "userspace_arp_and_native_local_tcp", "unauthorized_transit_denied",
    "authorized_bidirectional_tcp_and_dns", "logout_blocks_existing_and_new_tcp",
    "expiry_blocks_existing_and_new_tcp", "crash_guard_with_kernel_routing_and_fixed_neighbors",
})


def validate(guest, run_id):
    if (guest.get("schema") != 1 or guest.get("scope") != "isolated_packet_lab"
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
