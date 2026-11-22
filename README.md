# IronGate

A Rust network access gateway project starting from CoovaChilli's behavior and deployment requirements. The intended scope includes captive portal access, RADIUS authentication and accounting, DHCP integration, per-client policy, quotas, and bandwidth enforcement.

IronGate currently provides offline TOML configuration validation through `ig config check`. It does not serve DHCP, authenticate clients, forward traffic, or configure the host network. Doctor, NIC tools and apply commands remain planned.

## Try the CLI

```sh
cargo run -p irongate-cli -- --config examples/standalone.toml config check
cargo run -p irongate-cli -- config check --config examples/vlan-portals.toml --json
```

These commands read the selected TOML file only. They do not access referenced secrets, resolve service names, open network sockets, or change host networking. The examples use documentation-only service addresses, so a passing result includes warnings and explicitly leaves runtime readiness unverified. See [configuration validation](docs/configuration.md).

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
| `irongate-core` | Session ownership, policy, accounting, quotas, and timers | Empty `no_std` library; no dependencies or allocator imported |
| `irongate-proto` | Borrowed protocol parsing and bounded encoding | Empty `no_std` library; no dependencies or allocator imported |
| `irongate-config` | Schema and cross-field checks | Static validator with bounded input and findings; borrowed TOML views where supported |
| `irongate-cli` | Operator command `ig` | `config check`, text/JSON results, help and version |
| `irongated` | Host process, configuration, adapters, and event loop | Prints scaffold status and exits |

Use Rust 1.85 or newer with edition 2024 support. Validation used Rust 1.100.0; the declared minimum has not been tested. Four direct third-party crates support configuration/address parsing and JSON output; see the [dependency policy](docs/dependencies.md). The core and protocol crates remain dependency-free. Offline builds require dependencies to have been cached already.

```sh
cargo build --workspace --offline
cargo run -p irongated --offline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --offline -- -D warnings
cargo test --workspace --offline
cargo doc --workspace --no-deps --offline
```

Tests cover the example profiles, conflicting VLANs/subnets, provider references, resource limits, malformed input, redaction and actual CLI output/exit codes. Protocol forwarding and session behavior remain unimplemented. A successful configuration check establishes internal consistency for the supported profile, not gateway correctness or measured capacity.

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
- [Standalone providers and RADIUS configuration and health](docs/authentication-radius.md)
- [Deployment sizing and backend capacity](docs/deployment-scaling.md)
- [Per VLAN portals and network isolation](docs/vlan-networks.md)
- [Standalone TOML profile](examples/standalone.toml) and [VLAN portal profile](examples/vlan-portals.toml)
- [Implemented configuration checks](docs/configuration.md)
- [Dependency choices and binary data direction](docs/dependencies.md)
- [Development PC and isolated VM testing](docs/testing.md)

The first milestone is a deployment profile, measured baseline, and a tested userspace forwarding design. Keep policy and protocol behavior portable behind thin Linux adapters, and treat simple setup and observable correctness as release requirements. The first implementation should demonstrate a complete bounded protocol path, explicit exhaustion behavior, and diagnostics that explain why it works or fails.

## License and provenance

This scaffold uses GPL-2.0-or-later, matching the headers in the CoovaChilli source examined for the migration. See [COPYING](COPYING). No CoovaChilli C implementation has been copied into the Rust crates. Retain applicable attribution and record source provenance when porting implementation code.
