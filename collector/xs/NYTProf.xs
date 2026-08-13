/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * PR-G03a — Product debugger XS load (`perl -d:NYTProfM`).
 * PR-G03b — Hold a product v5 sink and emit TIME_LINE / TIME_BLOCK /
 *           DISCOUNT through the shipped nytp_emit_* API (single writer).
 * PR-G03c — Emit SUB_ENTRY / SUB_RETURN through nytp_emit_sub_* (same
 *           held sink). Not opcode/entersub attach.
 * PR-G03d — Emit ATTRIBUTE / OPTION / NEW_FID / SRC_LINE / SUB_INFO /
 *           PID_START / PID_END through nytp_emit_* (same held sink).
 * PR-G03e — START_DEFLATE via nytp_emit_start_deflate on the held sink
 *           (zlib after tag 'z'; -lz only). Mid-deflate fork residual.
 * PR-G04  — emit_sub_callers via nytp_emit_sub_callers (same held sink).
 *           Live DB::sub attach lives in Devel/NYTProfM.pm (NYTPROF file=).
 * PR-G05  — product_v6_collect + enable_sink_v6. Default D1-B link has
 *           NYTPROF_V6_COLLECT undefined (format=v6 fail-closed). D1-A
 *           xs-nytprof-v6 compiles with -DNYTPROF_V6_COLLECT and links
 *           v6 + -lz -lzstd -llz4.
 * PR-G06  — fork_prepare / fork_resume_parent / fork_resume_child call
 *           shipped nytp_fork_* + addpid reinit (COL-015). Mid-deflate
 *           continue-in-child remains residual.
 *
 * MODULE Devel::NYTProfM; PACKAGE = DB
 * Default link: libnytp_sink_v5.a + -lz only (D1-B).
 *
 * Default init_profiler() holds an in-memory nytp_v5_sink_create(NULL)
 * and does not write nytprof.out. NYTPROF file= + enable_sink is the
 * product v5 file path (G04). enable_sink_v6 is D1-A only.
 */
#define PERL_NO_GET_CONTEXT
#include "EXTERN.h"
#include "perl.h"
#include "XSUB.h"

#include "nytp_sink.h"
#include "nytp_sink_v5.h"
#include "nytp_clock.h"
#include "nytp_fork.h"
#ifdef NYTPROF_V6_COLLECT
#include "nytp_sink_v6.h"
#endif

#include <limits.h>
#include <string.h>

static nytp_sink *product_sink = NULL;

/* Copy Perl SV bytes into an owned C string; do not pass SvPV to nytp_sv_cstr. */
static char *
nytp_xs_owned_cstr(pTHX_ SV *sv)
{
    STRLEN len = 0;
    const char *pv;
    char *owned;

    if (sv != NULL && SvOK(sv)) {
        pv = SvPVbyte(sv, len);
    } else {
        pv = "";
        len = 0;
    }
    Newx(owned, len + 1, char);
    if (len > 0) {
        memcpy(owned, pv, len);
    }
    owned[len] = '\0';
    return owned;
}

static void
nytp_product_sink_drop(void)
{
    if (product_sink == NULL) {
        return;
    }
    (void)nytp_sink_close(product_sink);
    nytp_sink_destroy(product_sink);
    product_sink = NULL;
}

/* Replace the held sink with a new v5 sink (path NULL = in-memory). */
static nytp_status
nytp_product_sink_hold(const char *path)
{
    nytp_product_sink_drop();
    product_sink = nytp_v5_sink_create(path);
    if (product_sink == NULL) {
        return NYTP_ERR_IO;
    }
    return NYTP_OK;
}

/*
 * nytp_m4_mini_sample_run expects OPEN (it emits header attrs then activate).
 * enable_sink() leaves ACTIVE, so recreate at the same path in OPEN.
 */
static nytp_status
nytp_product_sink_reopen_open(void)
{
    char path_copy[4096];
    const char *path = NULL;
    int have_path = 0;

    path_copy[0] = '\0';
    if (product_sink != NULL) {
        path = nytp_v5_sink_path(product_sink);
        if (path != NULL && path[0] != '\0') {
            size_t n = strlen(path);
            if (n >= sizeof(path_copy)) {
                return NYTP_ERR_OVERFLOW;
            }
            memcpy(path_copy, path, n + 1);
            have_path = 1;
        }
    }
    nytp_product_sink_drop();
    product_sink = nytp_v5_sink_create(have_path ? path_copy : NULL);
    if (product_sink == NULL) {
        return NYTP_ERR_IO;
    }
    return NYTP_OK;
}

