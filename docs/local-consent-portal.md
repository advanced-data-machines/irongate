# Local consent portal lab

The guarded Linux lab now serves a native IronGate consent page at `http://10.50.10.1:8080/`. A subscriber accepts terms to obtain 30 seconds of real userspace IPv4 forwarding. Disconnect or expiry immediately blocks both existing and new TCP traffic. This is standalone network consent, without identity verification or a RADIUS dependency.

Run `just vm-portal-prepare` once to install Chromium **inside a separate disposable guest image**, then `just vm-portal-test`. Preparation uses QEMU userspace NAT for package downloads. The test boots a fresh overlay with `-nic none`: browser, client, gateway and upstream all run inside the guest. There are no host network changes, host directory shares, debugging TCP ports or changes to the learning VM. Chromium's sandbox is disabled only inside this disposable guest. The current demo uses a documented URL; automatic discovery and redirects remain future work.

## Admission and forwarding

```mermaid
sequenceDiagram
    participant B as Subscriber browser
    participant H as Bounded HTTP workers
    participant O as Single admission owner
    participant W as Journal writer
    B->>H: GET / (native TCP)
    H->>O: Observed peer + view request
    O-->>B: Session-bound one-use action
    B->>H: POST /consent + action
    H->>O: Typed request after strict framing/origin checks
    O->>O: Validate lease/generation/deadline; reserve records
    O->>O: Apply userspace policy cell
    O-->>B: Redirect to connected page
    O->>W: Ordered grant record (bounded queue)
    W-->>O: Acknowledge after sync_data
    Note over O: Every transit packet rechecks current authorization
    B->>O: Logout through HTTP worker
    O->>O: Revoke immediately; retain reserved stop
    O->>W: Ordered termination record
```

nftables permits only the narrow native portal connection from the configured subscriber IP/MAC and blocks kernel transit persistently, including on daemon death. It does not install a per-login forwarding rule. Packet classification, lease ownership, session deadlines and forwarding decisions remain in the existing single Rust owner. The test upstream is routed directly; general Internet routing/NAT is outside this profile.

The source lives in small modules under `crates/irongated/src/lab/portal/`: HTTP framing/header policy, worker service, owner actions, tokens, page assets, bounded journal, bridge and disk writer. Entry points remain short. The packet owner never waits for HTTP or disk I/O; presentation/serialization allocates in workers, while packet processing continues to use fixed buffers and borrowed core slots.

## Bounds and rejection behavior

| Resource | Lab limit |
| --- | --- |
| Client/network | One fixed subscriber, one untagged network; DHCP lease 300 seconds |
| Access/action lifetime | 30 seconds / 60 seconds |
| Action storage | Four fixed cells; repeated GET reuses a current action |
| HTTP workers / accepted queue / owner queue | 2 / 4 / 8 |
| HTTP request / headers / body | 8192 bytes / 32 / 128 bytes |
| Request/reply deadlines | Two seconds each; socket operations have shorter timeouts |
| Local retained records / disk file | 16 / 64 KiB |
| Record worker | One ordered write in flight; retry after one second on error |

Actions use 256 bits of OS randomness and bind to the current lease, network, session generation, action kind and deadline. No client-selected network or session identifier is accepted. Wrong action, stale/replayed/expired tokens fail with 409; wrong Origin/Host fails with 403. Responses are noncacheable, actions stay out of URLs/logs, and the same-origin referrer policy preserves browser form Origin validation. Opaque/null origins remain rejected. This is a cleartext isolated consent demo; HTTPS must precede password entry.

`GET /status` is read-only JSON without an action bearer. `GET /api/action` obtains the next action; POST `/consent` or `/logout` sends `token=<hex>` with the exact portal Origin and form content type. HTTP framing rejects ambiguous lengths, transfer encoding, pipelining and oversized input. These are lab endpoints, not a frozen external/cloud protocol.

## Record health and crash semantics

Before granting, the owner reserves capacity for both a grant and its eventual termination. Disk acknowledgements advance a durable prefix but do **not** free retained memory. Eight complete normal admissions fill the 16-cell lab journal. Full/unavailable storage pauses new admissions with 503, while logout/expiry still use the reserved termination cell and revoke access immediately. There is no silent overwrite.

`/status` exposes retained, durable and pending counts, capacity, write failures, oldest pending age and a stalled flag. An unacknowledged write after two seconds marks storage unavailable without issuing a second out-of-order write. A later successful acknowledgement restores storage health. Status distinguishes policy state from record persistence.

The worker writes `/var/lib/irongate-lab/local-records.jsonl` on the guest disk, synchronizes the file and newly created directory entries, and acknowledges only successfully synced rows. Partial unacknowledged tails are truncated before retry. Access can precede a disk acknowledgement: a crash can lose pending rows or omit a termination record. Only the acknowledged prefix is promised. Startup refuses an existing journal rather than overwriting it; recovery, rotation, archival, per-session directional counters and durable billing semantics remain future work. Current byte snapshots are daemon-wide successful IP submission totals, explicitly not RADIUS accounting.

The private root-only lab control can inject clearly marked `TestPressure` rows to exercise capacity behavior. It is inaccessible through the public HTTP API. Existing private consent controls remain fixtures for packet enforcement tests.

## Evidence and next increment

Each run retains `report.json`, serial logs and three browser PNGs under `target/vm/packet-lab/runs/<run-id>/`. Screenshot export uses bounded serial chunks validated against run identity and SHA-256. The browser runs in the subscriber namespace and uses the real form path. The portal suite adds six scenarios to the eleven packet/isolation scenarios: browser consent/logout, action rejection, expiry/stale generation, access expiry, slow HTTP isolation and journal pressure with a retained durable stop.

Gate A passed all 17 live scenarios in a fresh QEMU overlay. This is a one-client functional lab. Normal deployment/TOML runtime wiring, external/cloud handoff, HTTPS, RADIUS, two-VLAN qualification and scale/load/soak measurements remain pending. Local/external portal placement stays separate from consent/RADIUS provider selection; no implicit fallback is introduced. The next provider increment should add HTTPS and a narrow RADIUS client through this same admission owner, with independent backend and accounting health.
