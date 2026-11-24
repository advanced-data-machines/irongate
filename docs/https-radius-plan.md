# HTTPS and RADIUS: Gate B implementation plan

Gate A is integrated into `release/0.1.x` through `5695c36`: one-client standalone browser consent, actual userspace forwarding, revocation and bounded local records. The clean functional run is `b9b9896b713cbfb0cc8093a5ea59961a`, with 17 passing guest scenarios and three validated browser screenshots. It is local development evidence, not a production deployment or release archive.

Gate B will demonstrate HTTPS login against an external RADIUS service, enforced policy and independently acknowledged accounting. Keep standalone consent runnable throughout. Integrate each completed checkpoint into `release/0.1.x` before branching the next; avoid stacking unfinished features. These are proposed branches and limits, not implemented capabilities or delivery estimates.

The first HTTPS consent checkpoint is implemented on `feature/portal-https`; see its [implementation, dependency review and limits](portal-https.md). Review/integrate that feature before starting the provider/configuration boundary. No RADIUS implementation is included in it.

## Checkpoints in dependency order

| Branch | Deliverable | Completion evidence |
| --- | --- | --- |
| `feature/portal-https` | TLS/dependency review; HTTPS consent using the current owner and fixed workers; explicit portal origin and certificate/key configuration | Real browser trusts only the guest test CA; consent/logout/expiry and token rejection still work; bad/expired/untrusted certificates fail; stalled handshakes cannot stop packet/control progress |
| `feature/provider-boundary` | Typed asynchronous authentication attempts/results; explicit native RADIUS login configuration; bounded request/credential ownership | Generation/deadline-safe completion; logout/lease reuse cancels pending attempts; queue-full and late results cannot grant access; existing external-UAM examples retain their meaning |
| `feature/radius-auth` | One named FreeRADIUS backend in the guest; PAP with dedicated lab credentials; strict bounded protocol validation and supported policy | Valid Accept grants actual traffic; Reject, timeout, Challenge, invalid authentication and unsupported required policy leave access denied; healthy validated rejection is distinguishable from backend failure |
| `feature/radius-accounting` | Stable session identity, directional counters, Start/Interim/Stop obligations, bounded retry and admission capacity reservation | Independent backend records match sender/receiver evidence; retransmissions do not create new local obligations; delayed/unavailable accounting is observable; logout is immediate even when export fails |
| `feature/radius-diagnostics` | Read-only scoped runtime observations exposed to the operator CLI; separate authentication/accounting health | `ig doctor` and a narrow radius-status command produce matching text/JSON findings without generating logins or accounting traffic; split failures and recovery have explicit evidence |

HTTPS consent is a complete first checkpoint; it does not need credential handling or RADIUS. Authentication alone does not pass Gate B. Normal deployment, journal recovery, external/cloud handoff and two-VLAN/load qualification remain separate tracked work.

## First checkpoint: HTTPS consent

1. Review `rustls` as the leading candidate and choose one explicit crypto provider. Record the pinned versions, active/transitive features, licenses, build tools, static-musl compatibility, allocation behavior and binary-size change. Compare provider choices before selecting one; do not add two providers, an async runtime or an HTTP framework without a demonstrated need.
2. Keep TLS in the existing bounded service workers. Extract transport handling from HTTP request handling only where needed; reuse framing, scoped actions and the owner bridge. Keep entry points short and modules focused on transport, configuration, credentials, protocol, policy, counters and reporting.
3. Define a canonical HTTPS origin and guest-only listener, initially port 8443. Generalize strict Host/Origin validation from explicit configuration rather than trusting request headers. Preserve consent-only HTTP fixtures under an explicit lab mode; credentials must never be accepted on HTTP or through a downgrade fallback.
4. Generate a short-lived test CA and server certificate inside the guest preparation environment. Put private keys in guest-private storage. Trust the CA only in the disposable client/browser profile; never modify workstation trust. Use a certificate with the correct SAN for the selected portal address/name. Do not disable browser verification or use certificate-error bypass flags.
5. Retain two workers, four queued accepted connections and eight owner requests initially. Propose a two-second total TLS handshake deadline and a separate two-second HTTP read deadline. Apply one total handshake deadline across retries, not just per-read timeouts. Enforce request/body bounds after decryption.
6. Configure TLS write buffering explicitly, measure per-connection/handshake memory and test oversized or incomplete handshake input. A TLS library's output-buffer setting is not a total process-memory limit. Disable early data and begin with HTTP/1.1 only. Keep entropy, certificate loading, TLS and allocations outside the packet owner.
7. Extend the NIC-free VM fixture and certificate-negative tests, preserve all existing packet/crash-guard cases, capture HTTPS browser screenshots and retain source/binary/image/dependency identities. Guest preparation may download packages using QEMU userspace NAT; final scenario boots still have no external NICs.

