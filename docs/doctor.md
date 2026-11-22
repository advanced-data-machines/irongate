# IronGate read only doctor

`ig doctor` validates one TOML file and reads the configured Linux interfaces from `/sys/class/net`. It reports existence, administrative state, operational state, carrier, local MTU and the exposed device driver name. It opens no network sockets, sends no traffic, executes no external commands, reads no referenced secrets, and writes no host settings.

It also includes a host-wide inventory of visible interfaces, including unused cards, wireless, loopback and virtual devices. Use `ig doctor --host` to request that inventory without loading TOML.

## Commands

```sh
just ig doctor --host
just ig doctor --host --json
just ig doctor --config examples/standalone.toml
just ig doctor --config examples/vlan-portals.toml --json
just ig doctor --config examples/vlan-portals.toml --network staff --json
```

Without `--config`, configured doctor reads `irongate.toml` in the working directory. `--network NAME` limits required preflight checks to that subscriber profile and its upstream; the accompanying host inventory still lists all visible interfaces within its cap. The whole configuration is still validated. An invalid config or unknown profile stops inspection before reading sysfs. The examples use illustrative interface names; missing names on a development PC are expected findings, not a reason to create or reconfigure them. `--host` rejects `--config` and `--network` instead of silently ignoring them.

`--format text|json`, `--json`, `--no-color`, help and version work as for config check. There are no `--client`, `--window`, `--scope`, active probe or repair options yet. Unsupported options fail explicitly. Fixtures are injected through private test code; there is no CLI flag to substitute fake sysfs as host evidence.

## Scope and evidence

### Host inventory

The `host_network` object reports each interface's kind, administrative state, operational state, carrier, active-link evidence, MTU, exposed driver, speed in Mbit/s and duplex. Every attribute has a `status` and nullable `value`; unknown, unavailable and unsupported values retain their reason or error kind. Speed is the driver's current/latest link speed, not measured throughput or the card's maximum capability. Availability depends on driver support; wireless and virtual interfaces may expose no usable speed or duplex. The [Linux sysfs ABI](https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-class-net) defines these attributes.

An active link requires positive administrative and carrier evidence plus an operational state of `up` or `unknown`. Known down evidence reports inactive; incomplete evidence reports unknown. Cached speed/duplex values are suppressed when the link is inactive or cannot be established as active. These observations can race with link changes; they are not an atomic negotiation snapshot or evidence that traffic reaches its destination.

Host-only reports use `scope = host_network`, `config = null`, and result `reported` with exit 0 when enumeration completed. An unused down card or unavailable optional speed does not make inventory fail. Missing/failed enumeration or truncation produces `incomplete` and exit 4; permission failures or an unsupported platform produce exit 3. `host_network.complete` describes enumeration coverage, not availability of every attribute. Counts of active links/devices describe sampled interfaces only.

For configured doctor, inventory is informational and has independent completion/error metadata. Required preflight checks determine the top-level exit code and `complete`; an unrelated down device does not block the selected deployment. Automation needing complete host inventory must also check `host_network.complete`.

### Configured preflight

Each result contains `scope = local_interfaces`, its source, observation time in Unix seconds when available, and `snapshot_atomic = false`. Each check has a stable code, subject, status, required flag, explanation and next action. Observed/expected values and an error kind are included where applicable. The nested `config` result preserves validation warnings and failures.

| Check | Interpretation |
| --- | --- |
| `interface.exists` | Missing interface in an accessible sysfs view is a failure. An unavailable sysfs view is unverified. |
| `interface.admin_up` | Checks `IFF_UP` in hexadecimal interface flags; it does not bring the interface up. |
| `interface.operstate` | Known down, lowerlayerdown, dormant, testing or notpresent states block the local requirement. |
| `interface.carrier` | Reports the driver-provided carrier bit separately from administrative state. |
| `interface.mtu` | Checks that local MTU is at least the selected profiles' required IP MTU. A larger MTU is compatible. |
| `interface.driver` | Reports the final component of the device's driver symlink; no capability or firmware query is made. |

