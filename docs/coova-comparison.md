# Offline comparison with CoovaChilli

`just coova-compare` compares the Rust protocol pilot with selected original C functions from CoovaChilli commit `61854f91a5d159bdfe8a38794b99d86df81fddc7`. All inputs are synthetic; sends are intercepted into memory. No daemon runs, no sockets open, and neither repository's host network configuration is changed.

This now runs through a [reusable comparison framework](comparison-framework.md): named/versioned suites, selection/checks-only modes, schema-2 reports, saved same-implementation timing baselines and explicit failure artifacts. Both current workloads are marked unmatched across implementations. The recorded findings below remain the initial evidence, not proof of a language speed advantage.

## Run and artifacts

Required development tools are Python 3, Git, a C compiler (`cc` by default), Clang with AddressSanitizer/UBSan, Rust 1.100+, and Valgrind. No new Rust or Python packages are installed. The sibling checkout must contain the pinned Git object; local tracked/untracked changes are ignored and left untouched.

```sh
just coova-compare
# Alternative checkout and explicit workload controls:
python3 tools/coova_compare/run.py --coova /path/to/coova-chilli --iterations 200000 --samples 5
```

Output stays under `target/coova-compare`: readable `report.md`, structured `report.json`, optimized/sanitized C probes, and generated source. The report records the upstream revision, original function/blob hashes, working Rust/harness source hashes, binary hashes, input hashes, compiler flags, build profile, machine/affinity, individual timing samples, allocation totals and classified differences. The Rust Git HEAD alone is insufficient when changes are uncommitted; use the source and binary hashes too.

`--skip-memory` explicitly omits Valgrind and marks memory evidence skipped. Missing compilers, sanitizer failures, unexpected differences or allocation growth fail the run. Sanitizers require an environment where they can inspect child processes: the initial sandbox run failed because LeakSanitizer cannot operate under ptrace; rerunning outside that sandbox succeeded, with leak detection still enabled. This workflow does not silently disable sanitizers or escalate itself.

## What is compiled

The builder reads immutable blobs with `git show REVISION:path`, extracts complete function text without editing the bodies, and uses upstream `pkt.h` wire layouts/macros and DHCP numeric constants. Generated derivatives retain upstream copyright/license notices and source line mappings. There is no hand-translated C reference implementation and no source vendoring into the Rust crate.

| Original function | Comparison role |
| --- | --- |
| `dhcp_gettag` | Lookup one option in a supplied DHCP payload |
| `dhcp_create_pkt` | Original OFFER header/default construction |
| `dhcp_accept_opt` | Original configured reply options |
| `dhcp_handler` | Original hook boundary, with optional hooks disabled |
| `dhcp_sendOFFER` | Original assembled reply; `dhcp_send` is intercepted |
| `in_cksum`, `chksum` | Original checksum generation |

Fixture-only contexts supply deterministic server/client addresses, MACs, lease 3600 seconds, MTU 1500 and one DNS server. These contexts are not ABI-compatible replacements for daemon objects. IEEE 802.1Q is enabled; optional DHCP/RADIUS options, WPAD, location/modules, IPv6 and other-traffic accounting are excluded. Domain/DNS2/captive-portal URI are unset and debug is off. The disabled captive-portal expansion stub aborts if reached, rather than pretending to implement it. This is a deliberately scoped extracted-function build, not a configured build or benchmark of the whole CoovaChilli daemon.

`dhcp_getreq`, address allocation callbacks, sessions, RADIUS, ARP, routing and actual network I/O are not compared yet. Those paths require equivalent Rust state/behavior and an isolated topology before a meaningful comparison. OFFER construction is not proof that a daemon would accept a corresponding input or grant a lease.

## Comparison rules and initial findings

The shared option corpus covers valid DISCOVER/REQUEST, client/unknown/missing options, PAD, duplicates, missing END, truncated target/later values, overload, unknown message types, data after END and exact length boundaries. C lookup status and Rust validated lookup status are separate evidence fields. Rust validates the whole bounded DHCP message before exposing options; Coova's helper only searches for one tag. Finding a tag is not an assertion of complete packet validity or daemon acceptance.