static nytp_status
nytp_product_fork_resume_child(nytp_pid child_pid)
{
    nytp_fork_policy pol;
    char child_path[4096];
    const char *base = NULL;
    int n;
    nytp_status st;

    if (product_sink == NULL) {
        return NYTP_ERR_NULL;
    }
#ifdef NYTPROF_V6_COLLECT
    if (nytp_v6_sink_is_v6(product_sink)) {
        base = nytp_v6_sink_path(product_sink);
    } else
#endif
    {
        base = nytp_v5_sink_path(product_sink);
    }
    if (base == NULL || base[0] == '\0') {
        return NYTP_ERR_STATE;
    }
    n = nytp_fork_addpid_path(base, child_pid, child_path, sizeof(child_path));
    if (n < 0 || (size_t)n > sizeof(child_path)) {
        return NYTP_ERR_OVERFLOW;
    }
    pol = nytp_fork_policy_default();
    st = nytp_fork_resume_child(product_sink, &pol, NULL);
    if (st != NYTP_OK) {
        return st;
    }
#ifdef NYTPROF_V6_COLLECT
    if (nytp_v6_sink_is_v6(product_sink)) {
        st = nytp_v6_sink_fork_child_reinit(product_sink, child_path);
    } else
#endif
    {
        st = nytp_v5_sink_fork_child_reinit(product_sink, child_path);
    }
    if (st != NYTP_OK) {
        return st;
    }
    return nytp_sink_activate(product_sink);
}

MODULE = Devel::NYTProfM  PACKAGE = DB

PROTOTYPES: DISABLE

int
init_profiler()
    CODE:
        /* G03a: hold in-memory v5 sink; never a path, never nytprof.out. */
        {
            nytp_status st = nytp_product_sink_hold(NULL);
            if (st != NYTP_OK || product_sink == NULL) {
                croak("DB::init_profiler: nytp_v5_sink_create(NULL) failed");
            }
        }
        RETVAL = 1;
    OUTPUT:
        RETVAL

int
enable_sink(path)
    const char *path
    CODE:
        if (path == NULL || path[0] == '\0') {
            croak("DB::enable_sink requires a non-empty path");
        }
        {
            nytp_status st = nytp_product_sink_hold(path);
            if (st != NYTP_OK || product_sink == NULL) {
                croak("DB::enable_sink: nytp_v5_sink_create(%s) failed", path);
            }
            st = nytp_sink_activate(product_sink);
            RETVAL = (int)st;
        }
    OUTPUT:
        RETVAL

int
fork_prepare()
    CODE:
        if (product_sink == NULL) {
            RETVAL = (int)NYTP_ERR_NULL;
        } else {
            nytp_fork_policy pol = nytp_fork_policy_default();
            RETVAL = (int)nytp_fork_prepare(product_sink, &pol, NULL);
        }
    OUTPUT:
        RETVAL

int
fork_resume_parent()
    CODE:
        if (product_sink == NULL) {
            RETVAL = (int)NYTP_ERR_NULL;
        } else {
            RETVAL = (int)nytp_fork_resume_parent(product_sink, NULL);
        }
    OUTPUT:
        RETVAL

int
fork_resume_child(child_pid)
    UV child_pid
    CODE:
        RETVAL = (int)nytp_product_fork_resume_child((nytp_pid)child_pid);
    OUTPUT:
        RETVAL

int
product_v6_collect()
    CODE:
#ifdef NYTPROF_V6_COLLECT
        RETVAL = 1;
#else
        RETVAL = 0;
#endif
    OUTPUT:
        RETVAL

int
enable_sink_v6(path)
    const char *path
    CODE:
        if (path == NULL || path[0] == '\0') {
            croak("DB::enable_sink_v6 requires a non-empty path");
        }
#ifdef NYTPROF_V6_COLLECT
        {
            nytp_status st;
            nytp_product_sink_drop();
            product_sink = nytp_v6_sink_create(path);
            if (product_sink == NULL) {
                croak("DB::enable_sink_v6: nytp_v6_sink_create(%s) failed",
                      path);
            }
            st = nytp_sink_activate(product_sink);
            RETVAL = (int)st;
        }
#else
        croak("format=v6 requires v6-enabled build (install v6_collect "
              "package or rebuild with --with v6_collect)");
        RETVAL = (int)NYTP_ERR_UNSUPPORTED;
#endif
    OUTPUT:
        RETVAL

