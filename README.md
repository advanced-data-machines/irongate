# IronGate

A Rust network access gateway project starting from CoovaChilli's behavior and deployment requirements. The intended scope includes captive portal access, RADIUS authentication and accounting, DHCP integration, per-client policy, quotas, and bandwidth enforcement.

IronGate currently provides offline TOML validation through `ig config check` and read-only Linux interface preflight through `ig doctor`. Normal deployment does not serve DHCP, authenticate clients, forward traffic, or configure the host network. The separate guest-only packet lab now exercises real DHCP/ARP and consent-gated IPv4 transit inside QEMU. Broader NIC tools and apply commands remain planned.

The [MVP plan](docs/mvp-plan.md) works backward from a real captive portal demo: local consent first, then HTTPS login with external RADIUS authentication/accounting, followed by two per-VLAN portal profiles. Live testing will stay in a disposable VM. The [roadmap](docs/roadmap.md) distinguishes that lab MVP from production qualification.

Portal configuration explicitly distinguishes [local on-device and external/cloud hosting](docs/portal-hosting.md), including tenant-scoped metadata and per-VLAN placement. See [the cloud example](examples/cloud-portal.toml) and [the proposed isolated VM](docs/vm-demo.md). These are validated configuration contracts; portal services remain unimplemented.

## Try the CLI

```sh
cargo run -p irongate-cli -- --config examples/standalone.toml config check
cargo run -p irongate-cli -- config check --config examples/vlan-portals.toml --json
```

These commands read the selected TOML file only. They do not access referenced secrets, resolve service names, open network sockets, or change host networking. The examples use documentation-only service addresses, so a passing result includes warnings and explicitly leaves runtime readiness unverified. See [configuration validation](docs/configuration.md).

For passive interface checks:

```sh
cargo run -p irongate-cli -- doctor --host
cargo run -p irongate-cli -- doctor --config examples/vlan-portals.toml --network staff --json
```

Doctor includes host interface state, active-link evidence, MTU/driver metadata, and driver-reported speed and duplex when available. `--host` needs no TOML. It sends no traffic and changes no settings. Example interface names may be absent on this PC; doctor reports that without creating or modifying them. A pass covers local interfaces only, with backend health and gateway behavior still unverified. See [doctor scope and exit codes](docs/doctor.md).