Four shared OFFER inputs cover untagged, tagged client identifier, absent inbound UDP checksum and a cleared broadcast preference. A separate Python decoder validates emitted lengths, checksums, DHCP structure and option bounds. Shared addressing, transaction/client identity, VLAN TCI, ports and common option values must agree. Rust's first three replies must also match the independently generated expected wire fixtures. Differences are listed explicitly rather than erased by checksum/length normalization.

The initial run passed all 18 option and four OFFER cases under both optimized C and ASan/UBSan builds. The framework now has 16 regression tests, including valid-checksum packets with wrong transaction/message identities, a VLAN mismatch, baseline compatibility, alternating sample order and failure-artifact replacement, to ensure meaningful differences are not mistaken for success.

| Observation | Interpretation |
| --- | --- |
| C TTL 16, Rust TTL 64 | Explicit policy difference |
| Untagged C reply 320 bytes, Rust 342; tagged C 324, Rust 346 | Different options/padding; checksums verified independently |
| C includes interface MTU option 26; Rust does not | Missing Rust functionality for this profile; leave visible before deciding whether to add it |
| Rust echoes option 61; this C profile does not | Suspected C compatibility defect relative to [RFC 6842](https://www.rfc-editor.org/rfc/rfc6842.html); validate deployment profiles before a broader claim |
| Broadcast flag clear: C unicasts, Rust broadcasts | The pilot implements broadcast OFFERs only; unicast behavior is a future increment |
| PAD before message type: C helper misses type, Rust finds it | Suspected standalone C lookup defect; not a claim about complete daemon behavior |
| Lookup can find an option after END | Suspected standalone C lookup defect on malformed input; Rust rejects the message |
| C finds an early tag despite malformed later options | Different validation boundaries, not automatically a defect |

This corpus is not fuzzing or a security audit. Preserve correct deployment behavior; do not reproduce a suspected C defect merely to make outputs identical.

## Timing and memory

Both probes consume the same input bytes. Setup/decoding/output formatting are outside their measured loops, one warmup precedes timing, and subprocesses inherit affinity to one allowed CPU on supported Linux hosts. Reported values are medians of five batch-average ns/operation samples, not packet latency percentiles or gateway throughput. C uses `-O3 -fno-strict-aliasing -fno-lto`; Rust uses the workspace release profile with thin LTO. Exact flags and tools are in each report.

Option 61 lookup is timed separately: Rust validates once before timing, then searches its borrowed view; the original C helper searches the same valid payload without whole-message validation. OFFER timing excludes request parsing but includes different packet lengths/options. C also copies its stack-built result into the intercepted-send buffer, while Rust writes directly into caller storage. These observations are useful starting measurements, not a justified C-versus-Rust speedup ratio. Future matched-output/component benchmarks should address those differences before claiming relative performance.

Valgrind runs separately from timing. Identical-width iteration arguments compare one versus 100000 OFFER operations with identical setup/warmup. **Both C and Rust measured zero additional heap allocations**. Initial process totals (allocations, frees, bytes) were `(3, 3, 8538)` for C and `(23, 20, 13796)` for Rust, unchanged by replay count. The process startup/adapters differ, so these totals are not a per-packet or language memory comparison. Allocation-free Rust parsing/encoding is promising, but this C path already avoids repeated allocation too. Neither result establishes session churn stability, RSS bounds, kernel memory use or production performance.

## Next increment

Review the MTU option and broadcast/unicast scope differences before broadening reply compatibility. Add independent client/packet-library decoding and coverage-guided fuzzing, then bounded lease/session ownership. Once Rust has equivalent state transitions, extend the comparison to `dhcp_getreq`, ACK/NAK, renewal/release and capacity exhaustion. Live Coova/client interoperability belongs in a disposable VM; use dedicated gateway hardware later for wire-rate and NIC measurements.
