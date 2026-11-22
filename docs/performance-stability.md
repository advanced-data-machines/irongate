# IronGate performance and stability requirements

Performance and stability are release criteria, alongside correct access enforcement and diagnosable accounting. A Rust implementation or userspace design alone establishes neither. This document defines the measurement and failure contracts to implement; the scaffold has no measured gateway capacity yet.

## Design priorities

Keep packet decisions, session transitions and accounting correct at the declared capacity. Use bounded memory, predictable work and explicit overload behavior. Optimize throughput and latency within that contract. Diagnostics are part of the product, and their interference with forwarding must be measured and bounded.

Each supported deployment profile declares hardware and driver identity, OS version, subscriber count, pending authentication capacity, packet/traffic mix, offered packets and bytes per second, MTU, latency objective, accounting interval/lag budget, memory budget, and allowed drops. Keep intentional policy/rate-limit drops distinct from capacity loss. Define the overload boundary so operators know what the system promises.

Declare VLAN/network profile count, the distribution of subscribers and traffic between profiles, shared versus dedicated RADIUS pools, and whether the deployment is standalone or centralized. Benchmark cross-profile contention and surviving backend capacity during failover rather than extrapolating from one VLAN or healthy-server latency.

## Performance measurements

| Dimension | Required measurement |
| --- | --- |
| Forwarding capacity | Packets/second and bytes/second across minimum-size, mixed-size and near-MTU traffic in both directions |
| Packet latency | Median, p95, p99 and worst observed delay under stated load; separate intentional shaping delay |
| CPU efficiency | CPU time or cycles per packet and per accounting event, with CPU utilization and queue ownership |
| Client scale | Idle and active session counts, session setup/churn rate and per-session memory |
| Control responsiveness | Login, logout, revocation and query latency while the dataplane is saturated |
| Accounting health | Scheduled-versus-actual update time, oldest pending event, response delay, retries and recovery backlog |
| Memory | Startup reservation, live allocations, resident memory, pool occupancy and high-water marks |
| Observation overhead | Throughput, latency, CPU and accounting lag with diagnostics idle, normal and heavily queried |

State units and measurement boundaries. Link throughput, IP-byte throughput, accounting octets and application goodput are different measurements. Record whether wire-size calculations include Ethernet framing overhead. Record offered load and losses so an apparently faster implementation cannot hide dropped work.

Compare repeatable runs on the same hardware, topology and configuration with fixed traffic seeds or recorded inputs. Warm up explicitly, repeat trials, and retain raw results and variance. Use an external generator/receiver where possible and verify it is not the bottleneck. Loopback results and virtual namespaces are useful for correctness but insufficient for NIC capacity claims.

## Hot path rules

- Allocate packet, session, timer, request, shaping and observation storage at startup and reuse it.
- Parse borrowed buffers and avoid unnecessary copies. A zero-copy claim requires measured buffer ownership/lifetime behavior, not a label.
- Avoid blocking I/O, DNS lookup, formatted logging, unbounded scans, dynamic queue growth and per-packet task creation in the packet loop.
- Bound work per dispatch and reserve progress for timers, control traffic, accounting and revocation even under continuous packet arrival.
- Start with a single session owner for straightforward ordering. Establish CPU and packet-rate headroom early; do not assume one worker satisfies every target.
- Add batching, buffer reuse improvements, memory-mapped I/O or partitioned workers only when profiles identify the limiting cost. Preserve per-session ordering and explicit revocation/accounting ownership.
- Keep unsafe platform code isolated and audited. Do not trade packet validation or buffer bounds for a benchmark improvement.

Account for algorithmic costs as tables fill: bounded storage can still have unacceptable search or collision behavior. Measure worst-case table occupancy and adversarial input. Single-owner mutation avoids some synchronization costs but must still have bounded timer, lookup and scheduling work.

## Stability and overload contracts

| Condition | Required behavior |
| --- | --- |
| Session capacity reached | Reject new admission with a specific reason; keep existing sessions progressing |
| Packet or shaping pool full | Apply the declared drop/fairness policy, preserve essential control progress and count the cause |
| RADIUS/portal outage | Apply explicit session/outage policy with bounded pending work, retries and recovery rate |
| Accounting backpressure | Expose lag, retain critical obligations under the declared policy and never invent an acknowledgment |
| Slow or excessive CLI queries | Rate-limit bounded snapshots and reports; do not stall the session owner |
| Link flap or adapter failure | Report degraded state and prevent an alternate unauthorized forwarding path |
| Daemon restart or crash | Retain required host guard; apply documented reauthentication or recovery behavior |
| Malformed or adversarial traffic | Reject safely with bounded work and diagnostic rate limits |
| Partial config or tuning application | Report applied effects and restoration failures; never claim readiness from desired state alone |

No retry queue can guarantee indefinite retention during an unbounded outage. Decide outage duration, critical-event capacity, storage strategy and admission behavior before promising accounting durability. Diagnostics must expose exhausted capacity and lost evidence. A clean restart and a hard crash may have different recovery guarantees; state both.

## Qualification matrix

Test normal load, target capacity, short bursts and sustained offered load above capacity. Combine high session counts with low per-client bandwidth, active shaping, bidirectional traffic, session churn, accounting retries and operator queries. Add one failure at a time to isolate causes, then test combinations representative of deployments.

Soak across long-lived sessions, repeated client reuse, multiple accounting cycles, large cumulative counters and wall-clock changes. Choose a concrete duration in the release profile; a short smoke test is not a soak result. Verify stable memory after warmup, control-plane progress, timely revocation, accounting continuity and recovery after the fault ends.

Run parser fuzzing and targeted state-machine tests for stale events, duplicates, reorderings, timeouts, capacity exhaustion and session reuse. Persist reduced failures as regression fixtures. Verify that diagnostic findings identify injected bottlenecks and distinguish insufficient evidence from a confirmed cause.

## Benchmark artifacts and release gates

Store benchmark scenario definitions and report schemas in the repository when the harness is implemented. Results include source revision, build profile, compiler, config hash, hardware/firmware/driver/kernel, NIC settings, CPU placement, generator version, traffic mix, duration, sample coverage and raw metrics. Keep secrets out of artifacts. Do not compare materially different environments without labeling the difference.

Set numeric pass/fail thresholds after baseline discovery. At minimum, gate target packet capacity and tail latency, access-policy correctness, revocation latency under load, zero measured steady-state allocation on designated paths, memory stability, maximum accounting lag, defined overload behavior and diagnostics overhead. A performance gain cannot compensate for an enforcement or accounting correctness failure.

Run inexpensive deterministic checks on ordinary changes. Use a repeatable target-hardware benchmark for performance-sensitive changes and release qualification; noisy shared CI results must not silently redefine capacity. Re-run affected load/failure scenarios after packet I/O, scheduling, counter, memory or observation changes.
