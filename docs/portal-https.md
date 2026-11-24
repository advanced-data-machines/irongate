# HTTPS consent checkpoint

This feature adds HTTPS to the existing standalone consent lab. It preserves the admission owner, userspace forwarding and persistent nftables guard. Password entry, RADIUS, automatic discovery/redirects, certificate renewal and normal deployment are later increments. The browser opens an explicit portal URL.

## Run the isolated test

```sh
just vm-https-prepare  # Separate immutable guest image; OpenSSL and browser tools
just vm-https-test     # Fresh overlay, no external NICs, verified HTTPS browser
```

Preparation alone uses temporary QEMU userspace NAT to install guest packages. Final runs use `-nic none`, retain the separate console/evidence serial channels and share no host directories. The workstation trust store, routes, firewall, NIC settings and learning VM are unchanged. Existing `vm-lab-test` and `vm-portal-test` remain explicitly cleartext consent fixtures.

Every HTTPS run generates a fresh two-day test CA and one-day portal certificate inside the guest. A managed Chromium `CACertificates` policy trusts that single CA in the disposable guest; Python uses the same CA explicitly. The separate wrong-name, expired and untrusted certificates must fail verification. No certificate-error bypass flags or DevTools verification overrides are used. The private keys stay inside the guest overlay and are not exported into reports or screenshots. Guest disks contain test keys and must not be published as release evidence.

The test uses Chromium's [managed CA policy](https://chromeenterprise.google/policies/#CACertificates), rather than depending on the browser's changing NSS database selection. This is lab provisioning; a production public portal would use its configured publicly trusted certificate, or an organization's explicitly managed trust policy.

## Explicit configuration

The guarded lab daemon requires `/run/irongate-packet-lab/portal.toml` before startup. The HTTPS fixture writes:

```toml
mode = "https"
origin = "https://10.50.10.1:8443"
certificate_der = "/run/irongate-packet-lab/tls/server.der"
private_key_der = "/run/irongate-packet-lab/tls/server.key.der"
```

The legacy fixture explicitly selects `mode = "http-lab"` and `origin = "http://10.50.10.1:8080"`, with no key/certificate fields. HTTPS mode binds only 8443 and does not serve HTTP as a fallback. Configuration rejects unknown keys, mode/origin mismatch, userinfo, paths/queries, other addresses and unexpected ports. This intentionally narrow lab configuration is not the general gateway TOML schema or a configurable production listener.

Credentials are DER, with an unencrypted PKCS#8 private key; PEM/chain loading is not implemented in this checkpoint. Config input is capped at 4096 bytes and each credential at 16 KiB. Inputs must be absolute, regular files; final-component symlinks are rejected and nonblocking open avoids hanging on a FIFO. Private keys reject group/other permissions. Invalid/mismatched credentials fail startup without downgrade. Existing journal startup protection still applies. Certificate expiry/name validation is demonstrated by clients; the daemon does not yet expose parsed expiry or renewal health through `ig doctor`.

## Transport and resource boundaries

| Control | Current lab choice |
| --- | --- |
| TLS | TLS 1.3 only, one explicitly selected ring provider |
| Application protocol | HTTP/1.1; existing strict framing and exact configured Host/Origin |
| Resumption / early data | Session storage, tickets and early data disabled |
| Worker / accepted / owner queues | 2 workers / 4 accepted connections / 8 owner requests |
| TLS acceptance deadline | Two-second eligibility/handshake budget from accept; an expired queued arrival closes when dequeued, without starting a fresh budget |
| HTTP phase | Separate two-second read budget after TLS, two-second owner reply wait, two-second response I/O budget |
| Individual socket operations | Remaining phase budget, capped at 200 ms |
| TLS application-write buffering | Explicit 16 KiB bound; not a total connection/handshake/process-memory bound |
| Decrypted request limits | 8192 bytes, 32 headers, 128-byte body |

The acceptor never handshakes or writes a cleartext busy response on the TLS port. Full accepted queues close connections and increment a rejection counter. Queue closure is performed by workers when they become available, not by a separate expiry scheduler. Workers enforce phase deadlines on underlying encrypted I/O, including partial TLS records; a per-read timeout alone would allow a trickling peer to occupy a worker indefinitely. All TLS parsing, encryption, credential loading and allocation stay outside the single packet/session owner.

