# CoovaChilli compatibility profile

Source baseline: sibling repository `../coova-chilli`, commit `61854f9`. This map supports selective behavior porting. It does not imply full compatibility or that the listed behavior has been implemented in IronGate.

## Source map

| Behavior | Source and entry points | Rust design concern |
| --- | --- | --- |
| Client packet dispatch and policy | `src/dhcp.c`: `dhcp_decaps_cb`, `dhcp_receive_ip`, `dhcp_doDNAT` | Validate before field access; specify preauthentication exceptions and NAT behavior |
| Outbound traffic | `src/chilli.c`: `cb_dhcp_data_ind`, `chilli_acct_fromsub`; `src/tun.c`: `tun_encaps` | Preserve configured counter and shaping semantics |
| Return traffic | `src/tun.c`: `tun_decaps_cb`; `src/chilli.c`: `cb_tun_ind`, `chilli_acct_tosub`; `src/dhcp.c`: `dhcp_data_req` | Associate packets with the correct live subscriber |
| Authentication | `src/radius.c`: `radius_decaps`; `src/chilli.c`: `cb_radius_auth_conf`, `dnprot_accept` | Validate attributes and authenticators; correlate requests with session generations |
| Revocation and expiry | `src/chilli.c`: `checkconn`, `terminate_appconn`, `cb_radius_coa_ind` | Revoke forwarding before completing disconnect |
| Shared session state | `src/chilli.h`: `app_conn_t`; `src/session.h` | Replace pointer graphs with explicit ownership and bounded slots |
| DNS and packet layouts | `src/dns.c`, `src/pkt.h`, `src/pkt.c`, `src/chksum.c` | Borrowed parsing, bounded mutation and checksum repair |
| Portal and administration | `src/redir.c`, `src/redir.h`, `src/cmdsock.c` | Separate external portal contracts from native-struct IPC |
| Platform I/O | `src/net.c`, `src/tun.c`, optional `src/linux/xt_coova.c` | Choose kernel-backed or userspace forwarding deliberately |
| Config and saved state | `src/options.c`, `src/cmdline.ggo`, `src/statusfile.c` | Reject unsupported settings; version new formats; define restart semantics |

## Proposed first release profile

Linux IPv4 with Ethernet/ARP, one upstream path, and explicit untagged or single-tag VLAN subscriber profiles with distinct portals and policies. Support local DHCP or an explicitly selected integration, standalone consent/local records and centralized RADIUS/UAM profiles, required RADIUS methods, accounting, CoA/disconnect, DNS/garden rules, expiry, quotas and bandwidth limits.

Per-VLAN portal selection and isolation are required. Determine whether DHCP relay, MAC authentication, EAPOL, Layer 3 mode, arbitrary client addresses, multiple upstream routes, scripts, embedded platforms, local credentials/vouchers and state persistence are mandatory. Stacked VLAN tags and overlapping subscriber subnets are separate scope. Features absent from the agreed profile must be rejected clearly when configured.

## Deployment decisions to record

| Decision | Current status |
| --- | --- |
| Target device, CPU, RAM and Linux versions | Unspecified |
| Maximum active sessions and pending authentication requests | Unspecified |
| Traffic capacity, packet sizes, MTU and latency target | Unspecified |
| Portal protocol, redirect parameters and challenge behavior | Unspecified |
| Authentication methods, vendor attributes and RADIUS retry policy | Unspecified |
| CoA matching and permitted policy changes | Unspecified |
| Accounting directions, garden traffic and dropped-packet semantics | Unspecified |
| Timeout allowance, outage behavior and preauthentication policy | Unspecified |
| Configuration, scripts and administrative command compatibility | Unspecified |
| Restart, session loss and rollback tolerance | Unspecified |
| VLAN profiles, portals, provider and accounting mappings | Distinct per-VLAN profiles required; actual network inventory unspecified |

Native portal IPC and binary status files contain build-dependent C structures. A standalone Rust process should use explicit versioned formats and adapters if compatibility is required. Do not deserialize pointer-bearing C snapshots into Rust objects.
