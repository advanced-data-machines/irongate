# Session handoff: resume with HTTPS consent

Saved after the November 22, 2026 session (America/Denver). The user asked to stop here and resume next session. Do not begin the next feature as part of saving this handoff.

## Repository and branch state

- Workspace: `/home/sbeskur/repos/public/irongate`.
- Current branch: `release/0.1.x`.
- Starting checkpoint for this handoff: `4812532` (`docs: plan HTTPS and RADIUS checkpoints after Gate A`). The handoff itself is a subsequent documentation commit.
- All existing completed feature branches are ancestors of release: lease, session, Linux packet lab, expanded acceptance and local consent portal. Acceptance and portal work were fast-forwarded, preserving history. Feature branches were retained.
- Latest runtime/harness commit: `5695c36`; portal implementation: `0f665e1`; preceding evidence fixes: `2ed700d` and `5695c36`.
- No push, tag, deployment, branch deletion or new HTTPS branch was performed in this session. Check actual Git status/remotes before taking further actions.

## What works and what does not yet

Gate A passed in a fixed one-client, untagged QEMU lab: real BusyBox DHCP, userspace ARP/IPv4 forwarding, native browser/API consent, actual upstream access, logout/expiry, generation-safe one-use actions, bounded synced local records, spoof rejection and persistent IPv4/IPv6 kernel-bypass guards. Normal deployment is still a scaffold. This is not a capacity or production readiness claim.

Userspace owns per-session forwarding decisions; nftables is the independent persistent kernel-transit guard. Native portal HTTP runs outside packet processing through bounded worker queues and a typed bridge to the single admission owner. Source is split into readable modules under `crates/irongated/src/lab/portal/`.

The lab has a 300-second lease, 30-second public consent grant, 60-second action, two HTTP workers, accepted queue four, owner queue eight, 8192-byte requests, 32 headers and 128-byte bodies. Private root-only fixture consent still grants five seconds; do not confuse it with public portal admission.

The local journal retains 16 rows and reserves the final termination cell before granting; acknowledgements do not free retained capacity. Full/unavailable storage rejects new public grants while logout/expiry still revoke access. Rows are synced on the guest disk under `/var/lib/irongate-lab/local-records.jsonl`. Only the acknowledged prefix is promised: pending rows/crash termination can be lost. Recovery, rotation, durable billing and RADIUS export are not implemented. Current byte snapshots are daemon-wide submitted IP octets, not per-session directional accounting.

## Evidence and checks

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

## Next session

1. Read [the detailed HTTPS/RADIUS plan](https-radius-plan.md), [burndown](burndown.md) and [current consent implementation](local-consent-portal.md). Check Git status/history and any repository instructions.
2. Create `feature/portal-https` from current `release/0.1.x` after verifying it does not already exist. No such branch has been created yet.
3. Review rustls and one explicit crypto provider: pinned versions, active/transitive features, licenses, static-musl build fit, memory and binary size. Rustls is the leading candidate, not a selected or added dependency. No new TLS dependency was added during planning.
4. Implement HTTPS **consent** first using the current workers/owner. Explicitly configure the canonical Host/Origin; preserve bounded handshakes and HTTP limits. Establish a guest-only test CA with normal browser verification and negative certificate tests. Do not add credentials or RADIUS during this checkpoint.
5. Retain the existing packet/portal suite and add TLS pressure/failure evidence; collect a fresh clean-commit run before integration. Integrate each completed checkpoint before branching the next.

Subsequent planned branches: provider/configuration boundary, RADIUS authentication, accounting, then read-only runtime diagnostics. Gate B requires the whole authenticated and accounted lifecycle, not just an Accept response. Existing external-UAM configuration must retain its meaning. No general RADIUS server or credential database is planned.

## Linux/libc decision to carry forward

The tested daemon is static musl and has no host glibc dependency. The current default GNU development CLI references GLIBC through 2.34 and is not the portable release artifact. Propose shipping **both** tools as static musl; TLS/crypto dependencies must preserve that route. Rust 1.100 is the build minimum, not an installed-runtime requirement for prebuilt binaries.

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
just vm-portal-test     # Fresh NIC-free VM, full browser/packet suite
```

Prepared images/manifests live under `target/vm/packet-lab/`; preparation alone uses temporary QEMU userspace NAT for guest package downloads. Final tests use `-nic none`. Sandbox restrictions may require explicit escalation for Git writes or KVM; use the normal approval mechanism rather than changing host permissions. No fresh VM run is needed solely for this handoff document.