int
emit_time_line(ticks, fid, line)
    IV ticks
    UV fid
    UV line
    CODE:
        if (product_sink == NULL) {
            RETVAL = (int)NYTP_ERR_NULL;
        } else {
            RETVAL = (int)nytp_emit_time_line(product_sink, (nytp_ticks)ticks,
                                              (nytp_fid)fid, (nytp_line)line);
        }
    OUTPUT:
        RETVAL

int
emit_time_block(ticks, fid, line, block_line, sub_line)
    IV ticks
    UV fid
    UV line
    UV block_line
    UV sub_line
    CODE:
        if (product_sink == NULL) {
            RETVAL = (int)NYTP_ERR_NULL;
        } else {
            RETVAL = (int)nytp_emit_time_block(
                product_sink, (nytp_ticks)ticks, (nytp_fid)fid,
                (nytp_line)line, (nytp_line)block_line, (nytp_line)sub_line);
        }
    OUTPUT:
        RETVAL

int
emit_discount()
    CODE:
        if (product_sink == NULL) {
            RETVAL = (int)NYTP_ERR_NULL;
        } else {
            RETVAL = (int)nytp_emit_discount(product_sink);
        }
    OUTPUT:
        RETVAL

int
emit_sub_entry(caller_fid, caller_line)
    UV caller_fid
    UV caller_line
    CODE:
        if (product_sink == NULL) {
            RETVAL = (int)NYTP_ERR_NULL;
        } else {
            RETVAL = (int)nytp_emit_sub_entry(product_sink,
                                              (nytp_fid)caller_fid,
                                              (nytp_line)caller_line);
        }
    OUTPUT:
        RETVAL

int
emit_sub_return(depth, incl, excl, subname)
    UV depth
    NV incl
    NV excl
    SV *subname
    CODE:
        if (product_sink == NULL) {
            RETVAL = (int)NYTP_ERR_NULL;
        } else {
            char *owned = nytp_xs_owned_cstr(aTHX_ subname);
            RETVAL = (int)nytp_emit_sub_return(product_sink, (nytp_depth)depth,
                                               (double)incl, (double)excl,
                                               nytp_sv_cstr(owned));
            Safefree(owned);
        }
    OUTPUT:
        RETVAL

int
emit_sub_callers(fid, line, count, incl, excl, reci, rec_depth, called, caller)
    UV fid
    UV line
    UV count
    NV incl
    NV excl
    NV reci
    UV rec_depth
    SV *called
    SV *caller
    CODE:
        if (product_sink == NULL) {
            RETVAL = (int)NYTP_ERR_NULL;
        } else {
            char *called_c = nytp_xs_owned_cstr(aTHX_ called);
            char *caller_c = nytp_xs_owned_cstr(aTHX_ caller);
            RETVAL = (int)nytp_emit_sub_callers(
                product_sink, (nytp_fid)fid, (nytp_line)line,
                (uint32_t)count, (double)incl, (double)excl, (double)reci,
                (uint32_t)rec_depth, nytp_sv_cstr(called_c),
                nytp_sv_cstr(caller_c));
            Safefree(called_c);
            Safefree(caller_c);
        }
    OUTPUT:
        RETVAL

int
emit_attribute(key, value)
    SV *key
    SV *value
    CODE:
        if (product_sink == NULL) {
            RETVAL = (int)NYTP_ERR_NULL;
        } else {
            char *k = nytp_xs_owned_cstr(aTHX_ key);
            char *v = nytp_xs_owned_cstr(aTHX_ value);
            RETVAL = (int)nytp_emit_attribute(product_sink, nytp_sv_cstr(k),
                                              nytp_sv_cstr(v));
            Safefree(k);
            Safefree(v);
        }
    OUTPUT:
        RETVAL

int
emit_option(key, value)
    SV *key
    SV *value
    CODE:
        if (product_sink == NULL) {
            RETVAL = (int)NYTP_ERR_NULL;
        } else {
            char *k = nytp_xs_owned_cstr(aTHX_ key);
            char *v = nytp_xs_owned_cstr(aTHX_ value);
            RETVAL = (int)nytp_emit_option(product_sink, nytp_sv_cstr(k),
                                           nytp_sv_cstr(v));
            Safefree(k);
            Safefree(v);
        }
    OUTPUT:
        RETVAL

