# MVP burndown checkpoint

Status follows implemented acceptance evidence, not a percentage of checklist lines. There is no tracked, weighted effort history sufficient for a reliable percentage or delivery date. The normal deployment mode is still a scaffold; Gate A has passed in the fixed one-client isolated lab; gates B/C remain pending.

| Increment | Current status | Remaining to advance |
| --- | --- | --- |
| M0 Demo contract | Engineering lab target frozen as `mvp-lab-v1`; topology, units, limits and evidence boundaries recorded | Validate the targets as relevant subsystems become runnable; record any revised profile explicitly |
| M1 Lease/session core | Implemented, tested and integrated in `release/0.1.x` | Broader fuzz/interoperability and production recovery remain qualification work |
| M2 Linux packet lab | One-client functional path proven; expanded spoof/IPv6 evidence and startup refactor on `feature/lab-acceptance` | Load/capacity measurements, broader failure/fuzz coverage and production startup/recovery |
| M3 Local consent portal | Gate A demonstrated in QEMU: native browser/API consent, actual transit, logout/expiry and bounded synced records | Normal deployment wiring, recovery/rotation and broader qualification remain pending |
| M4 RADIUS | Pending | HTTPS login, validated responses, supported policy, accounting and separate backend health; Gate B |
| M4b External/cloud portal | Configuration contract exists; runtime pending | Secure handoff/callback, garden and tenant/device/network isolation |
| M5 MVP qualification | Pending | Two real network/VLAN profiles, repeatable mixed-provider scenarios, declared load/churn/soak and outage evidence; Gate C |

`release/0.1.x` integrated the session and initial packet work at `9c28a31`. Acceptance changes are a separate reviewable feature; they do not imply the load targets or portal/RADIUS gates passed. The [versioned contract](lab-acceptance.md) distinguishes current one-client functional evidence from the 64-session/two-network qualification target.

The local-consent feature builds on the acceptance checkpoint and has **135 passing Rust tests**, **16 comparison-framework regressions**, **17 host-only lab/portal regressions** and **17 live VM scenarios**. The coverage-backed CRAP report has **251 analyzed functions**; only CLI parsing remains above 30 at **39.32**. HTTP worker startup remains uncovered in ordinary unit-test LCOV (score 30), despite independent live browser evidence.

See the [local consent implementation](local-consent-portal.md) for bounds, action security, record health and acknowledged-prefix crash semantics. The initial successful development walkthrough is `67dffaba03ee6841751fe1591106d181`; it records a dirty working tree and must not be attributed to the parent commit as released code. Fresh clean-commit reports are retained under their own run IDs.

Next: add HTTPS and the narrow RADIUS provider through this same admission owner, with independent authentication/accounting health and outage evidence. Production startup, journal recovery/rotation, normal TOML runtime wiring, two-VLAN/external hosting and load/accounting qualification remain explicit work. Keep CLI parser cleanup focused. Raw reports remain local development artifacts until release archival automation exists.
