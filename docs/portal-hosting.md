# Portal hosting and tenant scope

Portal hosting is an explicit choice per `[[networks]]` profile: **local** means served by IronGate on the gateway device; **external** means a separate service on a LAN or in a cloud. An external service may serve one site or multiple tenants. Different VLANs may choose different placements at the same gateway.

Hosting, portal protocol and authentication provider describe different things. A local page should be able to use RADIUS; a cloud page is not itself a RADIUS server or authority to bypass policy. Current static schema combinations remain local/builtin_consent/consent/local accounting and external/external_uam/radius/radius accounting. A built-in RADIUS login mode is planned, not accepted or implemented yet. The new hosting/scope fields validate intent; neither portal runtime exists today.

## Explicit local configuration

```toml
[networks.portal]
hosting = "local"
mode = "builtin_consent"
listen = "192.168.50.1:3990"
terms_file = "/etc/irongate/terms.html"

[networks.authentication]
provider = "consent"
```

This is the page served on this device, with local assets and no cloud frontend requirement. Authentication and accounting selections remain separate tables. See the complete [standalone profile](../examples/standalone.toml).

## External/cloud configuration

```toml
[networks.portal]
hosting = "external"
mode = "external_uam"
url = "https://portal.example.test/tenants/tenant-a/login"
secret_file = "/etc/irongate/secrets/portal"
tenant_id = "tenant-a"
site_id = "site-one"
gateway_id = "ig-lab-01"

[networks.authentication]
provider = "radius"
radius_pool = "central"
```

See the complete [cloud profile](../examples/cloud-portal.toml). The URL identifies the frontend/protocol endpoint; the explicit IDs identify its configured tenant/site/device namespace. `gateway_id` is a registered fleet identity, not merely a display name or VLAN number. These IDs are not secrets and must not be treated as authorization credentials. Paths or query parameters that happen to contain a tenant name do not establish tenant scope.

For a private single-site external portal, omit all three IDs. Setting any one requires all three; each is a bounded 1–64-character ASCII identifier using letters, digits, hyphens or underscores. Tenant-scoped external URLs require HTTPS. Local modes reject external URL/secret/scope fields, and external modes reject local listen/terms fields. `hosting` must agree with the selected current mode; unknown hosting values or partial scopes fail validation.

All examples now declare hosting. Older schema-1 files without it remain valid with `portal.hosting.implicit`, a warning asking for an explicit setting; hosting is inferred from the existing unambiguous mode. This preserves existing files without silently hiding their placement. Text and JSON config results—and configured doctor output—show local versus external placement and whether tenant scope is configured. They do not echo portal secret references, URLs or tenant/site/gateway identifier values in the placement summary. Passing config checks never establishes service reachability or a working cloud protocol.

## Required external portal contract

Before external runtime implementation, freeze a versioned handoff/callback contract. The current `external_uam` mode names an interoperability boundary; it does not mean that any arbitrary cloud webpage is compatible. Add a distinct mode/version if a new native cloud protocol differs from the legacy UAM contract, rather than silently changing UAM semantics.

The gateway remains the authority for network/session identity, policy installation, counters and revocation. The cloud must not grant arbitrary access by supplying a MAC, VLAN, tenant or `authorized=true` field. A handoff must bind an unpredictable bounded attempt to the registered tenant/site/gateway, ingress network, session slot/generation, expiry and permitted action. The gateway must authenticate the response, check its configured namespace and live attempt, consume it once and reject tampering, replay, wrong tenant/device/network and delayed results after slot reuse. Use vetted cryptography and per-device scoped credentials/key rotation; never share one unconstrained secret across tenants.

Keep portal frontend and RADIUS paths independently configured and observable. The gateway RADIUS client should continue owning admission/accounting semantics when that provider is selected. Decide whether credentials submit directly to gateway HTTPS or use a precisely specified provider contract; do not introduce ad hoc credential encryption or grant tokens in a query string. Avoid sending reusable secrets, passwords or subscriber identifiers through redirect URLs, referrers or ordinary logs. Test browser cross-origin/private-network behavior and validated HTTPS trust for the selected flow.

Do not require inbound cloud access to a gateway behind NAT by default. A browser-carried callback or an explicitly authenticated outbound control channel are candidates; choose and test the actual contract before promising either. A central management channel is separate from the client-facing portal and must not become an undeclared forwarding bypass.

Pre-admission access to an external portal needs an explicit, profile-scoped garden: exact necessary HTTPS origin(s), resolver behavior, allowed redirects/assets and controlled address refresh. Do not exempt an entire CDN, shared cloud address range or tenant-provided arbitrary URL. Revalidate redirect targets and ensure changes do not widen access across VLANs. Cloud connectivity, portal reachability, callback verification, RADIUS authentication and RADIUS accounting are separate health dimensions.

Cloud outage must not silently switch to local consent or grant access. Deny new external-dependent admission according to declared policy; preserve or expire existing sessions under their already granted policy, and report the unavailable dependency. A local fallback would change the authentication contract and needs its own explicit, tested configuration in a future increment.

## Demo and acceptance

The [VM demo](vm-demo.md) will first run local consent, then a separate HTTPS portal service inside the VM as a cloud stand-in. The external fixture must exercise real gateway-bound handoff/provider decisions, not directly edit session state or firewall rules. Test two tenant/site/device namespaces on the shared service, swapped callbacks, forged tenant fields, replay, restart/session reuse and service outage. A local and external portal must coexist on separate VLANs. A successful mock integration proves the tested protocol/isolation contract, not arbitrary SaaS interoperability or WAN capacity.
