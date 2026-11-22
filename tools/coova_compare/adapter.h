/* IronGate offline fixture adapter, GPL-2.0-or-later.
 * Wire types/macros and numeric DHCP constants come from pinned upstream blobs.
 * Contexts below are fixture-only substitutes, not ABI-compatible daemon types.
 */
#ifndef IRONGATE_COOVA_ADAPTER_H
#define IRONGATE_COOVA_ADAPTER_H
#include <arpa/inet.h>
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <syslog.h>
#define ENABLE_IEEE8021Q 1
#define PKT_MAX_LEN 9000
#include "pkt.h"
#include "constants.h"

struct dhcp_t { uint32_t lease; uint8_t mac[6]; };
struct dhcp_conn_t {
    struct dhcp_t *parent;
    struct in_addr hisip, ourip, hismask, dns1, dns2;
    uint8_t hismac[6];
    int noc2c;
    uint16_t mtu;
    char domain[30];
};
struct fixture_options { int debug, dhcp_broadcast; const char *captiveportalapi_uri; };
extern struct fixture_options _options;
#define DHCP_CAPPORT_URI_MAX 255
#define dhcp_conn_idx(conn) 0
#define OTHER_SENDING(conn, ip)
int dhcp_gettag(struct dhcp_packet_t *, size_t, struct dhcp_tag_t **, uint8_t);
int dhcp_sendOFFER(struct dhcp_conn_t *, uint8_t *, size_t);
int dhcp_send(struct dhcp_t *, int, uint8_t *, uint8_t *, size_t);
uint8_t *dhcp_nexthop(struct dhcp_t *);
int dhcp_expand_captiveportal_uri(struct dhcp_conn_t *, char *, size_t);
#endif
