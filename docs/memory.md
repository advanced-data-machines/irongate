# IronGate memory policy

The target is no general-purpose heap allocation during steady-state packet processing or session creation and removal. Allocate large pools once at startup, then reuse their slots. Startup allocation is allowed; unbounded growth is not. The [offline protocol pilot](protocol-pilot.md) has measured zero additional heap allocations for a bounded DISCOVER/OFFER replay; session storage and the live packet path remain requirements to implement and measure.

## Resource storage

| Resource | Planned storage | Exhaustion behavior |
| --- | --- | --- |
| Packet buffers | Fixed-size buffers in a startup-allocated pool | Drop according to protocol policy; increment an exhaustion counter |
| Sessions | Fixed-capacity arena with free slots and generation identifiers | Refuse new sessions while preserving existing sessions |
| Client indexes | Bounded hash tables or equivalent preallocated indexes | Reject insertion before partially creating a session |
| Requests and retries | Bounded slots with deadlines and retry limits | Refuse or defer authentication within a bounded queue |
| Timers | Fixed-capacity scheduler with a bounded number of entries per session | Reserve timer capacity with session or request allocation |
| DHCP leases | Bounded lease table integrated with address ownership | Return the protocol-appropriate response when no lease is available |
| Logging and telemetry | Bounded records and queues | Drop or aggregate diagnostics instead of growing memory |
| Configuration | Bounded validated snapshot | Reject oversized input; budget old and new snapshots during reload |
| Shaping | Preallocated per-client descriptors and globally bounded queued packets | Enforce declared per-client and global drop policies; reserve control traffic capacity |
| Session diagnostics | Fixed counters, bounded history and snapshot buffers with explicit retention | Evict old observations and report gaps; keep pending accounting obligations separate from disposable history |
| Network profiles | Bounded VLAN/binding index, per-network session ceilings and fair queue budgets | Reject unknown/ambiguous bindings; prevent one VLAN exhausting all shared resources |
| Local authentication and records | Preloaded bounded provider state, bounded work queue and separately budgeted journal storage | Report storage/worker exhaustion and apply explicit admission policy; no blocking disk work in packet processing |

Do not place large arrays on a small thread stack. Use startup-owned storage and borrow slices from it. `no_std` does not by itself prohibit allocation: keep `alloc` and allocating dependencies out of the core and protocol crates. Audit feature flags and transitive dependencies when adding libraries.

Parse borrowed bytes and write responses into caller-owned buffers. Strings, options, and attribute lists need explicit byte and item limits. Avoid per-packet `Vec`, `String`, boxed futures, task spawning, and formatted log allocations. Determine how the chosen runtime and I/O backend behave under load before adopting them.

## Budget model

Define a deployment profile using:

```text
N = maximum sessions
B = packet buffer count
P = bytes per packet buffer, including headroom
R = pending request capacity
T = timer capacity

userspace budget =
    N * measured bytes per session slot
  + B * P
  + R * measured bytes per request slot
  + T * measured bytes per timer slot
  + index storage
  + bounded queues, configuration and diagnostics
  + process, thread stack and library overhead
```

Add separately measured kernel socket buffers, connection tracking, policy tables, shaping state, and driver memory. Packet size and headroom depend on MTU and VLAN requirements. No concrete client capacity or RAM guarantee is selected yet.

## Verification

Instrument allocator calls around repeated packet processing and session churn after startup. Fail a dedicated allocation regression test if those paths allocate or reallocate. Keep the measurement isolated from test-runner and diagnostic allocations.

Measure occupancy and high-water marks for every pool. Exercise exactly-full and over-capacity conditions, repeated slot reuse, delayed responses, queue saturation, reload overlap, and slow external services. Use a long-running soak test to verify stable live allocations and resident memory. Allocation-free Rust processing alone does not establish an absence of fragmentation elsewhere in the process or kernel.

Keep operator utilities outside the packet loop. They may allocate within documented bounds for configuration, device reports and snapshots; cap report size and retention. Do not let a slow diagnostic reader block forwarding or expand an event queue. Account for OS socket buffers and NIC rings even when the policy engine is allocation-free.
