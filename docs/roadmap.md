# IronGate roadmap

The workspace provides offline `ig config check`, an initial read-only `ig doctor`, and an offline borrowed DHCP request/OFFER protocol pilot. Broader host preflight, live packet handling and runtime gateway behavior remain unimplemented. Checklist completion requires implementation evidence and the acceptance criteria below.

The immediate product goal is a working captive portal demonstration, followed by RADIUS-backed login and accounting through the same session engine. Both flows, plus two distinct VLAN profiles, are required for the lab MVP. The [backward-planned MVP implementation plan](mvp-plan.md) defines scope, dependencies, acceptance gates, branches and effort assumptions. It takes priority over expanding isolated parser features or microbenchmarks.

## Current progress

- [x] Create a separate Rust workspace and daemon shell.
- [x] Separate allocation-free core and protocol library scaffolds.
- [x] Document ownership, memory policy, source behavior, and migration choices.
- [x] Set userspace portability, simple setup, behavior verification and NIC diagnostics as design requirements.
- [x] Specify readable TOML and a common CLI contract for people, scripts and AI agents.
- [x] Define `ig doctor` accounting/throughput diagnostics and bounded observation requirements.
- [x] Define performance/stability measurements, overload behavior and release gates.
- [x] Specify standalone consent, optional RADIUS providers, backend health and deployment sizing.
- [x] Require distinct per-VLAN portals, isolated network profiles and scoped diagnostics.
- [x] Checkpoint the initial workspace and design in Git.
- [x] Implement bounded TOML input, strict schema and cross-field configuration checks.
- [x] Implement text/JSON `ig config check` with stable codes, field paths and redacted failures.
- [x] Test all three profiles, malformed/contradictory configurations and real CLI contracts.
- [x] Document a minimal dependency set and disposable VM plan for future packet testing.
- [x] Implement passive Linux interface existence, link-state, MTU and driver checks with profile selection.
- [x] Preserve VLAN roles while inspecting shared trunks once; distinguish missing and unavailable evidence.
- [x] Test doctor through private fixtures and separate CLI modules, without new external dependencies.
- [x] Add config-free host inventory with active-link evidence, driver-reported speed/duplex, bounded enumeration and explicit unavailable values.
- [ ] Inventory deployed CoovaChilli options and integration contracts.
- [ ] Capture a reproducible behavioral and performance baseline.
- [ ] Set numerical packet-rate, latency, memory, revocation and accounting-lag targets for the first deployment profile.
- [ ] Validate the preferred userspace forwarding design in a bounded experiment.
- [ ] Deliver configuration validation, host preflight and read-only NIC inspection before live access enforcement.
- [x] Complete a bounded offline DHCP request/OFFER pilot with synthetic wire comparisons and measured replay allocation totals.
- [x] Compare pinned original C option/OFFER/checksum functions offline, with independent wire checks, sanitizers and separate timing/allocation evidence.
- [x] Establish reusable comparison suites, saved same-implementation baselines, explicit match status and regression/failure-report contracts.
- [ ] Qualify the pilot with differential decoding, coverage-guided fuzzing and isolated VM interoperability.

## MVP delivery order

| Increment | Result required before advancing |
| --- | --- |
| M0 Demo contract | Freeze lab profile, test topology, supported inputs and numerical acceptance goals |
| M1 Lease/session foundation | Bounded address ownership, DHCP lifecycle, generation-safe sessions and explicit effects |
| M2 Linux packet lab | Real client DHCP/ARP and bidirectional userspace transit with unauthorized-drop, revocation and crash guards |
| M3 Local consent portal | Client can accept terms, gain access, log out, and see matching records/diagnostics without RADIUS |
| M4 RADIUS portal/provider | HTTPS login, accept/reject/failure behavior, supported attributes, Start/Interim/Stop and separate auth/accounting health |
| M4b External portal contract | Explicit local/cloud placement, scoped handoff/callback, exact garden and separate multitenant HTTPS fixture |
| M5 MVP qualification | Two different per-VLAN portals, independent sessions, repeatable setup/teardown and load/failure evidence |

Keep all live work inside a disposable VM. The first demo can use an explicit portal URL and a routed controlled upstream; automatic popup interoperability and public Internet/NAT support follow. See [the detailed plan](mvp-plan.md) for exact inclusions and deferrals. The comparison framework supports each implemented path rather than becoming a prerequisite for broad Coova feature parity.