The current rustls documentation describes explicit crypto-provider features and the scope of `set_buffer_limit`; use that as input to the dependency review, not a blanket memory guarantee: [crate features](https://docs.rs/rustls/latest/rustls/) and [connection buffering](https://docs.rs/rustls/latest/rustls/server/struct.ServerConnection.html). Freeze the exact reviewed versions when implementing.

Exit: HTTPS standalone consent works with normal certificate verification, bounded handshake pressure is diagnosable, static artifacts pass compatibility inspection, and fresh clean-commit scenario/quality evidence is retained. Password entry follows in the provider/authentication checkpoints.

## Provider and authentication boundary

The core currently exposes consent admission, not a general external authentication transaction. Introduce the smallest typed provider attempt/decision extension needed; do not disguise a RADIUS result as local consent. Carry network, boot epoch, session/lease generation, attempt identifier and deadline through the worker result. Only the admission owner validates the current attempt and commits policy. An HTTP timeout or abandoned browser request must have a defined cancellation/result policy; it must not create a grant for a reused session.

Keep credentials in a bounded worker-owned payload, never in URLs, logs, screenshots, session records or generic status output. Select a vetted mechanism for clearing sensitive buffers where applicable and review any dependency explicitly. Session/packet storage stays allocator-independent. Initial proposed limits are eight outstanding backend requests, username at most 64 bytes, password at most 128 bytes and a five-second end-to-end authentication budget including retries. Bound UTF-8/form decoding, queued work and response attributes; document supported encoding. These are limits to validate before freezing the fixture, not implicit protocol-wide restrictions.

Add an explicit native-login provider mode and HTTPS settings to TOML. Existing RADIUS examples use external UAM: preserve that contract and reject unsupported combinations. Render an effective, redacted configuration showing placement, provider, endpoint, NAS identity, secret references, limits and failure policy. Keep local/external hosting independent of consent/RADIUS selection.

Pin one patched FreeRADIUS package/version and reproducible configuration in a new immutable guest image. Register exactly the gateway's observed source address and NAS identity. Initial PAP/UDP interoperability is restricted to this isolated VM; production protected RADIUS transport remains a separate qualification requirement. Review RFC 2865/2866/2869/5080 and the pinned backend's current security guidance before finalizing packet/authenticator rules.

Use vetted crypto primitives and OS randomness. Validate packet/attribute lengths, source endpoint, code, identifier, request context, Response-Authenticator and the agreed Message-Authenticator policy before accepting a result or updating healthy-response evidence. Test missing/invalid authenticators, spoofed responses, duplicates, late replies and identifier reuse. Follow request-type-specific retransmission/authenticator rules; do not assume Access and Accounting packets use identical rules. See [FreeRADIUS client security settings](https://github.com/FreeRADIUS/freeradius-server/blob/master/raddb/clients.conf). Pin a backend revision rather than relying on moving master defaults.

Start with Accept/Reject and a documented attribute allowlist. Session-Timeout caps access; Idle-Timeout requires a real activity clock and expiry evidence before being advertised; Acct-Interim-Interval needs explicit bounds and accounting scheduling. Unsupported Challenge returns a clear unsupported result without granting access. Required policy that cannot be enforced must reject admission rather than silently granting unrestricted access. Do not claim support for shaping/vendor policy, failover, EAP, CoA or arbitrary attributes in this increment.

## Accounting and failure policy

Before admission, reserve bounded lifecycle capacity for Start and eventual Stop plus the required per-session counter/interval state. Use a stable external Acct-Session-Id derived from a boot-unique namespace and network/session generation. It must not collide across restart or address reuse. Preserve critical obligations; only replaceable interim snapshots may be coalesced under an explicit policy. The current 16-cell consent trail is not a RADIUS outbox or restart/replay implementation.

Count successful submitted IP octets separately per session and direction, using 64-bit internal counters. NAS input means subscriber-to-upstream; NAS output means upstream-to-subscriber. Encode octets/gigawords correctly and define packet counts, garden/dropped traffic and retransmissions. A transmit submission is not proof of wire delivery. Capture independent traffic evidence, including a paced 64 kbit/s client alongside bulk traffic; retain uncertainty rather than claiming equivalent counters at different stages.

| Condition | New RADIUS admission | Existing authorization and accounting |
| --- | --- | --- |
| Valid Access-Accept, capacity available | Apply supported policy after reserving obligations; do not wait in the packet owner for disk/backend I/O | Export Start and retain it until a validated response; distinguish local retention from remote acknowledgement |
| Valid Reject or unsupported Challenge/policy | Deny with the specific outcome | No unrelated session or backend-health failure is inferred |
| Authentication timeout/invalid reply | Deny; no implicit local fallback | Existing grants retain their limits until normal logout/expiry; invalid replies do not refresh healthy evidence |
| Accounting known unavailable or lifecycle capacity exhausted | Pause new accounting-dependent grants | Revoke immediately on logout/expiry and retain reserved Stop; report backlog and oldest age |
| Accounting initially unobserved | Permit only within the declared bounded initial-observation/outage budget | Show unknown health; freeze the transition/deadline policy and test it before enabling this profile |
| Backend recovers | Resume only after validated evidence and available capacity | Drain at a bounded rate using stable event identity and cumulative snapshots |

Freeze queue/outage budgets and retry rules in a versioned lab profile before implementing the exporter. Expose scheduled-send lag, last validated response, pending count, oldest obligation, retries, invalid responses, exported/acknowledged snapshots and termination reason. Acknowledgement means a validated Accounting-Response, not merely a send or a synchronized local row. Backend acknowledgement does not prove a billing dashboard applied the intended interpretation. Do not promise exactly-once backend processing or crash replay without independent implementation and evidence.

## Diagnostics and Gate B acceptance

Runtime diagnostics should use a bounded, read-only local control surface with explicit schema and permissions. Extend the existing CLI without mixing passive observation with active probes. In standalone mode RADIUS is disabled, not unhealthy. Distinguish endpoint selection, source/NAS identity, valid Reject, timeout, invalid authentication, unavailable accounting and insufficient evidence. Include scope, age, units, limits and concrete next checks in text/JSON. Avoid retaining usernames or passwords in a general doctor report.

Gate B requires one complete fresh-client browser login, real before/after forwarding checks, supported policy expiry, logout, matching Start/Interim/Stop identity/counters and validated responses. Repeat with wrong credentials, wrong secret/NAS registration, invalid/late replies, backend delay/loss, authentication-working/accounting-failing, capacity pressure and recovery. Existing flows must remain isolated from new attempts and traffic must stop promptly on logout even during backend outage. A passing HTML login or authentication-only test is insufficient.

For every checkpoint run formatting/lint/meaningful tests, review fresh CRAP and dependency changes, and retain clean-commit VM evidence using the dedicated serial evidence channel. No new hot-path allocation, capacity, throughput or production-support claim follows without measuring the relevant path. CI/release archival automation is still pending.

## Linux and libc compatibility

Treat binary ABI, kernel features, CPU architecture and distro tooling as separate qualification dimensions. The tested `x86_64-unknown-linux-musl` daemon is a static PIE with no host dynamic loader/glibc dependency. The current default GNU development CLI is dynamically linked and references GLIBC versions through 2.34; it is not a portable release artifact. Rust's documented GNU target baseline does not constrain the symbols a particular build actually links: [Rust platform support](https://doc.rust-lang.org/rustc/platform-support.html).

Propose shipping both `ig` and `irongated` as static musl artifacts first. Inspect each release with `file`/`readelf` to reject unexpected dynamic interpreters, shared-library dependencies or host-specific CPU flags. If GNU artifacts are later needed, build against the oldest explicitly supported sysroot and inspect symbol requirements; building on a new workstation is not a compatibility strategy. Rust 1.100 is the build-tool requirement, not something users need installed to run a prebuilt binary. Static crypto dependencies also need security updates and rebuilds.

| Platform | Present evidence / proposed status |
| --- | --- |
| x86-64 Debian 13, guest kernel `6.12.117+deb13-cloud-amd64` | Current isolated functional qualification; normal deployment remains a scaffold |
| x86-64 Debian 12 and Ubuntu 24.04 | Proposed next portability matrix; not tested yet |
| Linux 6.1+ | Proposed initial kernel floor to qualify; not an inferred universal support guarantee |
| Older kernels, Ubuntu 22.04, RHEL-family systems | Assess separately, including vendor backports, nftables/tooling and security policy; no current support claim |
| ARM64 / embedded distributions | Later architecture/toolchain/NIC qualification; no current binary or live evidence |

Add a static CLI build/smoke check, then run the actual gateway scenarios on the oldest proposed guest kernel as well as the current image before declaring the floor supported. Compile-time musl compatibility alone cannot prove AF_PACKET, nftables guard behavior, entropy, TLS, namespace/tooling or physical NIC readiness. Keep the developer workstation unchanged during every qualification run.
