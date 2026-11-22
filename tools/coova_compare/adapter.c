/* Fixture context and intercepted sends only; no sockets or platform changes. */
#include "adapter.h"
#include <time.h>

struct fixture_options _options;
static uint8_t captured[2048];
static size_t captured_length;
static volatile uint32_t sink;

uint8_t *dhcp_nexthop(struct dhcp_t *context) { return context->mac; }
int dhcp_expand_captiveportal_uri(struct dhcp_conn_t *conn, char *out, size_t len) {
    (void)conn; (void)out; (void)len;
    abort(); /* Disabled profile: fail loudly if this stub is ever called. */
}
int dhcp_send(struct dhcp_t *context, int index, uint8_t *mac, uint8_t *packet, size_t length) {
    (void)context; (void)index; (void)mac;
    if (length > sizeof(captured)) abort();
    memcpy(captured, packet, length);
    captured_length = length;
    sink += packet[length - 1];
    return 0;
}

static uint64_t now(void) {
    struct timespec time;
    if (clock_gettime(CLOCK_MONOTONIC, &time)) abort();
    return (uint64_t)time.tv_sec * 1000000000u + time.tv_nsec;
}
static void hex(const uint8_t *bytes, size_t length) {
    for (size_t i = 0; i < length; i++) printf("%02x", bytes[i]);
}
static struct in_addr address(const char *text) {
    struct in_addr result;
    if (inet_pton(AF_INET, text, &result) != 1) abort();
    return result;
}

int main(int argc, char **argv) {
    if (argc != 4) return 2; /* mode, tag code, iteration count */
    char *end = NULL;
    unsigned long iterations = strtoul(argv[3], &end, 10);
    if (!end || *end || iterations < 1 || iterations > 100000000) return 2;
    uint8_t buffer[4097];
    size_t length = fread(buffer, 1, sizeof(buffer), stdin);
    if (ferror(stdin) || length > 4096) return 2;
    /* Exact input allocation makes ASan observe actual read boundaries. */
    uint8_t *input = malloc(length ? length : 1);
    if (!input) return 2;
    memcpy(input, buffer, length);
    uint64_t elapsed;
    if (!strcmp(argv[1], "option")) {
        unsigned long code = strtoul(argv[2], &end, 10);
        if (*end || code > 254 || length < 240) { free(input); return 2; }
        struct dhcp_tag_t *tag = NULL;
        int result = -1;
        result = dhcp_gettag((struct dhcp_packet_t *)input, length, &tag, code);
        uint64_t start = now();
        for (unsigned long i = 0; i < iterations; i++) {
            result = dhcp_gettag((struct dhcp_packet_t *)input, length, &tag, code);
            sink += result == 0 ? tag->l : 0;
        }
        elapsed = now() - start;
        printf("{\"status\":\"%s\",\"elapsed_ns\":%llu,\"value\":\"",
               result == 0 ? "found" : "missing", (unsigned long long)elapsed);
        if (!result) hex(tag->v, tag->l);
        puts("\"}");
    } else if (!strcmp(argv[1], "offer")) {
        /* This wrapper accepts only structurally complete pilot input frames.
         * No DHCP accept/reject result is inferred from invoking an encoder. */
        size_t link = length >= 18 && input[12] == 0x81 && input[13] == 0 ? 18 : 14;
        if (length < link + 28 + 240 || input[link] != 0x45) { free(input); return 2; }
        struct dhcp_t context = {.lease = 3600, .mac = {2,0,0,0,0,1}};
        struct dhcp_conn_t conn = {.parent = &context, .mtu = 1500};
        conn.hisip = address("192.0.2.100"); conn.ourip = address("192.0.2.1");
        conn.hismask = address("255.255.255.0"); conn.dns1 = address("192.0.2.53");
        memcpy(conn.hismac, input + link + 28 + 28, 6);
        if (dhcp_sendOFFER(&conn, input, length)) abort();
        uint64_t start = now();
        for (unsigned long i = 0; i < iterations; i++)
            if (dhcp_sendOFFER(&conn, input, length)) abort();
        elapsed = now() - start;
        printf("{\"status\":\"encoded\",\"elapsed_ns\":%llu,\"frame\":\"",
               (unsigned long long)elapsed);
        hex(captured, captured_length);
        puts("\"}");
    } else { free(input); return 2; }
    free(input);
    return 0;
}
