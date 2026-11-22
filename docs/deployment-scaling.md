# IronGate deployment sizing and scale

Provide documented deployment profiles with measurable capacity and failure behavior, not a claim that one configuration scales to any installation. Size the gateway dataplane, identity service, accounting pipeline, address space and operator tooling independently. The scaffold has no validated large-scale capacity yet.

## Configuration and sizing workflow

1. Declare hardware/NIC/OS, sites and gateways, subscriber address space, active sessions, authentication/churn rate, traffic mix and bidirectional packet rate.
2. Select standalone or centralized identity and accounting, latency objectives, accounting freshness, permitted outage duration and restart policy.
3. Compute capacity and memory budgets, inspect capabilities and validate the effective TOML configuration.
4. Run a representative benchmark and an outage/failover test. Export the config hash, hardware profile, measurements and actual limits together.
5. Install using a concrete plan, verify from a subscriber vantage point, and retain baseline evidence for `ig doctor` comparisons.

Proposed tools are `ig capacity estimate --config PATH --workload WORKLOAD.toml` for transparent calculations and `ig doctor --scope capacity --json` for measured headroom. The estimator must list assumptions and cannot certify throughput from CPU count or configuration alone. These commands are not implemented yet.

## Capacity dimensions

| Dimension | Inputs and constraints |
| --- | --- |
| Subscribers | Session slots, usable leases, client indexes, timers, generation history and local credentials/vouchers if used |
| Dataplane | Packet sizes/rate, bidirectional bandwidth, active flows, mutation, shaping queues, CPU and NIC behavior |
| Authentication | Peak attempts and multi-step exchanges per second, response latency, deadlines and pending request slots |
| Accounting | Active sessions, update intervals, session churn, response latency, retry amplification and durable backlog |
| Diagnostics | Retention, per-session detail, snapshot capacity and concurrent query limits |
| Platform | Socket/driver memory, device queues, route/guard resources, descriptor limits and target hardware |

Show the limiting capacity rather than merely the requested value. Session capacity exceeding available leases does not create more usable client addresses. A low average byte rate can still produce a large packet-processing or accounting load.

## Accounting and request budgets

For steady sessions with one cumulative interim update every `T` seconds, the baseline fleet-wide update rate is approximately `N / T`, before Start/Stop, retries and probes. Sum per-gateway load at each shared destination. Examples with `T = 60` seconds:

| Active sessions across a fleet | Baseline interim events per second |
| --- | --- |
| 1,000 | 16.7 |
| 10,000 | 166.7 |
| 100,000 | 1,666.7 |

These are arithmetic estimates, not demonstrated device capacity. Measure session churn and failures because they add work. Desynchronize scheduled updates without violating the effective interval; do not delay Stop events to flatten a chart. Avoid shortening intervals without showing the resulting backend load and freshness tradeoff.

Estimate average pending requests as arrival rate times response time, then size for measured tails, bursts, multi-step exchanges and outage/retry policy. During an outage, response-time estimates from healthy operation no longer size the backlog. State separately the in-flight protocol capacity, waiting queue, durable event backlog and discard/admission policy; increasing one does not increase the others.

RADIUS has an eight-bit Identifier field, so safe concurrency must account for identifier availability and response correlation in the selected transport/peer context. A large queue alone does not permit arbitrary simultaneous requests. Source sockets and transport pools require bounded memory, source/NAS identity consistency and tested late-response handling. See the [RADIUS packet format](https://www.rfc-editor.org/rfc/rfc2865.html#section-3).

Provision surviving backends for declared failover load, including health checks and recovery traffic. Multiple configured servers are redundancy only when their data, realm routing and accounting semantics support it. Show why each request selected a server and whether it is waiting for a slot, retry deadline or recovery policy.

## Single gateway and multiple gateways

Scale a single gateway only within its measured packet, latency, memory and control-plane envelope. Introduce multiple gateways for distinct sites or explicit subscriber partitions, with unique NAS and session identities, nonconflicting address ownership and a reproducible config profile. Aggregate accounting and backend capacity across the fleet.

Do not describe two gateways as high availability without defining traffic ownership, lease coordination, authorization transfer and accounting continuity. The initial rollout may require reauthentication after gateway failure. Transparent failover, roaming, global voucher redemption and cross-gateway quotas require shared coordination or explicit constraints; local per-gateway counters cannot provide a global quota by themselves.

Separate reusable site templates from explicit rendered TOML. Avoid hidden runtime overlays; archive the effective config and hash per gateway. Roll changes through a canary and observe failover/recovery headroom before expanding. Keep management access distinct from subscriber forwarding.

## Documentation and release evidence

Publish a small standalone quickstart and a separate tested centralized-backend guide. Each measured scale profile includes the exact workload, hardware/NIC settings, authentication method, update interval, capacities, memory usage, achieved packet rate, latency tails and behavior under failure. Profiles are examples with declared limits, not arbitrary small/medium/large labels.

Acceptance requires startup validation of impossible capacities, meaningful warnings for untested profiles, stable behavior at the declared limit, bounded overload, and recoverable accounting backlogs after a documented outage. Link sizing findings to [performance gates](performance-stability.md), [memory budgeting](memory.md), [accounting diagnostics](accounting-diagnostics.md) and [provider health](authentication-radius.md).