Linux's reported `unknown` operational state is not treated as down. It is shown as unverified and nonblocking when the other required checks pass. Absent device/driver metadata is commonly associated with virtual interfaces and is optional; it must not by itself make a healthy local link fail. Unreadable or malformed required attributes remain unverified, rather than becoming false down reports. See [Linux operational states](https://docs.kernel.org/networking/operstates.html) and the [sysfs networking ABI](https://www.kernel.org/doc/Documentation/ABI/testing/sysfs-class-net).

Shared VLAN trunks are inspected once, using the largest selected profile MTU and retaining each profile/VLAN role. The upstream is checked against that same minimum for the initial forwarding design. This does not establish upstream path MTU, fragmentation support or the device's ability to handle tagged frames. `vlan.path` explicitly remains unverified.

RADIUS authentication and accounting are separately shown as unverified when selected, or disabled when the selected profiles do not use them. DNS/portal reachability and gateway enforcement remain unverified. A configured server, readable NIC or passing local interface report does not establish remote authentication, accounting ingestion, traffic delivery or access isolation.

## Result and exit status

| Result | Exit | Meaning |
| --- | --- | --- |
| `passed` | 0 | Required checks in this local interface scope passed. Optional/unimplemented checks may remain unverified or unsupported. |
| `blocked` | 1 | At least one definite required failure, such as an absent interface, admin-down link or insufficient MTU. |
| `invalid` | 2 | Invalid configuration or unknown network selector; host inspection was skipped. Invalid CLI syntax also uses 2. |
| `incomplete` | 3 | Required evidence is unavailable due to permissions or an unsupported platform backend. |
| `incomplete` | 4 | Required evidence is missing, malformed, unreadable, or inspection exceeded its interface limit. |

An input permission failure retains exit 3 with an invalid input report. If a definite failure and missing evidence coexist, exit 1 takes priority and `complete` is false. Optional metadata/probes do not cause exit 3/4. Completion refers to required evidence in this limited scope, not completeness of all deployment verification. A definitive missing-interface finding does not require reading that absent interface's other attributes.

`runtime_verified` is always false. This milestone does not establish daemon health, routes, addresses, firewall/bypass guards, service conflicts, switch/native VLAN configuration, offloads, backend connectivity or subscriber behavior. No passing result should be interpreted as deployment readiness.

## Bounds and limitations

At most 64 distinct interfaces are inspected per request, after shared-trunk deduplication. Reports show the requested count and retained interface list; excess requests produce an explicit incomplete finding. Use `--network` to narrow the scope. Individual sysfs attribute reads are limited to 256 bytes, with one extra byte used to detect oversize values. Driver names are obtained from a symlink without opening its target.

Host inventory separately retains the lexicographically first 64 visible interface names, counts all discovered entries, and explicitly reports truncation. Sampling and retained storage are bounded, although directory enumeration has no time bound. It does not inspect addresses, routes, link partner capabilities, autonegotiation settings, offloads or packet counters yet.

The current process/sysfs namespace view is the observation boundary. Hidden or different namespaces are not searched. The timestamp labels a best-effort observation window; interface state may change between reads, and doctor does not claim an atomic snapshot or subscribe to events. Access failures are reported without automatic privilege escalation. No timeout guarantee is made for a stalled or substituted filesystem.

## Module layout and tests

Keep entry points as wiring and use focused modules. `irongate-config` separates bounded input, schema, validation, report data and minimal host requirements. The CLI separates arguments, config output, and doctor. Doctor has small modules for grouping requirements, sysfs reads, Linux rules, report/exit policy and rendering. Host inventory separates collection, data, sampling, speed/duplex interpretation and output. Fixture helpers and focused evidence, scope, inventory and link-metric tests live in separate files. No new third-party dependencies were added.

`just check` runs the suite. Doctor tests create only private temporary fixtures and do not inspect the host. They cover absent versus unavailable interfaces, down/carrier/operstate distinctions, driver metadata, compatible/inadequate MTU, bounded malformed input, shared trunks, profile selection, permission/platform outcomes, interface limits and gating before host inspection. CLI integration tests verify text/JSON failures without depending on real interface names. Explicit host smoke checks are separate, read-only invocations.
