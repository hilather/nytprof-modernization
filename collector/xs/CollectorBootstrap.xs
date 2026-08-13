/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * PR-G02 — Load-only XS bootstrap (not the debugger).
 *
 * Package: Devel::NYTProf::CollectorBootstrap
 * Links libnytp_sink_v5.a + -lz only. Calls a real v5 sink API on BOOT.
 * Must never set $Devel::NYTProf::PRODUCT_XS_ATTACH or claim attach works.
 */
#define PERL_NO_GET_CONTEXT
#include "EXTERN.h"
#include "perl.h"
#include "XSUB.h"

#include "nytp_sink.h"
#include "nytp_sink_v5.h"

#include <string.h>

static int
nytp_bootstrap_probe_v5(void)
{
    nytp_sink *sink;
    const uint8_t *wire;
    size_t len = 0;
    int ok = 0;

    sink = nytp_v5_sink_create(NULL);
    if (sink == NULL) {
        return 0;
    }
    if (nytp_v5_sink_is_v5(sink)) {
        wire = nytp_v5_sink_wire(sink, &len);
        if (wire != NULL && len >= 9 && memcmp(wire, "NYTProf 5", 9) == 0) {
            ok = 1;
        }
    }
    nytp_sink_destroy(sink);
    return ok;
}

MODULE = Devel::NYTProf::CollectorBootstrap  PACKAGE = Devel::NYTProf::CollectorBootstrap

BOOT:
{
    if (!nytp_bootstrap_probe_v5()) {
        croak("Devel::NYTProf::CollectorBootstrap: nytp_v5_sink_create/header probe failed");
    }
}

int
loaded()
    CODE:
        RETVAL = 1;
    OUTPUT:
        RETVAL

SV *
product_link_flavor()
    CODE:
        RETVAL = newSVpvn("v5-only", 7);
    OUTPUT:
        RETVAL

int
product_xs_attach()
    CODE:
        /* G02 load-only scaffold. Product attach is G03a–G04. */
        RETVAL = 0;
    OUTPUT:
        RETVAL

int
probe_v5_header()
    CODE:
        RETVAL = nytp_bootstrap_probe_v5();
    OUTPUT:
        RETVAL
