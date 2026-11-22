# IronGate Ethernet inspection and tuning

Provide useful Ethernet diagnostics with an optional, explicit tuning workflow. There is no universal optimal NIC configuration: the result depends on the driver, hardware, MTU, CPU topology, traffic profile and userspace packet backend. Inventory and measurement precede recommendations. These utilities are planned; no settings are changed by the current scaffold.

## Operator workflow

| Proposed command | Behavior |
| --- | --- |
| `ig nic inspect --interface IFACE --json` | Read-only identity, supported capabilities, current settings and counters |
| `ig nic watch --interface IFACE --seconds 30` | Bounded sampling of counter deltas and link changes; no generated load |
| `ig nic plan --interface IFACE --profile userspace --output nic-plan.json` | Candidate settings, rationale, expected tradeoffs and rollback values; no mutation |
| `ig nic apply --plan nic-plan.json` | Explicit privileged application with host-state revalidation and readback |
| `ig nic restore --snapshot nic-before.json` | Restore captured writable settings where supported; report every failure |

Inspect must expose unavailable permissions, unsupported features and fixed driver settings. It must not equate an unsupported setting with disabled or failed hardware. Planning requires a declared backend and workload; a baseline-only profile should preserve supported current settings.

## Inventory and decision inputs

Record interface name and index, permanent and current MAC where available, PCI/device path, driver/firmware identity, virtual/physical relationship, namespace, bridge/VLAN membership, carrier, negotiated speed/duplex, autonegotiation, MTU, supported offloads, channels/queues, RSS configuration, rings, interrupt coalescing, pause settings and available error/drop counters. Inspect interrupt/CPU and NUMA placement where available. Driver counters differ, so attach definitions or clearly identify unknown meanings.

| Area | Analysis |
| --- | --- |
| Link and errors | Flag unexpected speed/duplex, flaps, CRC errors and sustained drops before changing queues |
| Queues and RSS | Compare available queues, packet distribution and interrupt CPU load with the application's actual workers |
| Rings and buffering | Increase only for demonstrated burst loss within the memory/latency budget; larger buffers are not automatically better |
| Interrupt coalescing | Measure latency versus CPU and interrupt load; consider adaptive support and workload |
| Offloads | Validate GRO/LRO, GSO/TSO, checksums and VLAN metadata against the exact packet backend |
| Flow control and energy settings | Report capabilities and symptoms; changes require topology and workload evidence |

Prefer the Linux ethtool netlink interface where supported and isolate any ioctl or external-tool fallback in the Linux adapter. Prefer structured APIs to parsing human-oriented command output. Future platforms translate their own device APIs into the same capability report. Device tuning is platform-specific even when the report and recommendation rules are shared.

## Backend correctness before throughput

Coalesced packets, deferred checksum work and VLAN stripping can change what a userspace packet reader observes. The adapter must normalize or explicitly report these representations, or the plan must identify a specific incompatible setting. A capture showing an incomplete checksum is not by itself evidence of a corrupt wire packet. Test with an independent capture or traffic endpoint where required.

Do not disable every offload by default. Support a known-correct baseline for the selected backend, then test candidate changes one at a time. Correctness includes MTU/fragment behavior, VLAN isolation, accounting byte definitions, packet mutation and wire checksums. The portable core receives a documented packet representation rather than implicit Linux offload assumptions.

A single-owner event loop may not benefit from every hardware queue. Preserve session ordering and match queue recommendations to measured CPU bottlenecks. Inspect existing IRQ balancing or affinity management before proposing competing changes. Do not enable RPS simply because it is available.

## Applying and verifying changes

Plans include device identity, namespace, original settings, proposed values, driver capability evidence, intended workload, management-path impact and validation steps. Reject a stale plan or a device that has been renamed/replaced. Changes can interrupt the link; never apply tuning as a side effect of daemon startup or ordinary doctor checks.

Read settings back after each change and record settings the driver rounds or refuses. A sequence of device changes is not necessarily atomic. On failure, attempt restoration of the prior applied settings and report partial restoration. Restoring settings does not promise restoration of dropped traffic or disrupted sessions. Avoid changes to speed, firmware or permanent device configuration in the first utility scope.

Keep tuning persistent only through an explicit exported configuration for the chosen host manager; do not install competing boot scripts silently. Export a machine-readable before/after report and a bounded verification procedure. Active load generation is a separate explicit operation in a lab or declared maintenance window.

Compare representative packet sizes and session counts using CPU, throughput, loss, latency percentiles and packet correctness. Select a recommendation only when it improves the declared objective without breaking access policy or exceeding the memory budget. If no improvement is demonstrated, retain the baseline and say so.

## Linux references

- [Kernel networking scaling guidance](https://docs.kernel.org/networking/scaling.html) for RSS, RPS, RFS and XPS tradeoffs.
- [Ethtool netlink interface](https://docs.kernel.org/networking/ethtool-netlink.html) for capability queries and device configuration.
- [Kernel checksum offloads](https://docs.kernel.org/networking/checksum-offloads.html) for checksum metadata and completion semantics.
- [TUN and TAP interface](https://docs.kernel.org/networking/tuntap.html) for the userspace/kernel packet boundary.
