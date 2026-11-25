# RADIUS login: first lab-alpha increment

`feature/radius-login` adds one complete HTTPS login path against a real FreeRADIUS backend. It includes the small provider/configuration boundary required by that path rather than a separate plugin framework. HTTPS consent is integrated into `release/0.1.x` at `d119adf`; standalone consent remains a separate explicitly selected profile.

The first development run, `4901ffe3c3599826b331020899990c5c`, passed 20 guest scenarios using FreeRADIUS `3.2.7+dfsg-1+deb13u2`. It is dirty development evidence. Qualifying runs must independently record a clean matching runtime commit and pass the complete schema-5 `radius-login-v1` validator. Current ordinary checks pass 149 Rust tests, 20 host lab/evidence regressions and 16 comparison regressions. CRAP analyzes 309 functions; only the existing CLI parser exceeds 30 (39.32). Startup/native serving coverage remains incomplete; live execution is not merged into LCOV.

This completes the authentication slice of the [lab-alpha finish line](lab-alpha.md), not Gate B or production readiness. RADIUS accounting is now implemented separately in the [accounting lab slice](radius-accounting.md). Runtime CLI diagnostics, normal gateway deployment, protected backend transport and scale qualification remain pending.

## Run and observe

```sh
just vm-radius-prepare  # Separate guest image; FreeRADIUS is installed only there
just vm-radius-test     # Fresh NIC-free VM; browser, gateway and backend are isolated
```

Preparation alone uses temporary QEMU userspace NAT. Scenario boots have no external NIC, host share, bridge, trust-store change or workstation network mutation. The real backend runs in the upstream guest namespace at `192.0.2.80:1812`, registered for NAS source `192.0.2.1`. Its version is retained in the image manifest and report. The fixture stops/masks the guest's distribution service before starting its owned process; it does not use a backend in the management namespace.

Dedicated **test-only** credentials are `lab-user` / `lab-pass`; the shared secret is freshly generated inside each guest. Neither is a deployment credential. No credentials are passed as process arguments. Keys, secrets and browser profiles stay inside the guest overlay. Do not publish those disks or private configuration files as release evidence.

The browser uses the existing guest-only trusted CA and normal certificate verification. The three screenshots show an empty sign-in form, connected state and logged-out state. Python independently checks certificate rejection. The retained report distinguishes genuine backend interoperability from explicitly labeled synthetic wire faults.

## Explicit lab TOML

The fixture generates `/run/irongate-packet-lab/portal.toml`:

```toml
mode = "https"
origin = "https://10.50.10.1:8443"
certificate_der = "/run/irongate-packet-lab/tls/server.der"
private_key_der = "/run/irongate-packet-lab/tls/server.key.der"
provider = "radius"
radius_endpoint = "192.0.2.80:1812"
radius_secret_file = "/run/irongate-packet-lab/radius/secret"
nas_identifier = "irongate-lab"
```

RADIUS requires HTTPS. Missing/unsupported settings, an endpoint outside this fixed lab backend, invalid NAS identifiers, unused RADIUS settings in consent mode and unsafe secret files fail startup. Secrets must be regular absolute files without final-component symlinks, private to their owner, with 16–64 printable non-space ASCII bytes after trailing CR/LF removal. Reads are bounded. There is no fallback to public consent or HTTP when RADIUS fails. Private root-owned fixture controls remain available to the shared packet tests; they are not a public fallback or normal deployment API.

The unselected/default consent provider preserves the previous lab behavior. This narrow TOML is not the normal deployment schema; existing general configuration examples using external UAM retain their meaning. Native production configuration/runtime integration remains separate work.

## Admission flow and limits

1. The HTTPS client obtains a one-use action bound to its observed peer, network, lease and session generation. A login form cannot select another network/backend/session.
2. The owner consumes that action, verifies current lease and local record capacity, and begins a typed RADIUS attempt. Forwarding stays denied. Only opaque correlation leaves the owner; credentials stay on the service worker.
3. The worker sends a PAP Access-Request to the configured endpoint. OS entropy supplies the identifier/request nonce, and the native UDP socket is bound to the fixed NAS address and connected to the exact backend.
4. A reply must match the request context and pass both Response-Authenticator and Message-Authenticator checks before policy or healthy-response evidence is accepted.
5. Only the current, unexpired attempt can apply supported policy. The actual userspace policy effect must be acknowledged before the HTTP success redirect. Logout, deadline or lease/session reuse invalidates outstanding work.

A submitted valid login can complete if the browser disconnects; an explicit logout/cancel invalidates its attempt. Disconnecting an HTTP client is not a distributed cancellation guarantee. A backend reply to a cancelled or reused attempt cannot grant access.

| Boundary | Lab choice |
| --- | --- |
| Backend / method | One endpoint, PAP over isolated UDP only |
| Concurrent backend work | At most two existing service workers; no new authentication thread pool |
| Attempt lifetime | Existing core admission-attempt deadline: ten seconds |
| Backend exchange | Two seconds total, at most two identical sends, receive waits capped at 150 ms |
| Datagram / attributes | At most 4096 bytes / 64 response attributes; strict declared length |
| Credentials | Username 1–64 bytes, ASCII letters/digits/`_.@-`; password 1–128 printable ASCII bytes |
| Login form | At most 768 encoded body bytes; exact token/username/password vocabulary; duplicate/unknown fields and malformed escapes rejected |
| Positive policy | Session-Timeout, capped by local 30-second maximum and current lease; absent timeout uses that local limit; zero is unsupported |
| Records | Existing 16-cell local audit; grant capacity rechecked on completion and termination cell reserved |

