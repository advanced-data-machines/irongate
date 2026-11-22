# IronGate dependency choices

Use established crates when they remove meaningful correctness risk, with a small explicit feature set and a clear boundary. Avoid adding a framework or subsystem for hypothetical future use. The current operator/configuration milestone has four direct third-party dependencies; the core and protocol libraries remain free of runtime/third-party dependencies. The guest-only Linux daemon adapter additionally uses `rustix` for safe readiness polling.

The initial doctor backend uses standard-library sysfs reads and adds no dependencies. Keep entry points short and organize related parsing, rules, I/O and rendering into focused modules; introduce shared abstractions only when another real caller needs them.

The [protocol pilot](protocol-pilot.md) records an initial documentation review of `etherparse`, `edge-dhcp` and `dhcproto`. It is a small bounded experiment, not a decision to build a general network stack. Compare candidates against the pilot corpus before expanding protocol coverage. Optional allocation instrumentation uses system Valgrind and standard-library Python, with no Rust allocator wrapper or additional crate.

The optional [Coova comparison harness](coova-comparison.md) also requires system C/Clang tooling and a sibling Git checkout containing the pinned upstream source. It generates temporary C derivatives under `target`, preserves upstream notices, and adds no C linkage or dependency to the production Rust crates. Python orchestration uses only the standard library.

| Crate | Purpose | Enabled scope |
| --- | --- | --- |
| `toml` | Standards-compliant TOML parsing with source spans and borrowable strings | Only parse/std; default Serde deserialization, formatting and optional order-preserving/hash features disabled |
| `ipnet` | CIDR parsing, containment, canonical network and broadcast addresses | Small IPv4 subnet operations; no platform networking |
| `http` | Parse portal HTTP(S) URI structure | URI/authority types; no HTTP client/server or runtime |
| `serde_json` | Correct JSON escaping and machine output | Operator reports only, outside packet processing |

`serde_json` uses `serde_core` internally. TOML has a dependency named `serde_spanned`, whose Serde integration is disabled in the active feature graph. Do not describe this build as having no Serde-related dependencies. There are no direct Serde derives, CLI framework, async runtime, database, TLS stack, web framework or binary archive dependency.

Inspect the actual active graph rather than just the lockfile: Cargo may record optional packages that are not built. Use `cargo tree --workspace --edges normal` and `cargo tree -p irongate-config --edges features`. Keep Cargo.lock checked in. Dependency additions should explain their actual use, active/transitive cost, maintenance and target/MSRV implications in the change description. Test the supported behavior instead of assuming a parser validates every application-level constraint; portal port limits are one such constraint.

## Linux lab adapter review

The [guest-only packet lab](linux-packet-lab.md) adds `rustix 1.1.5` to the Linux daemon with default features disabled and only `std`/`event` enabled. Its concrete use is safe `poll` on already-bound, inherited packet/control descriptors; the guest supervisor uses Python's standard socket API for setup. This avoids handwritten unsafe syscall/FD construction while retaining the project's `forbid(unsafe_code)` lint. The crate declares Rust 1.65 compatibility, below this workspace's minimum.

On this Linux target, `cargo tree -p irongated --edges normal --offline` shows only `rustix`, `bitflags` and `linux-raw-sys` as new active packages. Cargo.lock also records backend/platform alternatives such as `errno`, `libc` and Windows packages; these are not in that active Linux graph. No socket framework, packet stack or async runtime is added. `serde_json` is reused for bounded lab controls outside the packet loop. Kernel/network namespace mutation stays in explicitly invoked guest harness code. Physical-NIC capability/offload/privilege qualification is still pending.

## Zero copy and binary archives

Prefer borrowed byte views and caller-owned buffers in packet handling. Evaluate `rkyv` for a concrete binary snapshot or IPC layout after defining versioning, size limits, validation, alignment, lifetime and compatibility requirements. It is not a TOML parser, and a binary archive does not remove validation or session ownership requirements. Do not add it to this milestone without a binary boundary to implement and measure.

Use a vetted serialization/parsing crate instead of hand-writing a general JSON or TOML implementation. Conversely, the current CLI's small fixed argument vocabulary is handled with the standard library; revisit a CLI framework when the command surface makes that simpler and less error-prone.