The [VM proposal](vm-demo.md) keeps all links, clients, RADIUS and an external/cloud portal stand-in inside one QEMU guest. The [hosting contract](portal-hosting.md) preserves local and external deployment choices, including different placement per VLAN, without treating tenant metadata as authority.

## Broader production qualification

These milestones retain the longer-term replacement scope. They are not all prerequisites for the first portal demonstration.

| Milestone | Deliverables | Evidence required to advance |
| --- | --- | --- |
| 0 Deployment profile and baseline | Hardware/OS matrix, client count, throughput and MTU goals, authentication methods, required options, issue-derived acceptance cases, sanitized captures, reproducible C build | Existing login, traffic, counters and disconnect behavior reproduced in an isolated topology |
| 0a Operator preflight | Versioned TOML network/provider schema, candidate edits, text/JSON CLI, change plans, capacity estimates, Linux preflight and NIC inventory | Standalone and RADIUS profiles validate; VLAN bindings and capacity conflicts are clear; noninteractive diagnostics work without mutations |
| 1 Protocol and memory pilot | Evaluate existing libraries; implement one Ethernet/IPv4/UDP/DHCP input path or equivalent service boundary; bounded leases and packet buffers | Truncation and malformed-input tests, replay comparisons, capacity exhaustion checks, allocator measurements and fuzz results |
| 2 Session and policy core | Session arena, generations, explicit transitions, policy effects, timers, staged traffic counters, quotas and bounded diagnostic snapshots | Stale-event rejection, slot reuse, deterministic expiry, correct quotas, coherent snapshots and zero measured allocation during session churn |
| 3 End-to-end gateway | Userspace forwarding, per-VLAN portals and isolation, standalone consent/local records, required RADIUS methods/attributes, backend health, accounting and bounded shaping | Two VLANs use independent portals/policy/counters; standalone works without RADIUS; centralized login/accounting/failover works; revocation and diagnostics identify the correct network/session |
| 4 Operations | Bounded reload, privilege handling, packaging, active client verification, NIC tuning plans and restart behavior | Target-hardware installation, service/backend failure, reload and rollback exercises; NIC changes are verified and reversible where supported |
| 5 Qualification | Replay corpus, fuzzing, interoperability, repeatable target-hardware benchmarks, overload/soak tests and controlled deployment | Agreed packet capacity, tail latency, memory stability, revocation, accounting-lag and diagnostics-overhead thresholds met with a demonstrated rollback |

## First implementation sequence

The first doctor/preflight slice is complete; [its scope](doctor.md) excludes routes, host guards, service conflicts, device capabilities and actual backend health. Future increments can add that evidence while keeping unavailable observations distinct from successful checks. Packet lab provisioning belongs in a disposable VM, not on the development PC. See [testing boundaries](testing.md). Complete the deployment inventory and target selection alongside these increments.

### Protocol pilot checkpoint

The operator slice and Rust 1.100 baseline are integrated into `release/0.1.x`. `feature/protocol-pilot` now implements the [offline Ethernet/IPv4/UDP/DHCP experiment](protocol-pilot.md): borrowed request views, caller-owned broadcast OFFER output, explicit work limits, synthetic wire fixtures and truncation/malformed/capacity tests. Initial Valgrind replay shows zero additional heap allocation across 100000 tagged request/OFFER operations. The [Coova comparison](coova-comparison.md) adds differential option/OFFER cases and independent checksum/structure checks; this C path also shows zero additional replay allocations. The [comparison framework](comparison-framework.md) preserves this evidence as versioned suites. These workloads do not establish leases, a complete DHCP server or live gateway performance. Finish this checkpoint, then work through the demo contract and MVP sequence above.

Clearer speed labels, Wi-Fi rate reporting and broader host preflight remain follow-ups; they do not block the protocol experiment. Record the first deployment's behavior and numerical targets alongside the pilot so its eventual benchmarks have useful acceptance criteria.

1. Freeze the MVP acceptance contract and disposable-lab ownership, including a representative deployment profile.
2. Implement bounded lease/session ownership and required DHCP lifecycle; extend comparison/fuzz cases for those actual paths.
3. Prove userspace forwarding, native local portal reachability, return traffic and failure guards in the isolated topology.
4. Deliver the local consent portal and record/diagnostic walkthrough.
5. Add HTTPS login and RADIUS authentication/accounting without changing session/policy ownership.
6. Prove the external portal handoff with explicit tenant/device scope using the isolated HTTPS fixture.
7. Qualify mixed local/external VLAN profiles and the load/failure cases before broadening the service surface.