`GET /status` includes a worker observation snapshot under `transport`: scheme, accepted connections, successful handshakes, handshake failures/timeouts and queue rejections. These counters diagnose service pressure separately from authorization and record persistence. They are not a complete certificate-health report or a substitute for an independent client test. Certificate failures on the client can appear as failed handshakes; counters alone do not identify the exact cause.

The nftables native-service exception follows the selected portal port and fixed subscriber IP/MAC. Only the HTTPS failure fixture adds guest ports 8444–8446 for deliberately invalid certificates. Userspace still authorizes each transit packet; TLS does not change kernel transit policy or install per-login nftables rules.

## Source and dependency review

Small modules under `crates/irongated/src/lab/portal/` separate configuration, credential loading, deadlines/transport, worker scheduling, service dispatch and metrics. The same dispatch/HTTP boundary is exercised using anonymous Unix sockets, with no host listener or trust change. A Codex sandbox may require approved execution outside the sandbox to set socket timeouts for those tests; this is not a root requirement. do not remove deadline checks to accommodate a sandbox.

The selected library is `rustls = 0.23.46`, defaults disabled, with only `std` and `ring`. TLS 1.2, aws-lc, compression, logging and framework/runtime features are not enabled. `http` and parse-only `toml` are reused from the workspace without enabling Serde deserialization. `rustix` additionally enables safe filesystem flags. No custom cryptography, certificate-generation Rust crate, async runtime or HTTP framework is introduced.

The Linux TLS runtime graph adds rustls, ring, rustls-pki-types, rustls-webpki, once_cell, cfg-if, getrandom, libc, untrusted, subtle and zeroize. Ring's build uses cc/find-msvc-tools/shlex; lockfile entries for Windows/WASI are not active Linux runtime packages. `libc` here supplies target bindings and does not imply a dynamically loaded glibc dependency. Ring contains native code; IronGate retains `forbid(unsafe_code)` for its own source, not a claim that dependencies contain no unsafe/native code.

Both built-in providers were considered: aws-lc is rustls's default and supports broader compliance/algorithm needs; ring is an established narrower option fitting the current x86-64 musl toolchain. This checkpoint selects ring without a FIPS requirement and verifies the actual build. Revisit provider maintenance, target support and compliance needs before extending the deployment matrix. See [rustls features/provider documentation](https://docs.rs/rustls/0.23.46/rustls/). This review is not an independent security audit.

Package metadata reports rustls as Apache-2.0 OR ISC OR MIT and ring as Apache-2.0 AND ISC. Preserve relevant dependency notices when packaging releases. The selected versions declare lower Rust minima than workspace Rust 1.100. Static crypto dependencies need patched rebuilds; shipping static binaries does not remove maintenance.

With thin-LTO musl release builds, the pre-TLS daemon at `5695c36` measured 907064 raw / 734024 stripped bytes; the initial TLS build measured 2404712 raw / 2028424 stripped bytes. These are executable sizes from one build, not runtime RAM or throughput measurements. Fresh evidence should retain its own exact sizes/hashes. `ig` and `irongated` pass `readelf` checks with no dynamic interpreter or shared-library dependencies. Broad Linux/architecture support still follows the proposed qualification matrix, not an assumption based on compilation.

## Acceptance evidence and limits

The HTTPS suite adds two cases to all 17 existing browser/packet cases: verified rejection of expired, wrong-name and untrusted certificates with no HTTP fallback; and bounded incomplete-handshake/queue pressure plus malformed TLS records, owner progress, actual forwarding/logout and memory observations. Host validation requires the complete schema-4 `local-https-v1` contract, valid scoped journal evidence and all three screenshot hashes. Python supplies an independent certificate-validation observation alongside Chromium.

Memory evidence records daemon RSS/high-water values around pressure; it is one-client functional observation, not a per-connection allocation proof, leak test, soak or capacity qualification. The 64-session/two-network throughput targets remain unmeasured. The local journal retains its 16-row capacity and acknowledged-prefix crash limitations.

After this checkpoint: review/integrate it, then implement the typed provider/configuration boundary before RADIUS login. Gate B still requires authentication, supported policy, accounting and runtime diagnostics. Production certificate provisioning/renewal, safe reload and expiry alerts remain explicit follow-up work.
