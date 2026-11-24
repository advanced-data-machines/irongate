# Session handoff: HTTPS consent checkpoint

The HTTPS consent checkpoint now lives on `feature/portal-https`, branched from release commit `438a1d2`. This handoff supersedes the earlier instructions to create that branch. Check actual Git status/history before resuming; the feature is separate from `release/0.1.x` until explicitly integrated.

## Current checkpoint and next action

- HTTPS consent is implemented with rustls 0.23.46, defaults disabled and one ring provider; both daemon and CLI have verified static-musl builds.
- Explicit lab TOML selects HTTPS/8443 or the legacy HTTP/8080 fixture, with no downgrade fallback. TLS 1.3 only; bounded workers/queues, phase deadlines and service counters remain outside the packet owner.
- Each HTTPS VM generates fresh test PKI and trusts one CA through Chromium's managed `CACertificates` policy inside that guest. No host trust/network changes. Expired, wrong-name and untrusted certificates must fail in both browser and Python.
- Current ordinary checks: 139 Rust tests, 19 host lab/TLS tests, 16 comparison regressions; CRAP 279 functions, only the old CLI parser above 30 at 39.32. Anonymous Unix-socket TLS tests may require sandbox escalation to set timeouts; they bind no host listener.
- HTTPS reports use schema 4 / `local-https-v1`, with 19 required cases, queue-overflow/deadline/malformed-record evidence and three browser screenshots. The HTTP fixture remains schema 3 with 17 cases.
- Review/integrate this feature, then branch `feature/provider-boundary` from release. Do not add RADIUS directly to the TLS branch. Continue with the [HTTPS/RADIUS plan](https-radius-plan.md) and [HTTPS implementation](portal-https.md).
- Production certificate issuance/renewal, reload/expiry diagnostics, normal deployment, RADIUS, journal recovery and scale qualification remain pending.

## Repository and evidence state

Workspace: `/home/sbeskur/repos/public/irongate`. All previously completed feature branches were integrated into `release/0.1.x` through `5695c36`, then plans/handoff were committed at `4812532` / `438a1d2`. No push, tag, deployment or feature-branch deletion was performed. Check remotes before any external action.

The earlier HTTP Gate A clean run is `b9b9896b713cbfb0cc8093a5ea59961a` at `5695c36`. Initial dirty HTTPS run `bdc380b045883d2fd67d5f7d4435e4e7` passed browser/certificate/deadline cases before queue saturation was added; it is not the final accepted full-contract baseline. Locate the latest **passed schema-4, dirty-false report matching the feature runtime commit** under `target/vm/packet-lab/runs/`; never treat a newer failed/interrupted/dirty run as clean evidence. Per-run quality bundles should carry matching source identity, logs, LCOV/CRAP, dependency graph, ABI and footprint observations. Generated artifacts are ignored and not a release archive.

## What works and what does not yet

Gate A passed in a fixed one-client, untagged QEMU lab: real BusyBox DHCP, userspace ARP/IPv4 forwarding, native browser/API consent, actual upstream access, logout/expiry, generation-safe one-use actions, bounded synced local records, spoof rejection and persistent IPv4/IPv6 kernel-bypass guards. Normal deployment is still a scaffold. This is not a capacity or production readiness claim.

Userspace owns per-session forwarding decisions; nftables is the independent persistent kernel-transit guard. Native portal HTTP runs outside packet processing through bounded worker queues and a typed bridge to the single admission owner. Source is split into readable modules under `crates/irongated/src/lab/portal/`.

The lab has a 300-second lease, 30-second public consent grant, 60-second action, two HTTP workers, accepted queue four, owner queue eight, 8192-byte requests, 32 headers and 128-byte bodies. Private root-only fixture consent still grants five seconds; do not confuse it with public portal admission.

The local journal retains 16 rows and reserves the final termination cell before granting; acknowledgements do not free retained capacity. Full/unavailable storage rejects new public grants while logout/expiry still revoke access. Rows are synced on the guest disk under `/var/lib/irongate-lab/local-records.jsonl`. Only the acknowledged prefix is promised: pending rows/crash termination can be lost. Recovery, rotation, durable billing and RADIUS export are not implemented. Current byte snapshots are daemon-wide submitted IP octets, not per-session directional accounting.

## Earlier Gate A evidence and checks

Canonical clean live run: `b9b9896b713cbfb0cc8093a5ea59961a`, source commit `5695c36`, dirty false, all 17 scenarios passed.

Local files under `target/vm/packet-lab/runs/b9b9896b713cbfb0cc8093a5ea59961a/`:

