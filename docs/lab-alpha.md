# Lab alpha: a fixed finish line

The next deliverable is a **RADIUS-backed lab alpha**. It is deliberately earlier than the broader two-VLAN/cloud/load-qualified MVP. Stop adding alpha features when this checklist passes; package the evidence for evaluation rather than expanding the framework.

| Increment | Good enough / exit criterion | Status |
| --- | --- | --- |
| Authentication | One configured backend/method, HTTPS login, validated Accept/Reject/timeout, applied Session-Timeout, stale-result rejection and visible outcomes | Integrated into `release/0.1.x` at `84abe82`; see [evidence and limits](radius-login.md) |
| Accounting | Matching Start/Interim/Stop identity, directional 64-bit counters, bounded retries/capacity and observable backlog; logout immediate during failure | Implemented on `feature/radius-accounting`; clean-commit acceptance pending; see [bounds](radius-accounting.md) |
| Repeatable demonstration | One documented profile and command, readable configuration, actionable passive diagnostics, clean-commit correctness and basic performance evidence | VM commands exist; operational/configuration integration and final evidence packaging remain |

The alpha is finished when an independent client obtains an address, stays blocked before login, receives actual transit after a valid login, is denied on rejection/invalid response/timeout, loses access on logout/expiry, and produces matching acknowledged accounting records with failures visibly distinguished. Repeat it from a fresh disposable VM without manual repair. Local audit rows do not substitute for RADIUS accounting.

Keep security validation, bounded storage/work, correct revocation and truthful diagnostic scope as requirements. Defer multiple auth methods, failover, CoA, vouchers, shaping, certificate automation, cosmetic cleanup and broad distro support. The old CLI parser CRAP finding stays tracked without becoming a reason to redesign unrelated CLI code during the login slice.

The original full MVP still requires two distinct VLAN portals, scoped external/cloud handoff and the declared load/churn/soak/outage qualification. Alpha completion must not be presented as those gates passing, gateway capacity or production readiness. There is no reliable weighted burndown history for a delivery-date promise.

## Later dedicated appliance

The user plans a dedicated gateway appliance for a more representative environment. Keep workstation work inside QEMU until that separate environment is ready. Begin appliance tests on an isolated test LAN, preferably with separate subscriber/uplink NICs (or an explicitly tested VLAN trunk), without changing the production LAN's default gateway.

Record actual CPU/RAM, architecture, kernel, driver/firmware, negotiated speeds, MTU and offload settings. Replay the same deny/admit/logout/expiry/crash/backend-outage scenarios first, then measure NIC drops, directional counters, CPU/RSS, latency and sustained load against the frozen profile. Add protected backend transport, real certificate management, startup/restart and storage recovery qualification before considering deployment. VM results are a baseline to compare with that hardware, not a substitute for its measurements.
