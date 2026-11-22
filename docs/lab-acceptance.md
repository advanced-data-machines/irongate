# Versioned lab acceptance contract

The engineering target is now **`mvp-lab-v1`**, defined in [`acceptance.toml`](../tools/vm/lab/acceptance.toml). This freezes the candidate deployment/test profile for implementation and qualification. The numbers are acceptance targets, not measured performance, production sizing or deployable configuration defaults. Change them through a reviewed profile/version change with a stated reason; do not lower a target silently after a failing run.

The current runnable functional suite is **`untagged-acceptance-v1`**: one client/network, fixed peers, five-second test admission, four vCPUs, 8192 MiB RAM, 32 GiB virtual disk and no external NIC. It cannot qualify a 64-session/two-network MVP. Each live report retains the target's parsed contents and SHA-256, and explicitly records scale qualification as `not_run`.

## Frozen qualification targets

| Area | Target and boundary |
| --- | --- |
| Topology | 64 session slots across two independent VLAN/network profiles; eight active traffic generators |
| Main load | 100 Mbit/s aggregate **IP** traffic, approximately 1200-byte UDP payloads; ten-second warmup, three 60-second trials |
| Small frames | Separate run at 10 Mbit/s of 64-byte IP packets; retain offered/received rates and loss |
| Loss | Zero unexpected datagram loss at the declared target; policy denials are separate observations |
| Added latency | p99 no greater than 5 ms, measured against the controlled topology baseline with matching packets/load |
| Grant/revocation | No greater than 250 ms from valid local request receipt to committed forwarding-state change; independently verify allowed/blocked traffic |
| Stability | 30-minute bounded-memory run with ten session admission/cleanup cycles per second and continued control progress |
| Hot-path memory | Zero additional packet/session heap allocations after startup/warmup; startup packet/session reservation no greater than 16 MiB |
| Process memory | Release daemon RSS no greater than 128 MiB; post-warmup growth no greater than 4 MiB during the soak; measure separately from guest RAM/fixtures |
| Low-rate client | 64 kbit/s IP traffic alongside the bulk flows, with diagnosis separating offered load, policy and capacity loss |
| Healthy accounting | 60-second interim schedule; send lag no greater than one second and validated acknowledgement no later than five seconds after the obligation is due |

All three load trials must pass; retain raw observations and variability rather than averaging a failure away. Record minimum-size/mixed-size traffic separately from the main load. The one-client functional suite has not measured these targets. Physical NIC throughput, offloads, hardware MTU/capacity and Wi-Fi/client discovery require separate qualification.

RADIUS counter input is frozen for this lab target as IP octets successfully submitted to the selected packet descriptor: NAS input is subscriber-to-upstream, NAS output is upstream-to-subscriber. Denied and failed submissions are excluded and separately counted. This is a declared submission-stage boundary, not proof of wire delivery or durable billing accuracy. The future accounting implementation must compare it with independent sender/receiver evidence, encode 64-bit counters interoperably and expose lag/outages. RADIUS and accounting are not implemented yet.

## Functional evidence required now

The eleven named cases in [`evidence.py`](../tools/vm/lab/evidence.py) cover DHCP, userspace ARP/native TCP, unauthorized denial, admitted TCP/DNS, logout, expiry, IPv4 crash guarding, authorized MAC/IP spoof rejection, userspace IPv6 rejection, and IPv6 kernel guarding after crash. A passing result also requires namespace separation, loopback-only management, exact run/suite identity and successful cleanup.

Spoof checks run while the session remains authorized. Independent packet observers must see each injected frame at gateway ingress and none at upstream ingress. A valid raw IPv4 positive control must traverse the same path. Thus malformed input, missing capture setup or expired admission cannot stand in for spoof protection. Host tests independently check the injected packets' checksum/address forms.

The IPv6 userspace test sends a valid raw IPv6 UDP frame while ordinary IPv6 is disabled. The separate kernel test then deliberately enables IPv6 and routing in the owned namespaces after IronGate has died, assigns addresses and permanent neighbors, sends a real IPv6 datagram, and requires a dedicated nftables IPv6-drop counter increase. It proves the guard independently of disabled IPv6, missing routes, NDP or a missing upstream listener. It does not implement supported IPv6 transit.

Live report schema **2** identifies this stronger suite. Schema-1 seven-case packet-lab reports remain historical evidence and cannot pass the expanded validator. There is no silent pass for missing cases, expired authorization, unseen ingress, an absent positive control or an unexercised guard. Failed/inconclusive observations are retained in that run's JSON/logs.

## Startup quality work and remaining gates

Descriptor setup now lives in `lab/transport.rs`; one deterministic owner turn lives in `lab/owner.rs`. The live loop and unprivileged socket tests share that turn, including actual polling, clock advancement, control priority and packet submission. Tests verify that queued logout takes priority over queued transit, regular files are rejected as packet descriptors, and clock failures propagate.

The first coverage-backed run after this change reports **216 functions** with only the existing CLI parser above CRAP 30 (**39.32**). The former combined startup function scored 207.02; the smaller wiring entry now scores **5.67**. More substantively, the shared turn and transport validation have **100% ordinary-test line coverage**. Guest-only inherited descriptor/bootstrap and the infinite serving loop still lack ordinary-test coverage; QEMU executes them, but guest counters are not merged into LCOV. Those remaining gaps must stay visible rather than being treated as 100% startup coverage.

The first expanded live run (`3358db0b75472bad1fac21cf6de91b28`) passed all eleven cases with one observed ingress and zero observed upstream packets for each spoof/IPv6 probe, and a one-packet dedicated IPv6 guard increment. It was a development working-tree run. Final acceptance evidence must identify the exact committed source; generated artifacts remain under ignored `target/` and are not the permanent release archive.

The lab target contract and first functional M2 proof are now defined/tested. M2 load/capacity qualification, fuzzing/broader interoperability and production startup recovery remain open. The next product feature is M3 local consent: browser/API handoff, one-use generation-bound actions, local records and matching status evidence. M4 RADIUS/accounting and M5 two-network load/soak evidence follow their actual implementations. See the [burndown](burndown.md) and [MVP plan](mvp-plan.md).