int
emit_new_fid(fid, eval_fid, eval_line, flags, size, mtime, name)
    UV fid
    UV eval_fid
    UV eval_line
    UV flags
    UV size
    UV mtime
    SV *name
    CODE:
        if (product_sink == NULL) {
            RETVAL = (int)NYTP_ERR_NULL;
        } else {
            char *owned = nytp_xs_owned_cstr(aTHX_ name);
            RETVAL = (int)nytp_emit_new_fid(
                product_sink, (nytp_fid)fid, (nytp_fid)eval_fid,
                (nytp_line)eval_line, (uint32_t)flags, (uint32_t)size,
                (uint32_t)mtime, nytp_sv_cstr(owned));
            Safefree(owned);
        }
    OUTPUT:
        RETVAL

int
emit_src_line(fid, line, text)
    UV fid
    UV line
    SV *text
    CODE:
        if (product_sink == NULL) {
            RETVAL = (int)NYTP_ERR_NULL;
        } else {
            char *owned = nytp_xs_owned_cstr(aTHX_ text);
            RETVAL = (int)nytp_emit_src_line(product_sink, (nytp_fid)fid,
                                             (nytp_line)line,
                                             nytp_sv_cstr(owned));
            Safefree(owned);
        }
    OUTPUT:
        RETVAL

int
emit_sub_info(fid, first_line, last_line, name)
    UV fid
    UV first_line
    UV last_line
    SV *name
    CODE:
        if (product_sink == NULL) {
            RETVAL = (int)NYTP_ERR_NULL;
        } else {
            char *owned = nytp_xs_owned_cstr(aTHX_ name);
            RETVAL = (int)nytp_emit_sub_info(product_sink, (nytp_fid)fid,
                                             (nytp_line)first_line,
                                             (nytp_line)last_line,
                                             nytp_sv_cstr(owned));
            Safefree(owned);
        }
    OUTPUT:
        RETVAL

int
emit_pid_start(pid, ppid, start_time)
    UV pid
    UV ppid
    NV start_time
    CODE:
        if (product_sink == NULL) {
            RETVAL = (int)NYTP_ERR_NULL;
        } else {
            RETVAL = (int)nytp_emit_pid_start(product_sink, (nytp_pid)pid,
                                              (nytp_pid)ppid,
                                              (double)start_time);
        }
    OUTPUT:
        RETVAL

int
emit_pid_end(pid, end_time)
    UV pid
    NV end_time
    CODE:
        if (product_sink == NULL) {
            RETVAL = (int)NYTP_ERR_NULL;
        } else {
            RETVAL = (int)nytp_emit_pid_end(product_sink, (nytp_pid)pid,
                                            (double)end_time);
        }
    OUTPUT:
        RETVAL

int
emit_start_deflate()
    CODE:
        if (product_sink == NULL) {
            RETVAL = (int)NYTP_ERR_NULL;
        } else {
            RETVAL = (int)nytp_emit_start_deflate(product_sink);
        }
    OUTPUT:
        RETVAL

int
is_deflating()
    CODE:
        if (product_sink == NULL) {
            RETVAL = 0;
        } else {
            RETVAL = nytp_v5_sink_is_deflating(product_sink);
        }
    OUTPUT:
        RETVAL

int
finish_profiler()
    CODE:
        nytp_product_sink_drop();
        RETVAL = 1;
    OUTPUT:
        RETVAL

int
run_m4_mini_sample()
    CODE:
        {
            nytp_m4_harness_result res;
            nytp_status st;

            memset(&res, 0, sizeof(res));
            st = nytp_product_sink_reopen_open();
            if (st != NYTP_OK || product_sink == NULL) {
                RETVAL = (int)(st != NYTP_OK ? st : NYTP_ERR_IO);
            } else {
                st = nytp_m4_mini_sample_run(product_sink, &res);
                if (st == NYTP_OK && res.kinds_match && res.ticks_match) {
                    RETVAL = 0;
                } else if (st != NYTP_OK) {
                    RETVAL = (int)st;
                } else {
                    RETVAL = (int)NYTP_ERR_STATE;
                }
            }
        }
    OUTPUT:
        RETVAL

int
overflow_probe()
    CODE:
        {
            nytp_sink *tmp;
            nytp_ticks big = (nytp_ticks)INT32_MAX + 1;
            nytp_status st;

            tmp = nytp_v5_sink_create(NULL);
            if (tmp == NULL) {
                RETVAL = (int)NYTP_ERR_IO;
            } else {
                st = nytp_sink_activate(tmp);
                if (st == NYTP_OK) {
                    st = nytp_emit_time_line(tmp, big, 1, 1);
                }
                nytp_sink_destroy(tmp);
                RETVAL = (int)st;
            }
        }
    OUTPUT:
        RETVAL
