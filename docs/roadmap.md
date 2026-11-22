# IronGate roadmap

The workspace provides offline `ig config check` and an initial read-only `ig doctor` with scoped interface evidence, text/JSON reports and regression tests. Broader host preflight, packet handling and runtime gateway behavior remain unimplemented. Checklist completion requires implementation evidence and the acceptance criteria below.

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
- [ ] Complete the first allocation-measured protocol implementation.

## Milestones

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

### Recommended next checkpoint

Keep the completed operator slice on `feature/doctor` for review. After integration into `release/0.1.x`, the recommended next branch is `feature/protocol-pilot`: a small offline Ethernet/IPv4/UDP/DHCP parsing and reply-encoding experiment in `irongate-proto`. Start with a documented packet corpus and explicit supported/rejected forms, borrowed packet views, caller-owned output buffers, and malformed-input/capacity tests. Demonstrate allocation-free parsing and encoding before adding a lease service or live sockets. This tests the project's performance and memory assumptions early while keeping all development safe on this PC. Live interoperability follows in the disposable VM.

Clearer speed labels, Wi-Fi rate reporting and broader host preflight remain follow-ups; they do not block the protocol experiment. Record the first deployment's behavior and numerical targets alongside the pilot so its eventual benchmarks have useful acceptance criteria.

1. Select one representative deployment and record its required behavior in `coova-compatibility.md`.
2. Establish an isolated subscriber/gateway/upstream topology with a portal and RADIUS test server.
3. Evaluate `edge-dhcp` for bounded DHCP behavior; compare `dhcproto` and Dora as protocol and interoperability references. Do not add a dependency until allocation behavior, features, target support and license fit have been reviewed.
4. Implement borrowed validation of one complete input path, bounded reply encoding and explicit capacity errors. Test actual malformed input and full-capacity behavior.
5. Add allocator instrumentation and measure the pilot before growing the service surface.
6. Validate userspace forwarding for revocation, counters, bounded shaping and portal compatibility. Measure the platform boundary and record any justified kernel backend dependency in `architecture.md`.

Reference projects: [edge-dhcp](https://docs.rs/edge-dhcp/latest/edge_dhcp/), [dhcproto](https://github.com/bluecatengineering/dhcproto), [Dora](https://github.com/bluecatengineering/dora), and [uspot](https://github.com/f00b4r0/uspot). These are components or architecture references, not established Rust replacements for CoovaChilli.

## Effort assumptions

The initial source review estimated 2–4 engineer-weeks for discovery, 4–8 engineer-weeks for a parser pilot, and 8–14 engineer-months in total for a focused Linux production replacement including those stages. A team of two experienced networking engineers should provisionally plan 5–9 calendar months. Broad feature and platform parity was estimated at 24–48+ engineer-months.

These are planning ranges, not commitments. Unknown deployment requirements, forwarding choice, dependency reuse, strict allocation limits and embedded targets can change them. Setup, diagnostics, tuning, standalone providers, local record durability and multiple VLAN profiles expand the original scope and have not been separately estimated; do not treat them as free within the earlier range. Re-estimate after milestones 0 and 1 using the preferred userspace design and operator acceptance criteria. Local credentials/vouchers and transparent gateway HA remain separate increments.

## Qualification scenarios

Cover DHCP renewal/release and exhaustion; DNS/garden exceptions; unauthorized traffic; login accept/reject/challenge; retry/failover; invalid authenticators; delayed replies after session reuse; CoA/disconnect; accounting start/interim/stop; bandwidth and byte limits; fragmented and truncated packets; reload/restart; and network/backend failure.

Add cold setup from the documented profile; confusing interface names; bridge/VLAN and network-manager conflicts; competing DHCP/DNS; duplicate addresses; missing host guard; IPv6 bypass; daemon crash; link renegotiation; unsupported NIC settings; offload metadata; shaping saturation; and failure halfway through a tuning plan. Read-only checks must not mutate networking. A missing client-side probe must report end-to-end verification as unverified, not passed.

Include the [accounting diagnostic acceptance cases](accounting-diagnostics.md): low-rate sessions at high offered load, effective RADIUS/CoA rate overrides, counted-versus-transmitted discrepancies, overdue updates and retries, counter rollover, session reuse, incomplete history and long-duration churn. Verify the quality of the diagnostic finding as well as the underlying counter behavior.

Use [performance and stability requirements](performance-stability.md) for the workload matrix, benchmark evidence and failure contracts. A single event-loop owner is the initial design, not an unmeasured throughput guarantee. Validate its headroom early and re-estimate if the deployment requires parallel packet workers.

Include [standalone/provider health tests](authentication-radius.md), [fleet/backend sizing](deployment-scaling.md) and [VLAN isolation tests](vlan-networks.md). Verify per-network portal callback binding, same-MAC independent sessions, unknown VLAN denial, disabled-versus-unavailable RADIUS health, authentication/accounting split failures and survivor backend load.

Compare captures and effects with CoovaChilli while normalizing timestamps, identifiers and challenges. Preserve correct deployment behavior, and explicitly record intentional fixes to malformed-input handling. Measure packets/second, CPU, latency percentiles, drops, session setup rate, memory per session, pool occupancy and allocator calls. Numeric release thresholds depend on the baseline and target capacity.

Keep captured credentials and subscriber identifiers out of source control. The initial rollback path is a maintenance restart with reauthentication; persistent session import and live handoff require separate design and testing.