Review `edge-dhcp`, `etherparse`, `dhcproto` and other proven components when they remove a concrete implementation risk; retain the dependency/memory review before adoption. Packet-library comparison and fuzzing belong alongside affected protocol work, rather than an indefinite research phase before the portal.

Reference projects: [edge-dhcp](https://docs.rs/edge-dhcp/latest/edge_dhcp/), [dhcproto](https://github.com/bluecatengineering/dhcproto), [Dora](https://github.com/bluecatengineering/dora), and [uspot](https://github.com/f00b4r0/uspot). These are components or architecture references, not established Rust replacements for CoovaChilli.

## Effort assumptions

The initial source review estimated 2–4 engineer-weeks for discovery, 4–8 engineer-weeks for a parser pilot, and 8–14 engineer-months in total for a focused Linux production replacement including those stages. A team of two experienced networking engineers should provisionally plan 5–9 calendar months. Broad feature and platform parity was estimated at 24–48+ engineer-months.

These are planning ranges, not commitments. Unknown deployment requirements, forwarding choice, dependency reuse, strict allocation limits and embedded targets can change them. Setup, diagnostics, tuning, standalone providers, local record durability and multiple VLAN profiles expand the original scope and have not been separately estimated; do not treat them as free within the earlier range. Re-estimate after milestones 0 and 1 using the preferred userspace design and operator acceptance criteria. Local credentials/vouchers and transparent gateway HA remain separate increments.

The narrower controlled-lab MVP is provisionally 11–21 engineer-weeks from the current checkpoint, including the external portal fixture/contract, with the first local portal gate roughly 6–11 weeks of that. This separate estimate does not replace production qualification or imply measured delivery capacity; revisit it after the demo contract and Linux packet experiment. See [the work breakdown and assumptions](mvp-plan.md).

## Qualification scenarios

Cover DHCP renewal/release and exhaustion; DNS/garden exceptions; unauthorized traffic; login accept/reject/challenge; retry/failover; invalid authenticators; delayed replies after session reuse; CoA/disconnect; accounting start/interim/stop; bandwidth and byte limits; fragmented and truncated packets; reload/restart; and network/backend failure.

Add cold setup from the documented profile; confusing interface names; bridge/VLAN and network-manager conflicts; competing DHCP/DNS; duplicate addresses; missing host guard; IPv6 bypass; daemon crash; link renegotiation; unsupported NIC settings; offload metadata; shaping saturation; and failure halfway through a tuning plan. Read-only checks must not mutate networking. A missing client-side probe must report end-to-end verification as unverified, not passed.

Include the [accounting diagnostic acceptance cases](accounting-diagnostics.md): low-rate sessions at high offered load, effective RADIUS/CoA rate overrides, counted-versus-transmitted discrepancies, overdue updates and retries, counter rollover, session reuse, incomplete history and long-duration churn. Verify the quality of the diagnostic finding as well as the underlying counter behavior.

Use [performance and stability requirements](performance-stability.md) for the workload matrix, benchmark evidence and failure contracts. A single event-loop owner is the initial design, not an unmeasured throughput guarantee. Validate its headroom early and re-estimate if the deployment requires parallel packet workers.

Include [standalone/provider health tests](authentication-radius.md), [fleet/backend sizing](deployment-scaling.md) and [VLAN isolation tests](vlan-networks.md). Verify per-network portal callback binding, same-MAC independent sessions, unknown VLAN denial, disabled-versus-unavailable RADIUS health, authentication/accounting split failures and survivor backend load.

Compare captures and effects with CoovaChilli while normalizing timestamps, identifiers and challenges. Preserve correct deployment behavior, and explicitly record intentional fixes to malformed-input handling. Measure packets/second, CPU, latency percentiles, drops, session setup rate, memory per session, pool occupancy and allocator calls. Numeric release thresholds depend on the baseline and target capacity.

Keep captured credentials and subscriber identifiers out of source control. The initial rollback path is a maintenance restart with reauthentication; persistent session import and live handoff require separate design and testing.