With [just](https://github.com/casey/just) installed, the common commands are:

```sh
just                 # List commands
just build
just fmt
just test
just check           # Formatting, Clippy, and tests
just ig config check --config examples/standalone.toml --json
```

## Workspace

| Crate | Responsibility | Current state |
| --- | --- | --- |
| `irongate-core` | Session ownership, policy, accounting, quotas, and timers | Offline bounded leases and consent/admission state, retained policy effects and generation-safe handles; `no_std`, no dependencies or allocator imported |
| `irongate-proto` | Borrowed protocol parsing and bounded encoding | Offline Ethernet/IPv4/UDP/DHCP requests and OFFER/ACK/NAK; `no_std`, no runtime dependencies or allocator imported |
| `irongate-config` | Schema, cross-field checks and minimal host requirements | Bounded input and findings; borrowed TOML views where supported |
| `irongate-cli` | Operator command `ig` | `config check`, passive `doctor`, text/JSON results, help and version |
| `irongated` | Host process, configuration, adapters, and event loop | Normal mode prints scaffold status; guarded Linux lab mode handles DHCP/ARP and admission-gated transit in QEMU |

Use Rust 1.100 or newer with edition 2024 support. Validation used Rust 1.100.0, matching the declared minimum. This baseline permits evaluation of the stable allocator API; no custom allocator is currently used. Four direct third-party crates support configuration/address parsing and JSON output; the Linux lab adds narrowly enabled `rustix` polling; see the [dependency policy](docs/dependencies.md). The core and protocol libraries have no third-party or runtime dependencies; protocol examples/tests use the workspace core as a development dependency. Offline builds require dependencies to have been cached already.

```sh
cargo build --workspace --offline
cargo run -p irongated --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --offline -- -D warnings
cargo test --workspace --offline
cargo doc --workspace --no-deps --offline
```

Tests cover the example profiles, conflicting VLANs/subnets, provider references, resource limits, malformed input, redaction and actual CLI output/exit codes. The guest-only lab now exercises live forwarding and the admission owner; normal deployment, portal services and RADIUS integration remain unimplemented. A successful configuration check establishes internal consistency for the supported profile, not gateway correctness or measured capacity.

Doctor tests use private filesystem fixtures for link evidence, shared trunks, scope selection and incomplete observations. Keep `lib.rs` and `main.rs` as short entry points; put related behavior in focused, readable modules.

The [offline protocol pilot](docs/protocol-pilot.md) parses borrowed direct-client DHCP requests and encodes broadcast OFFERs into caller-owned buffers, including a single VLAN tag. The protocol library owns no leases or sockets; the separate core owns lease state, and the guarded Linux lab adapter connects them to real guest packet I/O. `just protocol-allocations` optionally uses system Valgrind to compare heap totals for zero versus 100000 packet replays; it requires Python and Valgrind, without new Rust dependencies.

The [offline lease foundation](docs/lease-core.md) adds bounded ownership, generation-safe reuse, renewal, release, decline/quarantine, expiry and ACK/NAK replies. `just lease-demo` runs a complete synthetic exchange with no networking; `just lease-allocations` checks 100000 full lifecycle replays with Valgrind. A lease does not authorize upstream traffic. The pilot supports at most 256 addresses per pool; daemon/config integration and real-client interoperability remain future work.

The [offline session foundation](docs/session-core.md) adds local consent attempts, explicit policy acknowledgements, expiry/revocation and retained cleanup. Run `just session-demo` to see admission remain denied until application is confirmed, or `just session-allocations` for the churn allocation check. The worker is simulated: real traffic enforcement and a portal service are still pending.

The [Linux packet lab](docs/linux-packet-lab.md) now boots a fresh NIC-free QEMU overlay and verifies real client DHCP/ARP, native local TCP, bidirectional consent-gated transit, logout, expiry and crash guarding. Use `just vm-lab-command` to inspect the command, `just vm-lab-prepare` for the separate guest package image, and `just vm-lab-test` for the automated scenario. Guest artifacts travel on a read-only seed disk; no host directory/network is shared. This is a fixed one-client lab profile with root-only test consent, not a production gateway or portal.

`just coova-compare` builds selected original C functions from the pinned sibling CoovaChilli revision and compares shared option/OFFER fixtures, independent wire validation, sanitizers, batch timings and allocation totals. It needs system C/Clang tooling and Valgrind; see [comparison scope and findings](docs/coova-comparison.md). It does not run either gateway or change host networking.

Use `just comparison-check` for framework regressions without C tools, or `just coova-compare --checks-only --suite dhcp-options` for a selected correctness/sanitizer suite. The [reusable framework](docs/comparison-framework.md) documents extension contracts, saved baselines and explicitly matched versus unmatched performance evidence.

## Design and progress

- [Architecture and migration choices](docs/architecture.md)
- [Memory policy and capacity budgeting](docs/memory.md)
- [Roadmap and progress checklist](docs/roadmap.md)
- [CoovaChilli behavior and source map](docs/coova-compatibility.md)
- [Setup, diagnostics and behavior verification](docs/operations.md)
- [Ethernet inspection and tuning utilities](docs/nic-tools.md)
- [TOML and automation CLI contract](docs/cli-contract.md)
- [RADIUS TOML profile](examples/irongate.toml)
- [Session accounting and throughput diagnostics](docs/accounting-diagnostics.md)
- [Performance and stability release criteria](docs/performance-stability.md)
- [Release evidence and initial quality checkpoint](docs/release-evidence.md)
- [Standalone providers and RADIUS configuration and health](docs/authentication-radius.md)
- [Deployment sizing and backend capacity](docs/deployment-scaling.md)
- [Per VLAN portals and network isolation](docs/vlan-networks.md)
- [Standalone TOML profile](examples/standalone.toml) and [VLAN portal profile](examples/vlan-portals.toml)
- [Implemented configuration checks](docs/configuration.md)
- [Read only doctor](docs/doctor.md)
- [Offline protocol pilot and allocation evidence](docs/protocol-pilot.md)
- [Bounded lease foundation and offline DHCP demo](docs/lease-core.md)
- [Offline session admission, policy effects and revocation](docs/session-core.md)
- [Offline Coova comparison and findings](docs/coova-comparison.md)
- [Comparison framework and regression baselines](docs/comparison-framework.md)
- [Captive portal MVP implementation plan](docs/mvp-plan.md)
- [Local/external portal hosting and tenant scope](docs/portal-hosting.md)
- [Proposed QEMU/KVM demonstration](docs/vm-demo.md)
- [QEMU lesson one and first boot commands](docs/qemu-first-boot.md)
- [Live Linux packet lab, commands and evidence](docs/linux-packet-lab.md)
- [Two-page authentication and performance brochure](docs/irongate-performance-brochure.pdf) and [editable print source](docs/irongate-performance-brochure.html)
- [Dependency choices and binary data direction](docs/dependencies.md)
- [Development PC and isolated VM testing](docs/testing.md)

The first milestone is a deployment profile, measured baseline, and a tested userspace forwarding design. Keep policy and protocol behavior portable behind thin Linux adapters, and treat simple setup and observable correctness as release requirements. The first implementation should demonstrate a complete bounded protocol path, explicit exhaustion behavior, and diagnostics that explain why it works or fails.

## License and provenance

This scaffold uses GPL-2.0-or-later, matching the headers in the CoovaChilli source examined for the migration. See [COPYING](COPYING). No CoovaChilli C implementation has been copied into the Rust crates. Retain applicable attribution and record source provenance when porting implementation code.