Idle-Timeout, shaping, address overrides, Class, vendor policy and other unimplemented attributes are not silently applied or ignored in an Accept: the strict supported-profile allowlist rejects unsupported policy. Reply-Message is authenticated but never reflected or logged. A Challenge is reported unsupported without granting access. Additional methods, realms, transports, failover, CoA and a general RADIUS dictionary are outside this slice.

Request/reply packet buffers are fixed. Response verification hashes borrowed packet views with the original request nonce and a zeroed-MA view; it does not copy a full packet or allocate an attribute collection. The core remains `no_std`, caller-owned and dependency-free. Credentials and raw request buffers are explicitly cleared with zeroize; this is not a guarantee about every copy inside TLS, the OS or third-party primitives. Inline bounded reply/credential variants deliberately avoid owner-side boxing; the two targeted Clippy size exemptions document that choice.

## Supported outcome and health evidence

`GET /status` is passive and reports selected provider separately from effective policy source. Authentication observations include endpoint/NAS IP, attempts, Access-Accept/Reject counts, timeouts, invalid replies, transport failures, unsupported decisions, stale results, last exchange duration/send count, and last validated response time/age. No username or password appears in that report.

| Outcome | HTTP / access | Observation |
| --- | --- | --- |
| Authenticated Accept with supported policy | 303 after application / allowed until its deadline | RADIUS policy source; local grant retained |
| Authenticated Reject | 401 / denied | Backend responsive; credentials or policy rejected |
| Unsupported authenticated policy or Challenge | 422 / denied | Backend responsive; this profile cannot apply its decision |
| Invalid responses through deadline | 502 / denied | Invalid-response count; does not refresh validated health |
| No response / transport failure | 503 / denied | Timeout or transport failure; no implicit provider fallback |
| Cancelled/replaced/expired attempt | 409 / denied | Stale result; no replacement-session grant |
| Local lifecycle capacity unavailable | 503 / denied | Local resource condition, distinct from backend responsiveness |

Health describes retained exchange evidence, not an active reachability probe or a guarantee about downstream databases. A timeout alone cannot distinguish route loss, unregistered NAS or secret mismatch. The backend never sits in the packet-by-packet forwarding path. Logout/expiry do not wait for backend availability.

Local rows now record the policy provider. The test expects three grant/termination lifecycles, including one expiry, with six synced scoped rows. These remain **local audit records, not RADIUS Start/Interim/Stop accounting**. The authentication-only fixture reports accounting as `not_implemented`; the separate accounting-enabled fixture reports accounting health independently. Journal retention/recovery/crash limitations are unchanged.

## Dependency and protocol review

The slice adds pinned RustCrypto `md-5 0.10.6` and `hmac 0.12.1` for legacy RADIUS primitives, plus `form_urlencoded 1.2.2` for form decoding. It directly reuses already-active `zeroize` and `subtle`. New supporting packages are digest, block-buffer, crypto-common, generic-array, typenum, percent-encoding and build-time version_check. No async runtime, web framework, custom crypto or production RADIUS server is added. Keep the reviewed lockfile and license notices.

MD5/PAP are used solely because the isolated legacy UDP interoperability profile requires them; they are not selected for password storage or new security protocols. Public/appliance deployment requires a protected-backend-transport decision and its own tests. The normative checks derive from [RFC 2865](https://www.rfc-editor.org/rfc/rfc2865.html), [Message-Authenticator rules](https://www.rfc-editor.org/rfc/rfc3579.html) and [RFC 5080](https://www.rfc-editor.org/rfc/rfc5080.html), with the patched backend configured to require Message-Authenticator. FreeRADIUS's [client security settings](https://github.com/FreeRADIUS/freeradius-server/blob/v3.2.x/raddb/clients.conf) are part of the fixture contract, not an assumption that defaults are sufficient.

Existing Rust RADIUS components were considered. `radius-rust` exposes dictionary/packet objects and separate reply/Message-Authenticator verification helpers ([API](https://docs.rs/radius-rust/0.4.4/radius_rust/client/client/struct.Client.html)); adopting it would still require our strict bounds, verified profile and attempt/health integration. `radius-tokio` is a broader async server stack ([scope](https://docs.rs/radius-tokio/0.1.0/radius_tokio/)), which is outside the client-only task. This small codec is deliberately not a general replacement for those libraries, and this review makes no claim that they are insecure. Independent Python hashlib/HMAC vectors and real FreeRADIUS interoperability provide separate correctness oracles. The review is not a security audit.

## Evidence and next work

The 20-case suite combines eleven original packet/isolation cases, two HTTPS certificate/pressure cases and seven RADIUS cases. It checks real browser login/logout, valid rejection, enforced timeout, unsupported policy, no consent/replay fallback, wrong authenticators/identifier/source/length, missing MA, unavailable backend, unsupported challenge, a late valid reply across lease reuse, and real-backend recovery. Synthetic faults replace the real backend only during the labeled fault portion; they are not presented as real credential checks.

Every accepted run retains exact commit/dirty state, binary/image/harness hashes, backend version, scoped local audit, screenshots and dedicated serial evidence. Keep the separate standalone HTTPS suite passing. Static-musl ABI/footprint checks and fresh LCOV/CRAP belong with the clean final report; ignored `target/` is still not a release archive.

The [accounting slice](radius-accounting.md) adds bounded obligations, directional counters and visible lag/recovery. Next make the selected lab profile and diagnostics repeatable for evaluation. The [alpha checklist](lab-alpha.md) defines the stop point; appliance and full-MVP qualification follow separately.
