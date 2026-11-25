# Session handoff: RADIUS accounting slice

Current branch: `release/0.1.x`. Accounting runtime `880a27d` and its evidence documentation `f3a0956` were integrated by fast-forward; a documentation-only commit records that integration. Authentication remains integrated/tagged at `84abe82` / `checkpoint/radius-login`. Check Git status/history before resuming. Do not push, tag a new release, delete branches or change workstation networking without authorization.

A two-page lab Quick Start Guide is saved as `docs/irongate-quick-start.pdf`, with editable HTML/CSS alongside it. Rebuild with `just qsg` using installed headless Chrome/Chromium. It covers the full preparation chain, automated accounting run, expected results and troubleshooting.

A separate operator first draft is `docs/irongate-deployment-quick-start.pdf` (editable HTML, shared QSG CSS; rebuild with `just deployment-qsg`). It describes proposed release installation, site configuration, apply, subscriber verification and rollback. Unimplemented commands/package/service are explicitly labeled; it is not executable deployment guidance or added alpha scope.

## Current checkpoint and next action

- The second [lab-alpha increment](lab-alpha.md) is implemented: real RADIUS Start/Interim/Stop, boot-unique lifecycle IDs, per-session directional submitted IP octets, four fixed lifecycle slots with reserved Stop capacity, one dedicated bounded worker and observable outage/backlog/recovery.
- See [accounting implementation and limits](radius-accounting.md) first. Outbox storage is volatile; crash replay, shutdown draining and durable billing are not promised. Passwords/shared secrets never enter owner messages; authenticated bounded User-Name is now retained for accounting. Public status/test exports omit usernames.
- Ordinary checks: **161 Rust tests**, **22 host lab/evidence tests**, **16 comparison regressions**. CRAP analyzes **342 functions**; only the existing CLI parser exceeds 30 (39.32). Startup/serving coverage remains incomplete; VM execution is not merged into LCOV.
- Development accounting run `122c603d251af060f6bb05cce7532238` passed all **18 schema-6 cases**. Input 65,792/output 314,368 IP octets matched the independent workload and FreeRADIUS detail sink. Logout during the sink outage took about 3 ms in that run; this is functional evidence, not a latency guarantee. The run is dirty and predates final status/module changes; retain clean-commit reruns separately.
- Use `just vm-accounting-test` (reuse the `vm-radius-prepare` image). The fixture enables explicit accounting TOML; original `just vm-radius-test` keeps its authentication-only 20-case contract. Run VMs sequentially.
- Runtime source is committed at `880a27d`. Clean accounting run `b6be6a364080963b722ead5252d60630` passed 18 cases; independent clean login regression `56a0366f5358d5f20e178289e2775130` passed 20. Source/binary/lockfile and quality-manifest hashes match. The accounting run retains the complete `quality/` bundle and three verified PNGs. The feature is merged into `release/0.1.x`; no new tag or push was performed. No new Cargo dependencies or core/proto allocator dependencies were added.
- Outage policy: preserve existing authenticated sessions until their existing limits/expiry, logout or lease loss. Backend failure does not trigger automatic logout. Accounting failure pauses new admissions and retries retained obligations; requested logout never waits for a backend acknowledgment.
- Next session, finish **operational/configuration/diagnostics and repeatable demonstration integration**, the third and final alpha increment. Then stop alpha feature expansion. Two VLANs/cloud, normal deployment, production recovery/transport/certificates and scale/appliance qualification remain separate later gates.
- Keep every live backend, PKI, firewall, routing and NIC change inside the disposable QEMU guest. No workstation network/trust changes. The user plans a dedicated appliance later, starting on an isolated test LAN.

## Repository and evidence state

Workspace: `/home/sbeskur/repos/public/irongate`. Earlier completed branches were integrated into release through `5695c36`; plans/handoff at `4812532` / `438a1d2`; HTTPS integrated at `d119adf`. Do not push or send external messages without authorization.

Canonical earlier clean HTTPS run: `33a7664f01b53dabe55b6900a855d8bd`, source `d119adf`, 19 cases. Earlier HTTP Gate A: `b9b9896b713cbfb0cc8093a5ea59961a`, source `5695c36`, 17 cases. Reports, screenshots and quality bundles are ignored local artifacts, not a release archive. Guest disk overlays contain test keys/secrets; never publish those disks/private configs as public release evidence. Final bundles should retain matching commit/binary identity, logs, LCOV/CRAP, dependency graph and ABI/footprint observations.

## RADIUS-specific lessons

Core adds `begin_radius`, `radius_accept` and `radius_deny`, a separate pending-auth phase, source provenance and typed cancellation/expiry reasons. Credentials stay in bounded zeroized worker storage; only opaque attempts and typed decisions cross to the owner. Root fixture consent is still used by shared packet-pressure tests; public consent fallback is forbidden in RADIUS mode.

The guest FreeRADIUS policy needs multiline module/authenticate blocks. Capture startup diagnostics with `-l stdout`, without debug `-X`, and redact the generated secret and test password before retaining logs. The backend uses one private config and registered NAS `192.0.2.1`; a separate explicitly labeled Python server replaces it only during wire-fault injection, then the real backend restarts for recovery evidence. No backend runs on the workstation.

The core/protocol libraries remain dependency-free and allocator-independent. The Linux daemon adds pinned RustCrypto MD5/HMAC and form_urlencoded; reuse existing zeroize/subtle. This is an isolated legacy UDP profile, not a production transport endorsement. Review protected backend transport before appliance deployment. Secrets are fresh per guest; `lab-user` / `lab-pass` are public test-only credentials.

## What works and what does not yet

Gate A passed in a fixed one-client, untagged QEMU lab: real BusyBox DHCP, userspace ARP/IPv4 forwarding, native browser/API consent, actual upstream access, logout/expiry, generation-safe one-use actions, bounded synced local records, spoof rejection and persistent IPv4/IPv6 kernel-bypass guards. Normal deployment is still a scaffold. This is not a capacity or production readiness claim.

Userspace owns per-session forwarding decisions; nftables is the independent persistent kernel-transit guard. Native portal HTTP runs outside packet processing through bounded worker queues and a typed bridge to the single admission owner. Source is split into readable modules under `crates/irongated/src/lab/portal/`.

The lab has a 300-second lease, 30-second public consent grant, 60-second action, two HTTP workers, accepted queue four, owner queue eight, 8192-byte requests, 32 headers and 128-byte bodies. Private root-only fixture consent still grants five seconds; do not confuse it with public portal admission.

The local journal retains 16 rows and reserves the final termination cell before granting; acknowledgements do not free retained capacity. Full/unavailable storage rejects new public grants while logout/expiry still revoke access. Rows are synced on the guest disk under `/var/lib/irongate-lab/local-records.jsonl`. Only the acknowledged prefix is promised: pending rows/crash termination can be lost. Recovery, rotation and durable billing are not implemented. Local audit byte snapshots remain daemon-wide submitted IP octets; the new separate accounting path uses per-session directional totals.

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
