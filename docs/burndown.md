# MVP burndown checkpoint

Status follows implemented acceptance evidence, not a percentage of checklist lines. There is no tracked, weighted effort history sufficient for a reliable percentage or delivery date. The normal deployment mode is still a scaffold; none of the product walkthrough gates A/B/C has passed.

| Increment | Current status | Remaining to advance |
| --- | --- | --- |
| M0 Demo contract | Engineering lab target frozen as `mvp-lab-v1`; topology, units, limits and evidence boundaries recorded | Validate the targets as relevant subsystems become runnable; record any revised profile explicitly |
| M1 Lease/session core | Implemented, tested and integrated in `release/0.1.x` | Broader fuzz/interoperability and production recovery remain qualification work |
| M2 Linux packet lab | One-client functional path proven; expanded spoof/IPv6 evidence and startup refactor on `feature/lab-acceptance` | Load/capacity measurements, broader failure/fuzz coverage and production startup/recovery |
| M3 Local consent portal | Next product increment | Secure browser/API handoff, consent/logout UI, bounded local records and matching diagnostics; Gate A |
| M4 RADIUS | Pending | HTTPS login, validated responses, supported policy, accounting and separate backend health; Gate B |
| M4b External/cloud portal | Configuration contract exists; runtime pending | Secure handoff/callback, garden and tenant/device/network isolation |
| M5 MVP qualification | Pending | Two real network/VLAN profiles, repeatable mixed-provider scenarios, declared load/churn/soak and outage evidence; Gate C |

`release/0.1.x` integrated the session and initial packet work at `9c28a31`. Acceptance changes are a separate reviewable feature; they do not imply the load targets or portal/RADIUS gates passed. The [versioned contract](lab-acceptance.md) distinguishes current one-client functional evidence from the 64-session/two-network qualification target.

The acceptance feature's checkpoint has **124 passing Rust tests**, **16 comparison-framework regressions**, **14 host-only lab-contract tests**, and **11 live VM scenarios**. The coverage-backed CRAP report has **216 analyzed functions**; only CLI parsing remains above 30 at **39.32**. Guest startup/serving coverage is still incomplete even though the earlier startup threshold finding is resolved.

Next: review/integrate this acceptance checkpoint, then build M3 through the existing admission owner. Keep the CLI parser cleanup as a focused maintenance change, and retain load/accounting qualification as explicit pending work rather than blocking on unavailable measurements or claiming them complete. Raw reports remain local development artifacts until release archival automation exists.
