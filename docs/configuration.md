# IronGate configuration validation

`ig config check` implements schema version 1 for the three checked-in example profiles. It reads a regular UTF-8 TOML file and checks syntax, types, supported values and cross-field rules. It neither inspects nor changes host networking and does not read secret files. Passing means static consistency only; the daemon's gateway functions remain unimplemented.

## Commands and results

```sh
cargo build -p irongate-cli
./target/debug/ig --config examples/standalone.toml config check
./target/debug/ig config check --config examples/vlan-portals.toml --json
./target/debug/ig --help
```

Without `--config`, the file is `irongate.toml` in the current working directory. `--format text|json` and `--json` select output. Options work before or after `config check`; duplicate/conflicting format or config options are rejected. `--no-color` is accepted and output is always plain. Help/version are human-readable even when JSON is requested. Unimplemented commands/options are rejected rather than silently ignored; the separate [doctor command](doctor.md) now provides passive interface preflight.

Exit 0 means static validation passed, possibly with warnings. Exit 2 means invalid command, input, schema or field relationships. Exit 3 means permission to read the input was denied. Exit 7 indicates an output/internal failure. JSON is one document on stdout, including validation and argument errors; expected JSON failures do not emit additional stderr text. Text argument errors use stderr. Report schema version 1 is independent of input schema version 1.

Results expose stable finding codes, field paths, corrective actions, input path, completion, truncation, and `runtime_verified = false`. Array indexes in field paths are zero-based. Syntax and schema findings include one-based source positions where available; semantic findings identify the exact schema field. Invalid supplied values and source lines are not echoed. Text output escapes control characters in paths and field names; JSON encoding uses `serde_json`.

## Supported profile and checks

All known fields are explicit in the examples; unknown keys and wrong types are errors. Durations and capacities are nonnegative integers with schema-defined ranges; required resources and durations must be positive. Counts are at most u32, VLAN/port fields at most u16, and retries at most u8. Journal byte limits may use u64. Names use 1–64 ASCII letters, digits, hyphens or underscores. Initial interface names use 1–15 ASCII letters/digits plus dot, underscore or hyphen.

| Area | Current checks |
| --- | --- |
| Host intent | Managed/preconfigured mode, userspace forwarding, unmatched traffic dropped, distinct subscriber/upstream bindings |
| Networks | Unique names; interface/VLAN bindings unique; explicit untagged or VLAN 1–4094; IPv6 blocked |
| IPv4/DHCP | Canonical unicast subnet, prefix 1–30, nonoverlapping profiles, usable gateway/range addresses, ordered range excluding gateway, enough leases for the session ceiling |
| DNS | Nonempty array of unicast IPv4 addresses; no resolver access |
| Portal | Absolute HTTP(S) URI, no embedded credentials/fragment, valid optional port; absolute secret-file reference; built-in consent bound to its own gateway:port with terms-file reference |
| Providers | Initial combinations are external_uam/radius/radius or builtin_consent/consent/local; unsupported combinations and inapplicable fields are errors |
| RADIUS | Unique pool/server names and endpoints, resolvable pool references, literal unicast IP endpoints, UDP, nonzero ports/priority/timeout, absolute secret references, explicit status-probe selection |
| Accounting | RADIUS interim interval at least 60 seconds; local journal path/budget and explicit full-journal policy |
| Outage policy | Initial RADIUS profile denies new sessions and retains previously granted policy for existing sessions |
| Capacity | Network/global limits cover profiles and nonovercommitted session ceilings; packet buffers hold Ethernet/VLAN frames; shaping cannot consume every buffer; RADIUS profiles have request slots |

Repeated RADIUS priorities are permitted; precise equal-priority scheduling is a later runtime contract. Global session capacity may exceed the sum of profile ceilings, but each profile must have enough leases. The packet storage summary is count times size, excluding all other overhead; it is not a RAM guarantee. MTU/frame validation excludes FCS and any future backend headroom. No per-NIC capacity is inferred.

Documentation-only 192.0.2.0/24 service addresses and unencrypted external portal HTTP generate warnings. Unused RADIUS pools and timeout/attempt budgets over 60 seconds also generate warnings. A warning does not establish reachability, production readiness or adequate performance. TLS RADIUS, relay, overlapping subscriber subnets, local users/vouchers and additional provider combinations require explicit implementation and tests before the schema accepts them.

## Bounds and limitations

The input limit is 1 MiB; at most 256 findings are retained, with an explicit truncation flag. The TOML parser's recursion limit remains enabled. Configuration parsing and reporting allocate outside the future packet loop. The parser may borrow strings directly from the input; escaped strings and parser structures can allocate, so this is not an entirely zero-copy parser.

Referenced paths are checked for absolute syntax only. Their existence, permissions and content remain unverified. The input itself must be a regular file; symlinks to regular files work. This checker is not a security boundary against hostile concurrent replacement of a supplied filesystem path, and it makes no deadline guarantee for a stalled network filesystem. Use repository-local files for development tests.

Config check itself performs no host health inspection. The separate [doctor command](doctor.md) reads local interface state only. Config editing, deployment plans, apply, daemon reload and automatic repair remain unimplemented. The [CLI design contract](cli-contract.md) describes future interfaces separately.
