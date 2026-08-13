/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * PR-G02 — Product D1-B link probe.
 *
 * Links against libnytp_sink_v5.a with -lz only (no zstd/lz4).
 * Calls a real shipped v5 API; not a stub.
 *
 * Build/run: make -C collector probe-v5
 * Not part of `make test` (test/dev path still uses full libnytp_sink.a).
 */
#include "nytp_sink.h"
#include "nytp_sink_v5.h"

#include <stdio.h>
#include <string.h>

int main(void)
{
    nytp_sink *sink;
    const uint8_t *wire;
    size_t len = 0;

    sink = nytp_v5_sink_create(NULL);
    if (sink == NULL) {
        fprintf(stderr, "FAIL: nytp_v5_sink_create(NULL) returned NULL\n");
        return 1;
    }
    if (!nytp_v5_sink_is_v5(sink)) {
        fprintf(stderr, "FAIL: nytp_v5_sink_is_v5 is false\n");
        nytp_sink_destroy(sink);
        return 1;
    }
    wire = nytp_v5_sink_wire(sink, &len);
    if (wire == NULL || len < 9 || memcmp(wire, "NYTProf 5", 9) != 0) {
        fprintf(stderr, "FAIL: wire header is not NYTProf 5 (len=%zu)\n", len);
        nytp_sink_destroy(sink);
        return 1;
    }
    nytp_sink_destroy(sink);
    printf("OK: v5-only product link probe (NYTProf 5 header)\n");
    return 0;
}
