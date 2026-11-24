"""Fail closed on incomplete, stale or contradictory live-scenario reports."""

CASES = frozenset({
    "real_busybox_dhcp", "userspace_arp_and_native_local_tcp", "unauthorized_transit_denied",
    "authorized_bidirectional_tcp_and_dns", "logout_blocks_existing_and_new_tcp",
    "expiry_blocks_existing_and_new_tcp", "crash_guard_with_kernel_routing_and_fixed_neighbors",
    "authorized_spoofed_mac_denied", "authorized_spoofed_ip_denied", "authorized_ipv6_userspace_denied",
    "ipv6_kernel_guard_after_crash",
})

PORTAL_CASES=frozenset({"browser_consent_and_logout","portal_one_use_origin_and_action_rejection",
    "portal_expired_and_stale_generation_actions","portal_expiry_blocks_existing_and_new_tcp",
    "portal_slow_client_does_not_block_control","portal_record_pressure_preserves_logout"})


TLS_CASES=frozenset({"tls_certificate_validation_and_no_http_fallback","tls_handshake_deadline_pressure_and_memory"})

def validate(guest, run_id,portal=False,https=False):
    if https:portal=True
    schema,suite=(3,"local-consent-v1")if portal else(2,"untagged-acceptance-v1")
    if https:schema,suite=4,"local-https-v1"
    expected=(CASES|PORTAL_CASES|TLS_CASES)if https else CASES|PORTAL_CASES if portal else CASES
    if (guest.get("schema") != schema or guest.get("suite") != suite or guest.get("scope") != "isolated_packet_lab"
            or guest.get("run_id") != run_id or guest.get("status") != "passed"):
        raise ValueError("Guest result identity/status does not match this run")
    cases = guest.get("cases", [])
    if (not isinstance(cases, list) or len(cases) != len(expected)
            or any(not isinstance(case, dict) or case.get("passed") is not True for case in cases)
            or {case.get("name") for case in cases} != expected):
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
    if portal:validate_portal(guest,run_id,namespaces)
    if https:validate_tls(guest)


def validate_portal(guest,run_id,namespaces):
    browser=guest.get("browser",{})
    if (browser.get("namespace")!=namespaces["iglab-client"] or browser.get("transport")!="devtools_pipe"
            or not browser.get("product")):
        raise ValueError("Browser did not run in the subscriber namespace")
    records=guest.get("local_records",{})
    rows=records.get("rows",[])
    if records.get("retained")!=16 or records.get("durable")!=16 or records.get("capacity")!=16:
        raise ValueError("Missing bounded/durable journal evidence")
    if ([r.get("id")for r in rows]!=list(range(1,17))
            or any(r.get("run_id")!=run_id or r.get("network")!=1 for r in rows)
            or sum(r.get("kind")=="Granted"for r in rows)!=4 or rows[-1].get("kind")!="Revoked"):
        raise ValueError("Portal journal prefix/scope does not match admissions")
    statuses={"replay":409,"origin":403,"wrong_action":409,"network_field":400,"expired":409,"stale_generation":409,"capacity":503}
    if guest.get("portal_http")!=statuses:raise ValueError("Portal rejection evidence is incomplete")


def validate_tls(guest):
    browser=guest.get("browser",{})
    if browser.get("origin")!="https://10.50.10.1:8443" or browser.get("certificate_verification")!="normal":
        raise ValueError("Verified HTTPS browser evidence missing")
    certificates=guest.get("tls_certificates",{})
    expected={"expired":"ERR_CERT_DATE_INVALID","wrong_name":"ERR_CERT_COMMON_NAME_INVALID","untrusted":"ERR_CERT_AUTHORITY_INVALID"}
    if any(code not in certificates.get("browser",{}).get(name,"") for name,code in expected.items()):
        raise ValueError("Browser certificate rejection evidence missing")
    if any(certificates.get("python",{}).get(name,{}).get("rejected") is not True for name in expected):
        raise ValueError("Independent certificate validation missing")
    pressure=guest.get("tls_pressure",{});transport=pressure.get("transport",{})
    if (pressure.get("deadline",{}).get("closed")!=12 or not 1.5<pressure.get("deadline",{}).get("elapsed_seconds",0)<3.5
            or pressure.get("oversized",{}).get("rejected")!=16 or transport.get("scheme")!="https"
            or transport.get("queue_rejections",0)<1 or transport.get("tls_timeouts",0)<2 or transport.get("tls_failures",0)<16):
        raise ValueError("Bounded TLS pressure evidence missing")
    for key in ("memory_before","memory_after"):
        if pressure.get(key,{}).get("VmRSS",0)<=0 or pressure.get(key,{}).get("VmHWM",0)<=0:
            raise ValueError("TLS memory observations missing")