- `report.json`: source/binary/image identity, scenarios, namespace/guard evidence and journal rows.
- `portal-before.png`, `portal-connected.png`, `portal-logged-out.png`: actual guest Chromium captures.
- `evidence.log`: dedicated second serial channel for machine results and bounded screenshot chunks.
- `serial.log`: boot/shutdown console output.
- `quality/`: check and Python logs, CRAP JSON/Markdown, LCOV and identity/hash metadata.

These files are ignored local build artifacts, not a durable release archive. If missing after cleanup or another checkout, rerun the fixture; do not fabricate evidence. Release archival automation remains pending.

Verification at the integrated release checkpoint: formatting/Clippy pass, 135 Rust tests, 17 host-only lab/portal tests and 16 comparison regressions pass. CRAP analyzes 251 functions; only the existing CLI parser exceeds 30 (39.32). HTTP worker startup still lacks ordinary unit-test LCOV coverage (score 30); guest functional tests do not count as merged LCOV coverage.

Important harness lessons: shared console/cloud-init buffering corrupted screenshot chunks during shutdown; redirecting directly to the original console also failed. The verified fix uses a separately configured second UART (`/dev/ttyS1`) for machine evidence, drained before shutdown. Keep strict identity/digest/completeness checks. Failed runs remain retained. Browser navigation waits for a fresh document, and the stale-generation scenario uses an explicit release from a live BusyBox DHCP client. The portal uses `Referrer-Policy: same-origin` to preserve strict form Origin checks; do not accept opaque/null origins to work around browser failures.

## HTTPS-specific lessons

Shared-console corruption is avoided by the dedicated second serial channel and drain described above. Do not revert to the console/cloud-init tee for machine evidence. One parallel HTTPS host run was interrupted and retained as failed; qualify final VM runs sequentially.

The guest OpenSSL rejects a negative `-days` certificate lifetime; expired fixtures use explicit past not-before/not-after dates. Guest Chromium did not accept the NSS trust fixture; the working setup uses its documented managed CA policy, never certificate-error bypass flags. Public test-only DER credentials under Rust test fixtures are intentionally committed; live guest keys are generated separately and never exported. Guest disk overlays contain test secrets and must not be included in public release bundles.

## Linux/libc decision to carry forward

The tested daemon is static musl and has no host glibc dependency. The default GNU development CLI references GLIBC through 2.34. This feature also builds **both** tools as static musl and checks for absent dynamic interpreters/shared dependencies. Preserve that route for releases. Rust 1.100 is the build minimum, not an installed-runtime requirement for prebuilt binaries.

Only x86-64 Debian 13 with guest kernel `6.12.117+deb13-cloud-amd64` has current live evidence. Proposed initial qualification matrix: Debian 12/13 and Ubuntu 24.04, x86-64, Linux 6.1+. These are targets to test, not approved universal support claims. Older kernels, RHEL-family systems and ARM need separate qualification. Binary ABI compatibility does not prove kernel/nftables/tooling/NIC compatibility.

## Working constraints and commands

Keep the developer PC safe: all live networking, trust changes, backend fixtures and fault injection stay inside owned disposable QEMU guests. No host NIC/firewall/routes/sysctls/trust-store changes, host LAN bridge, directory sharing or modification of the learning VM. Browser sandbox disabling is limited to the disposable guest. Ordinary doctor remains passive. Do not push or send messages externally without authorization.

Prefer small focused Rust modules and short entry points. Keep packet/session storage caller-owned and allocator-independent; bounded HTTP/TLS/record workers may allocate outside that path. Use few proven crates with explicit features; no framework or rkyv addition without a concrete reviewed need. Performance, stability and diagnosable accounting are priorities. Preserve evidence of unknown/pending states and avoid capacity/billing claims without measurements.

Useful commands:

```sh
just check
just vm-lab-check
just comparison-check
just crap
just vm-portal-prepare  # Reuses verified immutable guest browser image if present
just vm-portal-test     # Explicit HTTP consent regression
just vm-https-prepare  # Separate immutable guest certificate-tool image
just vm-https-test     # Verified HTTPS, TLS failures/pressure, prior packet cases
```

Prepared images/manifests live under `target/vm/packet-lab/`; preparation alone uses temporary QEMU userspace NAT for guest package downloads. Final tests use `-nic none`. Sandbox restrictions may require explicit escalation for Git writes or KVM; use the normal approval mechanism rather than changing host permissions. No fresh VM run is needed solely for this handoff document.
