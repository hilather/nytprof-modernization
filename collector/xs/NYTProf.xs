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
 * PR-B1   — fid_for_filename (first-seen NEW_FID) + visit_contexts
 *           block_and_sub_lines + optional DBSTATE/NEXTSTATE TIME_BLOCK
 *           slice (not full opcode / DI-03).
 * PR-B2   — thin OP_PRINT / OP_MATCH slowops (not full slowops.h).
 * PR-3    — finish_profiler: flush last-site, begin_finalize, SRC_LINE
 *           (product_fid_map walk, no HAS_SRC) + lookup-only SUB_INFO
 *           (parse_DBsub_value + product_fid_lookup; no NEW_FID), then
 *           PID_END. PL_perldb SAVESRC|SAVESRC_NOSUBS at file= (not $^P 0x400).
 * PR-8    — C stmt-ops TIME_BLOCK + PRINT/MATCH slowops use the same
 *           nytp_clock_now / last-site clock as TIME_LINE (not ticks=1 / 0.0).
 * PR-9    — Slowop exclusive is folded into the parent Perl frame via
 *           product_pending_child_excl + DB::take_pending_child_excl so
 *           tokenize-shaped excl = incl − children (not ≈ CORE:match).
 * PR-15   — Default stmts=1 TIME_LINE from OP_DBSTATE (pp_product_dbstate_line)
 *           + last-COP fid pointer cache. Not NEXTSTATE/UNSTACK (blocks=1).
 *           INIT leaves $DB::single=0 so Perl DB::DB does not run.
 * PR-16   — wrap_push / wrap_pop: one C crossing for wrap COP pin +
 *           fid + clock + pending-excl + SUB_RETURN/SUB_CALLERS.
 * DI-03 E1a — unstatic sink/fid/calls/mailbox for grafted pp_entersub.c.
 * DI-03 E1b — default call attach is opcode ENTERSUB (omit entersub ⇒ on).
 *           wrap=1 / use_db_sub=1 (or entersub=0) is the wrap escape.
 * DI-03 E3  — unstatic last-site flush/seed for grafted pp_leave.c.
 *           leave=1 only; default leave stays 0. UNSTACK/LEAVELOOP stay
 *           on pp_product_stmt when PRODUCT_BLOCKS. No NEXTSTATE here.
 * DI-03 E4  — default slowops=2 installs full 6.15 slowops.h
 *           (pkg::CORE:op). 3/full are aliases. Exclusive is thin.
 *           slowops=1 fail-closed.
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
#include "nytp_batch.h"
#include "nytp_clock.h"
#include "nytp_fork.h"
#ifdef NYTPROF_V6_COLLECT
#include "nytp_sink_v6.h"
#endif
#include "nytprof_pp.h"

#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#ifndef OutCopFILE
#define OutCopFILE CopFILE
#endif
#ifndef OpSIBLING
#ifdef PERL_OP_PARENT
#define OpSIBLING(o) ((o)->op_moresib ? (o)->op_sibparent : NULL)
#else
#define OpSIBLING(o) ((o)->op_sibling)
#endif
#endif

/* 6.15 NYTProf.xs ~124 — VIA_STMT path only (eval/autosplit residual). */
#ifndef NYTP_FIDf_VIA_STMT
#define NYTP_FIDf_VIA_STMT 0x0002
#endif

nytp_sink *product_sink = NULL;
/* When durable, product_sink is the batch facade and product_v5 is the child. */
static nytp_sink *product_v5 = NULL;
static int product_durable = 0;
static int product_compress_level = 0;

static nytp_sink *
product_v5_sink(void)
{
    return product_v5 != NULL ? product_v5 : product_sink;
}

/* First-seen fid table (6.15 get_file_id VIA_STMT, no eval fold). */
static HV *product_fid_map = NULL;
static UV product_next_fid = 1;
/* Same-file statement hot path: OutCopFILE pointer is stable per COP. */
static const char *product_fid_last_pv = NULL;
static STRLEN product_fid_last_len = 0;
static UV product_fid_last_id = 0;
static void product_wrap_reset(void);

/* visit_contexts pin + last_* (6.15 ~1643–1651; counts only, no ticks). */
static COP *product_pin_cop = NULL;
static unsigned int product_last_block_line = 0;
static unsigned int product_last_sub_line = 0;
static unsigned int product_last_executed_line = 0;

/* Targeted stmt slice (DI-01 fallback; not DI-03).
 * NEXTSTATE/DBSTATE plus UNSTACK/LEAVELOOP: a for-modifier compiles to
 * one dbstate + preinc + unstack (no per-iter nextstate). */
static int product_stmt_ops_installed = 0;
static Perl_ppaddr_t product_orig_pp_dbstate = NULL;
static Perl_ppaddr_t product_orig_pp_nextstate = NULL;
static Perl_ppaddr_t product_orig_pp_unstack = NULL;
#ifdef OP_LEAVELOOP
static Perl_ppaddr_t product_orig_pp_leaveloop = NULL;
#endif
static COP *product_last_stmt_cop = NULL;
static COP *product_unstack_cop = NULL;
static int product_unstack_since_stmt = 0;

/* KD-L / PR-8: one XS last-site clock for TIME_LINE and TIME_BLOCK. */
static nytp_ticks product_last_abs = 0;
static nytp_fid product_last_site_fid = 0;
static nytp_line product_last_site_line = 0;
static nytp_line product_last_site_block_line = 0;
static nytp_line product_last_site_sub_line = 0;
static int product_last_site_is_block = 0;
static int product_has_last_site = 0;
static nytp_ticks product_last_seal_abs = 0;

/* 6.15 NYTP_OPTf_SAVESRC default on. file= apply uses the Perl macros
 * (PERLDBf_SAVESRC | PERLDBf_SAVESRC_NOSUBS), not $^P |= 0x400 alone. */
static int product_savesrc = 1;

/* Slowop exclusive waiting to be absorbed by the current Perl DB::sub frame. */
static double product_pending_child_excl = 0.0;

static void
product_pending_child_excl_reset(void)
{
    product_pending_child_excl = 0.0;
}

void
product_add_pending_child_excl(NV excl)
{
    if (excl > 0.0)
        product_pending_child_excl += excl;
}

NV
product_take_pending_child_excl(void)
{
    NV v = product_pending_child_excl;
    product_pending_child_excl = 0.0;
    return v;
}

static void
product_last_site_reset(void)
{
    product_last_abs = 0;
    product_last_site_fid = 0;
    product_last_site_line = 0;
    product_last_site_block_line = 0;
    product_last_site_sub_line = 0;
    product_last_site_is_block = 0;
    product_has_last_site = 0;
    product_last_seal_abs = 0;
    product_pending_child_excl_reset();
}

/* Emit leftover interval to the previous site (TIME_LINE or TIME_BLOCK). */
static nytp_status
product_emit_last_site_elapsed(nytp_ticks elapsed)
{
    if (product_sink == NULL) {
        return NYTP_ERR_NULL;
    }
    if (product_last_site_is_block) {
        return nytp_emit_time_block(product_sink, elapsed,
                                    product_last_site_fid,
                                    product_last_site_line,
                                    product_last_site_block_line,
                                    product_last_site_sub_line);
    }
    return nytp_emit_time_line(product_sink, elapsed, product_last_site_fid,
                               product_last_site_line);
}

/* Periodic durable seal: reuse last-site `now`. Does not emit leftover last-site. */
#define NYTP_PRODUCT_SEAL_DIRTY_MIN 262144u

/* Drain batch then publish. Does not go through nytp_sink_flush, so a
 * snapshot I/O error cannot sticky-fail the live sink. */
static nytp_status
product_durable_publish(void)
{
    nytp_sink *v5 = product_v5_sink();
    nytp_batch *batch;
    nytp_status st;

    if (v5 == NULL) {
        return NYTP_ERR_NULL;
    }
    batch = nytp_batch_sink_batch(product_sink);
    if (batch != NULL) {
        st = nytp_batch_flush(batch);
        if (st != NYTP_OK) {
            return st;
        }
    }
    return nytp_v5_seal_publish(v5);
}

static nytp_status
product_maybe_durable_seal(nytp_ticks now)
{
    nytp_sink *v5;
    size_t live;
    size_t last;
    nytp_status st;
    nytp_ticks t1 = 0;

    if (!product_durable) {
        return NYTP_OK;
    }
    v5 = product_v5_sink();
    if (v5 == NULL) {
        return NYTP_OK;
    }
    live = nytp_v5_sink_wire_len(v5);
    last = nytp_v5_sink_len_at_last_seal(v5);
    if (now < product_last_seal_abs) {
        return NYTP_OK;
    }
    if (now - product_last_seal_abs < (nytp_ticks)NYTP_TICKS_PER_SEC) {
        return NYTP_OK;
    }
    if (live < last || live - last < NYTP_PRODUCT_SEAL_DIRTY_MIN) {
        return NYTP_OK;
    }
    st = product_durable_publish();
    if (st != NYTP_OK) {
        /* Snapshot is best-effort; keep emitting into live RAM. */
        return NYTP_OK;
    }
    if (nytp_clock_now(&t1) == NYTP_OK && t1 > now) {
        product_last_abs += (t1 - now);
    }
    product_last_seal_abs = now;
    return NYTP_OK;
}

nytp_status
product_flush_last_site(void)
{
    nytp_ticks now = 0;
    nytp_status st;

    if (!product_has_last_site || product_sink == NULL) {
        return NYTP_OK;
    }
    st = nytp_clock_now(&now);
    if (st != NYTP_OK) {
        return st;
    }
    if (now < product_last_abs) {
        return NYTP_ERR_OVERFLOW;
    }
    st = product_emit_last_site_elapsed(now - product_last_abs);
    if (st != NYTP_OK) {
        return st;
    }
    product_has_last_site = 0;
    return NYTP_OK;
}

/* Close previous interval, then seed this site *after* the emit so
 * hook/write cost is not charged to TIME_LINE (6.15 DB_stmt order). */
static nytp_status
product_close_last_site(void)
{
    nytp_ticks now = 0;
    nytp_status st;

    if (!product_has_last_site || product_sink == NULL) {
        return NYTP_OK;
    }
    st = nytp_clock_now(&now);
    if (st != NYTP_OK) {
        return st;
    }
    if (now < product_last_abs) {
        return NYTP_ERR_OVERFLOW;
    }
    st = product_emit_last_site_elapsed(now - product_last_abs);
    if (st != NYTP_OK) {
        return st;
    }
    product_has_last_site = 0;
    (void)product_maybe_durable_seal(now);
    return NYTP_OK;
}

static nytp_status
product_seed_last_site(nytp_fid fid, nytp_line line, int is_block,
                       nytp_line block_line, nytp_line sub_line)
{
    nytp_ticks now = 0;
    nytp_status st;

    st = nytp_clock_now(&now);
    if (st != NYTP_OK) {
        return st;
    }
    product_last_abs = now;
    product_last_site_fid = fid;
    product_last_site_line = line;
    product_last_site_block_line = block_line;
    product_last_site_sub_line = sub_line;
    product_last_site_is_block = is_block;
    product_has_last_site = 1;
    return NYTP_OK;
}

nytp_status
product_emit_attributed_time_line(nytp_fid fid, nytp_line line)
{
    nytp_status st;

    if (product_sink == NULL) {
        return NYTP_ERR_NULL;
    }
    if (line == 0) {
        line = 1;
    }
    st = product_close_last_site();
    if (st != NYTP_OK) {
        return st;
    }
    return product_seed_last_site(fid, line, 0, 0, 0);
}

/* BASE-003: attribute now-last to the previous site, then seed this COP. */
nytp_status
product_emit_attributed_time_block(nytp_fid fid, nytp_line line,
                                   nytp_line block_line, nytp_line sub_line)
{
    nytp_status st;

    if (product_sink == NULL) {
        return NYTP_ERR_NULL;
    }
    if (line == 0) {
        line = 1;
    }
    if (block_line == 0) {
        block_line = line;
    }
    if (sub_line == 0) {
        sub_line = line;
    }
    st = product_close_last_site();
    if (st != NYTP_OK) {
        return st;
    }
    return product_seed_last_site(fid, line, 1, block_line, sub_line);
}

static nytp_status
product_emit_header_and_pid_start(pTHX)
{
    char tps[16];
    nytp_status st;
    SV *app_sv;
    STRLEN app_len = 0;
    const char *argv0;

    if (product_sink == NULL) {
        return NYTP_ERR_NULL;
    }
    (void)snprintf(tps, sizeof(tps), "%d", NYTP_TICKS_PER_SEC);
    st = nytp_emit_attribute(product_sink, nytp_sv_cstr("ticks_per_sec"),
                             nytp_sv_cstr(tps));
    if (st != NYTP_OK) {
        return st;
    }
    /* 6.15 output_header: ATTRIBUTE application = $0. Without this the
     * HTML primary-fid heuristic can pick Config_heavy.pl (EL8
     * /usr/lib64/perl5/... is not matched by /lib/perl). */
    app_sv = get_sv("0", 0);
    argv0 = (app_sv != NULL && SvOK(app_sv)) ? SvPV(app_sv, app_len) : NULL;
    if (argv0 == NULL || app_len == 0) {
        argv0 = "-";
        app_len = 1;
    }
    st = nytp_emit_attribute(product_sink, nytp_sv_cstr("application"),
                             nytp_sv(argv0, app_len, 0));
    if (st != NYTP_OK) {
        return st;
    }
    if (product_compress_level > 0) {
        char lvl[16];
        (void)snprintf(lvl, sizeof(lvl), "%d", product_compress_level);
        st = nytp_emit_option(product_sink, nytp_sv_cstr("compress"),
                              nytp_sv_cstr(lvl));
        if (st != NYTP_OK) {
            return st;
        }
    }
    st = nytp_sink_activate(product_sink);
    if (st != NYTP_OK) {
        return st;
    }
    st = nytp_emit_pid_start(product_sink, (nytp_pid)getpid(),
                             (nytp_pid)getppid(), 0.0);
    if (st != NYTP_OK) {
        return st;
    }
    if (product_durable) {
        nytp_batch *batch = nytp_batch_sink_batch(product_sink);
        nytp_ticks now = 0;
        /* Drain only — do not v5_flush/seal yet, or z would sit after magic. */
        if (batch != NULL) {
            st = nytp_batch_flush(batch);
            if (st != NYTP_OK) {
                return st;
            }
        }
        st = nytp_v5_sink_mark_header_end(product_v5_sink());
        if (st != NYTP_OK) {
            return st;
        }
        if (nytp_clock_now(&now) == NYTP_OK) {
            product_last_seal_abs = now;
        }
    }
    return st;
}

static void
product_fid_reset(pTHX)
{
    if (product_fid_map != NULL) {
        hv_clear(product_fid_map);
    }
    product_fid_last_pv = NULL;
    product_fid_last_len = 0;
    product_fid_last_id = 0;
    product_next_fid = 1;
    product_last_stmt_cop = NULL;
    product_unstack_cop = NULL;
    product_unstack_since_stmt = 0;
    product_last_site_reset();
}

static UV
product_fid_for_pv(pTHX_ const char *pv, STRLEN len)
{
    SV **slot;
    UV fid;

    if (pv == NULL || len == 0) {
        pv = "-";
        len = 1;
    }
    if (product_fid_map == NULL) {
        product_fid_map = newHV();
    }
    slot = hv_fetch(product_fid_map, pv, (I32)len, 0);
    if (slot != NULL && *slot != NULL && SvOK(*slot)) {
        return SvUV(*slot);
    }
    fid = product_next_fid++;
    if (product_sink != NULL) {
        (void)nytp_emit_new_fid(product_sink, (nytp_fid)fid, 0, 0,
                                NYTP_FIDf_VIA_STMT, 0, 0, nytp_sv_cstr(pv));
    }
    (void)hv_store(product_fid_map, pv, (I32)len, newSVuv(fid), 0);
    return fid;
}

/* OutCopFILE is interned and stable. Do not cache SvPVbyte from
 * fid_for_filename — that pointer dies with the SV and evicts this hit. */
nytp_fid
product_fid_for_file_ptr(pTHX_ const char *pv)
{
    UV fid;
    STRLEN len;

    if (pv == NULL || pv[0] == '\0') {
        pv = "-";
    }
    if (product_fid_last_pv == pv && product_fid_last_id != 0) {
        return (nytp_fid)product_fid_last_id;
    }
    len = (STRLEN)strlen(pv);
    fid = product_fid_for_pv(aTHX_ pv, len);
    product_fid_last_pv = pv;
    product_fid_last_len = len;
    product_fid_last_id = fid;
    return (nytp_fid)fid;
}

static UV
product_fid_for_filename(pTHX_ SV *path_sv)
{
    STRLEN len = 0;
    const char *pv;

    if (path_sv != NULL && SvOK(path_sv)) {
        pv = SvPVbyte(path_sv, len);
    } else {
        pv = "-";
        len = 1;
    }
    return product_fid_for_pv(aTHX_ pv, len);
}

/* Lookup-only (KD-N). Never emit NEW_FID. Missing → 0. */
static UV
product_fid_lookup(pTHX_ const char *filename, STRLEN filename_len)
{
    SV **slot;

    if (product_fid_map == NULL || filename == NULL || filename_len == 0) {
        return 0;
    }
    slot = hv_fetch(product_fid_map, filename, (I32)filename_len, 0);
    if (slot != NULL && *slot != NULL && SvOK(*slot)) {
        return SvUV(*slot);
    }
    return 0;
}

/*
 * Port of baseline/6.15/src/NYTProf.xs parse_DBsub_value ~3383
 * ("filename:first-last"). Used by write_sub_line_ranges ~3563.
 */
static int
product_parse_DBsub_value(pTHX_ SV *sv, STRLEN *filename_len_p,
                          UV *first_line_p, UV *last_line_p, char *sub_name)
{
    char *filename;
    char *first;
    char *last;
    int first_is_neg = 0;

    if (sv == NULL || !SvOK(sv)) {
        return 0;
    }
    filename = SvPV_nolen(sv);
    first = strrchr(filename, ':');
    if (first && filename_len_p) {
        *filename_len_p = (STRLEN)(first - filename);
    }
    if (!first) {
        return 0;
    }
    first++; /* start of first number (6.15 `if (!first++)`) */
    if (*first == '-') {
        ++first;
        first_is_neg = 1;
    }
    last = strchr(first, '-');
    if (!last || !grok_number(first, (STRLEN)(last - first), first_line_p)) {
        return 0;
    }
    if (first_is_neg) {
        warn("Negative first line number in %%DB::sub entry '%s' for %s\n",
             filename, sub_name ? sub_name : "?");
        *first_line_p = 0;
    }
    if (*++last == '-') {
        warn("Negative last line number in %%DB::sub entry '%s' for %s\n",
             filename, sub_name ? sub_name : "?");
        last = (char *)"0";
    }
    if (last_line_p) {
        *last_line_p = (UV)atoi(last);
    }
    return 1;
}

#ifndef isGV_with_GP
#define isGV_with_GP(sv) (isGV(sv) && GvGP((GV *)(sv)))
#endif

/* GvCV derefs GvGP. Stash slots may be a CV, an RV to a CV, or a
 * GP-less GV (isGV true) — the last is a SEGV without isGV_with_GP.
 */
static CV *
product_stash_val_cv(pTHX_ SV *val)
{
    PERL_UNUSED_CONTEXT;
    if (val == NULL || val == &PL_sv_placeholder) {
        return NULL;
    }
    if (SvTYPE(val) == SVt_PVCV) {
        return (CV *)val;
    }
    if (SvROK(val) && SvRV(val) != NULL && SvTYPE(SvRV(val)) == SVt_PVCV) {
        return (CV *)SvRV(val);
    }
    if (!isGV_with_GP(val)) {
        return NULL;
    }
    return GvCV((GV *)val);
}

/* Mark every CV in a stash CvNODEBUG so Perl will not call DB::sub.
 * Needed for B::Hooks::EndOfScope::XS::on_scope_end: even `goto &$raw`
 * from DB::sub during use/BEGIN corrupts compile-time %^H (Variable::Magic
 * getdata then returns a source fragment such as "#pod\n").
 */
static int
product_nodebug_stash(pTHX_ const char *name)
{
    HV *stash;
    HE *he;
    int n = 0;

    if (name == NULL || name[0] == '\0') {
        return 0;
    }
    stash = gv_stashpv(name, 0);
    if (stash == NULL) {
        return 0;
    }
    hv_iterinit(stash);
    while ((he = hv_iternext(stash)) != NULL) {
        CV *cv = product_stash_val_cv(aTHX_ HeVAL(he));
        if (cv == NULL || SvTYPE((SV *)cv) != SVt_PVCV) {
            continue;
        }
        CvNODEBUG_on(cv);
        n++;
    }
    return n;
}

static void
product_apply_savesrc_flags(pTHX)
{
    if (!product_savesrc) {
        return;
    }
    /* 6.15 NYTProf.xs ~3177–3179. Do not $^P |= 0x400 alone. */
#if defined(PERLDBf_SAVESRC) && defined(PERLDBf_SAVESRC_NOSUBS)
    PL_perldb |= PERLDBf_SAVESRC | PERLDBf_SAVESRC_NOSUBS;
#elif defined(PERLDBf_SAVESRC)
    PL_perldb |= PERLDBf_SAVESRC;
#endif
}

/*
 * Walk existing product_fid_map only. Emit SRC_LINE from Perl's
 * @{"_<$file"} (gv_fetchfile_flags). Do not require NYTP_FIDf_HAS_SRC.
 */
static void
product_emit_src_lines(pTHX)
{
    char *filename;
    I32 filename_len;
    SV *fid_sv;

    if (product_sink == NULL || product_fid_map == NULL) {
        return;
    }
    hv_iterinit(product_fid_map);
    while ((fid_sv = hv_iternextsv(product_fid_map, &filename,
                                   &filename_len)) != NULL) {
        GV *gv;
        AV *src_av;
        UV fid;
        I32 last;
        I32 line;

        if (!SvOK(fid_sv) || filename == NULL || filename_len <= 0) {
            continue;
        }
        fid = SvUV(fid_sv);
        if (fid == 0) {
            continue;
        }
        gv = gv_fetchfile_flags(filename, (STRLEN)filename_len, 0);
        if (gv == NULL) {
            continue;
        }
        src_av = GvAV(gv);
        if (src_av == NULL) {
            continue;
        }
        last = av_len(src_av);
        for (line = 1; line <= last; ++line) {
            SV **svp = av_fetch(src_av, line, 0);
            STRLEN len = 0;
            const char *src = "";

            if (svp != NULL && *svp != NULL && SvOK(*svp)) {
                src = SvPV(*svp, len);
            }
            (void)nytp_emit_src_line(product_sink, (nytp_fid)fid,
                                     (nytp_line)line, nytp_sv(src, len, 0));
        }
    }
}

/*
 * Walk %DB::sub. Lookup-only fid (KD-N). Skip rows with no existing fid
 * (6.15 get_file_id(..., 0) + continue, write_sub_line_ranges ~3579–3584).
 */
static void
product_emit_sub_infos(pTHX)
{
    HV *hv;
    char *sub_name;
    I32 sub_name_len;
    SV *file_lines_sv;

    if (product_sink == NULL || PL_DBsub == NULL) {
        return;
    }
    hv = GvHV(PL_DBsub);
    if (hv == NULL) {
        return;
    }
    hv_iterinit(hv);
    while ((file_lines_sv = hv_iternextsv(hv, &sub_name, &sub_name_len))
           != NULL) {
        STRLEN file_lines_len = 0;
        char *filename;
        STRLEN filename_len = 0;
        UV first_line = 0;
        UV last_line = 0;
        UV fid;

        if (file_lines_sv == NULL || !SvOK(file_lines_sv) || sub_name == NULL) {
            continue;
        }
        filename = SvPV(file_lines_sv, file_lines_len);
        /* 6.15 write_sub_line_ranges ~3460–3463: skip /:[^0]-0$/ */
        if (file_lines_len > 4
            && filename[file_lines_len - 2] == '-'
            && filename[file_lines_len - 1] == '0'
            && filename[file_lines_len - 4] != ':'
            && filename[file_lines_len - 3] != '0') {
            continue;
        }
        if (!product_parse_DBsub_value(aTHX_ file_lines_sv, &filename_len,
                                       &first_line, &last_line, sub_name)) {
            continue;
        }
        if (filename_len == 0) {
            continue;
        }
        fid = product_fid_lookup(aTHX_ filename, filename_len);
        if (fid == 0) {
            continue;
        }
        (void)nytp_emit_sub_info(product_sink, (nytp_fid)fid,
                                 (nytp_line)first_line, (nytp_line)last_line,
                                 nytp_sv(sub_name, (size_t)sub_name_len, 0));
    }
}

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
        product_v5 = NULL;
        return;
    }
    (void)nytp_sink_close(product_sink);
    nytp_sink_destroy(product_sink);
    product_sink = NULL;
    product_v5 = NULL;
}

/* Replace the held sink with a new v5 sink (path NULL = in-memory).
 * compress_level: 0 = zlib default 6 when deflate starts; 1..9 = that level.
 * durable: wrap in nytp_batch (KD-D1); default 0 does not claim crash-safety.
 */
static nytp_status
nytp_product_sink_hold(const char *path, int compress_level, int durable)
{
    dTHX;
    nytp_sink *v5;

    nytp_product_sink_drop();
    product_fid_reset(aTHX);
    product_wrap_reset();
    product_callers_reset();
    product_durable = durable ? 1 : 0;
    product_compress_level = compress_level;
    if (product_compress_level < 0 || product_compress_level > 9) {
        return NYTP_ERR_OVERFLOW;
    }
    v5 = nytp_v5_sink_create_ex(path, compress_level);
    if (v5 == NULL) {
        return NYTP_ERR_IO;
    }
    if (product_durable) {
        if (nytp_v5_sink_set_durable(v5, 1) != NYTP_OK) {
            nytp_sink_destroy(v5);
            return NYTP_ERR_STATE;
        }
    }
    product_v5 = v5;
    if (product_durable) {
        product_sink = nytp_batch_sink_create(
            v5, NYTP_BATCH_DEFAULT_CAPACITY, NYTP_BATCH_DEFAULT_ARENA,
            NYTP_BATCH_DEFAULT_CAPACITY, 1);
        if (product_sink == NULL) {
            nytp_sink_destroy(v5);
            product_v5 = NULL;
            return NYTP_ERR_IO;
        }
    } else {
        product_sink = v5;
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
        path = nytp_v5_sink_path(product_v5_sink());
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
    {
        dTHX;
        product_fid_reset(aTHX);
        product_callers_reset();
    }
    return nytp_product_sink_hold(have_path ? path_copy : NULL,
                                  product_compress_level, product_durable);
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
        base = nytp_v5_sink_path(product_v5_sink());
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
        st = nytp_v5_sink_fork_child_reinit(product_v5_sink(), child_path);
    }
    if (st != NYTP_OK) {
        return st;
    }
    {
        dTHX;
        product_fid_reset(aTHX);
        product_callers_reset();
    }
    return nytp_sink_activate(product_sink);
}

/*
 * visit_contexts / start_cop_of_context / _check_context — port of
 * baseline/6.15/src/NYTProf.xs ~1272–1523 (do not edit the pin).
 * Pin COP is product_pin_cop (DB::DB: DB CXt_SUB blk_oldcop; opcode: PL_curcop).
 * Traces omitted. last_* are product_last_*.
 */

#ifdef CXt_NULL
/* keep CxTYPE cases compiling on all advertised perls */
#endif

static int
product_dopopcx_at(pTHX_ PERL_CONTEXT *cxstk, I32 startingblock, UV cx_type_mask)
{
    I32 i;
    PERL_CONTEXT *cx;
    for (i = startingblock; i >= 0; i--) {
        UV type_bit;
        cx = &cxstk[i];
        type_bit = ((UV)1) << CxTYPE(cx);
        if (type_bit & cx_type_mask)
            return (int)i;
    }
    return (int)i;
}

static COP *
product_start_cop_of_context(pTHX_ PERL_CONTEXT *cx)
{
    OP *start_op;
    int type;

    switch (CxTYPE(cx)) {
        case CXt_EVAL:
            start_op = (OP *)cx->blk_oldcop;
            break;
        case CXt_FORMAT:
            start_op = CvSTART(cx->blk_sub.cv);
            break;
        case CXt_SUB:
            start_op = CvSTART(cx->blk_sub.cv);
            break;
#ifdef CXt_LOOP
        case CXt_LOOP:
#if (PERL_VERSION < 10) || (PERL_VERSION == 9 && !defined(CX_LOOP_NEXTOP_GET))
            start_op = cx->blk_loop.redo_op;
#else
            start_op = cx->blk_loop.my_op->op_redoop;
#endif
            break;
#else
#ifdef CXt_LOOP_PLAIN
        case CXt_LOOP_PLAIN:
#endif
#ifdef CXt_LOOP_LAZYIV
        case CXt_LOOP_LAZYIV:
#endif
#ifdef CXt_LOOP_LAZYSV
        case CXt_LOOP_LAZYSV:
#endif
#ifdef CXt_LOOP_FOR
        case CXt_LOOP_FOR:
#endif
#ifdef CXt_LOOP_ARY
        case CXt_LOOP_ARY:
#endif
#ifdef CXt_LOOP_LIST
        case CXt_LOOP_LIST:
#endif
            start_op = (cx->blk_loop.my_op != NULL)
                           ? cx->blk_loop.my_op->op_redoop
                           : NULL;
            break;
#endif
        case CXt_BLOCK:
            start_op = (OP *)cx->blk_oldcop;
            break;
#ifdef CXt_SUBST
        case CXt_SUBST:
#endif
#ifdef CXt_NULL
        case CXt_NULL:
#endif
        default:
            start_op = NULL;
            break;
    }
    if (!start_op)
        return NULL;
    {
        OP *o = start_op;
        while (o && (type = (o->op_type) ? (int)o->op_type : (int)o->op_targ)) {
            if (type == OP_NEXTSTATE ||
#if PERL_VERSION < 11
                type == OP_SETSTATE ||
#endif
                type == OP_DBSTATE) {
                return (COP *)o;
            }
            return NULL;
        }
    }
    return NULL;
}

static PERL_CONTEXT *
product_visit_contexts(pTHX_ UV cx_type_mask,
                       int (*callback)(pTHX_ PERL_CONTEXT *cx,
                                       UV *cx_type_mask_ptr))
{
    I32 cxix = cxstack_ix;
    PERL_CONTEXT *cx = NULL;
    PERL_CONTEXT *ccstack = cxstack;
    PERL_SI *top_si = PL_curstackinfo;

    while (1) {
        while (cxix < 0 && top_si->si_type != PERLSI_MAIN) {
            top_si = top_si->si_prev;
            ccstack = top_si->si_cxstack;
            cxix = product_dopopcx_at(aTHX_ ccstack, top_si->si_cxix,
                                      cx_type_mask);
        }
        if (cxix < 0 || (cxix == 0 && !top_si->si_prev)) {
            return NULL;
        }
        cx = &ccstack[cxix];
        if (callback(aTHX_ cx, &cx_type_mask))
            return cx;
        cxix = product_dopopcx_at(aTHX_ ccstack, cxix - 1, cx_type_mask);
    }
    return NULL;
}

static int
product_cop_in_same_file(COP *a, COP *b)
{
    char *a_file;
    char *b_file;
    if (a == NULL || b == NULL)
        return 0;
    a_file = OutCopFILE(a);
    b_file = OutCopFILE(b);
    if (a_file == b_file)
        return 1;
    if (a_file && b_file && strEQ(a_file, b_file))
        return 1;
    return 0;
}

static int
product_check_context(pTHX_ PERL_CONTEXT *cx, UV *cx_type_mask_ptr)
{
    COP *near_cop;
    PERL_UNUSED_ARG(cx_type_mask_ptr);

    if (CxTYPE(cx) == CXt_SUB) {
        if (PL_debstash && cx->blk_sub.cv
            && CvSTASH(cx->blk_sub.cv) == PL_debstash)
            return 0;

        near_cop = product_start_cop_of_context(aTHX_ cx);
        if (near_cop && product_pin_cop
            && product_cop_in_same_file(near_cop, product_pin_cop)) {
            product_last_sub_line = (unsigned int)CopLINE(near_cop);
            if (!product_last_block_line)
                product_last_block_line = product_last_sub_line;
        }
        return 1;
    }

    if (product_last_block_line)
        return 0;

    if ((near_cop = product_start_cop_of_context(aTHX_ cx)) == NULL)
        return 0;

    if (product_pin_cop
        && !product_cop_in_same_file(near_cop, product_pin_cop)) {
        if (OutCopFILE(product_pin_cop)
            && '(' == *OutCopFILE(product_pin_cop)) {
            product_last_block_line = product_last_sub_line =
                product_last_executed_line;
            return 1;
        }
        return 1;
    }

    product_last_block_line = (unsigned int)CopLINE(near_cop);
    return 0;
}

static COP *
product_db_pin_cop(pTHX)
{
    I32 cxix = cxstack_ix;
    PERL_CONTEXT *ccstack = cxstack;
    PERL_SI *top_si = PL_curstackinfo;

    while (1) {
        while (cxix < 0 && top_si && top_si->si_type != PERLSI_MAIN) {
            top_si = top_si->si_prev;
            if (top_si == NULL)
                return PL_curcop;
            ccstack = top_si->si_cxstack;
            cxix = top_si->si_cxix;
        }
        if (top_si == NULL || cxix < 0
            || (cxix == 0 && !top_si->si_prev)) {
            return PL_curcop;
        }
        {
            PERL_CONTEXT *cx = &ccstack[cxix];
            if (CxTYPE(cx) == CXt_SUB && cx->blk_sub.cv && PL_debstash
                && CvSTASH(cx->blk_sub.cv) == PL_debstash
                && cx->blk_oldcop) {
                return (COP *)cx->blk_oldcop;
            }
        }
        cxix--;
    }
}

/* Skip every package-DB frame (DB::sub and this XSUB). Last DB
 * blk_oldcop is the user entersub site — same as Perl caller(0)
 * from package DB, without walking names in Perl. */
#define PRODUCT_WRAP_NAME 256

typedef struct {
    char called[PRODUCT_WRAP_NAME];
    char caller[PRODUCT_WRAP_NAME];
    nytp_ticks t0;
    double child_excl;
    UV fid;
    UV line;
} product_wrap_frame;

static product_wrap_frame *product_wrap_stack = NULL;
static int product_wrap_cap = 0;
static int product_wrap_sp = 0;

static void
product_wrap_reset(void)
{
    product_wrap_sp = 0;
}

static void
product_wrap_grow(pTHX)
{
    int ncap = product_wrap_cap ? product_wrap_cap * 2 : 64;
    if (product_wrap_stack == NULL) {
        Newxz(product_wrap_stack, ncap, product_wrap_frame);
    } else {
        Renew(product_wrap_stack, ncap, product_wrap_frame);
    }
    product_wrap_cap = ncap;
}

static void
product_wrap_copy_name(char *dst, size_t dstlen, const char *src)
{
    if (dst == NULL || dstlen == 0)
        return;
    if (src == NULL || src[0] == '\0')
        src = "main::RUNTIME";
    (void)snprintf(dst, dstlen, "%s", src);
}

static int
product_is_debugger_file(const char *file)
{
    if (file == NULL || file[0] == '\0')
        return 0;
    if (strstr(file, "NYTProfM.pm") != NULL)
        return 1;
    if (strstr(file, "NYTProf.xs") != NULL)
        return 1;
    return 0;
}

static int
product_cv_is_debugger(pTHX_ CV *cv)
{
    HV *stash;

    if (cv == NULL)
        return 0;
    stash = CvSTASH(cv);
    if (PL_debstash != NULL && stash == PL_debstash)
        return 1;
    if (stash != NULL) {
        const char *hvname = HvNAME(stash);
        if (hvname != NULL && hvname[0] == 'D' && hvname[1] == 'B'
            && hvname[2] == '\0')
            return 1;
    }
    return 0;
}

static COP *
product_wrap_pin_cop(pTHX)
{
    I32 cxix = cxstack_ix;
    PERL_CONTEXT *ccstack = cxstack;
    PERL_SI *top_si = PL_curstackinfo;
    COP *site = NULL;

    while (1) {
        while (cxix < 0 && top_si && top_si->si_type != PERLSI_MAIN) {
            top_si = top_si->si_prev;
            if (top_si == NULL)
                return site ? site : PL_curcop;
            ccstack = top_si->si_cxstack;
            cxix = top_si->si_cxix;
        }
        if (top_si == NULL || cxix < 0
            || (cxix == 0 && !top_si->si_prev)) {
            return site ? site : PL_curcop;
        }
        {
            PERL_CONTEXT *cx = &ccstack[cxix];
            if (CxTYPE(cx) != CXt_SUB) {
                cxix--;
                continue;
            }
            if (product_cv_is_debugger(aTHX_ cx->blk_sub.cv)) {
                if (cx->blk_oldcop
                    && !product_is_debugger_file(
                            OutCopFILE((COP *)cx->blk_oldcop))) {
                    site = (COP *)cx->blk_oldcop;
                }
                cxix--;
                continue;
            }
            if (site != NULL)
                return site;
            if (cx->blk_oldcop)
                return (COP *)cx->blk_oldcop;
            return PL_curcop;
        }
    }
}

static void
product_fill_block_sub(pTHX_ COP *pin, UV exec_line, UV *bl_out, UV *sl_out)
{
    product_pin_cop = (pin != NULL) ? pin : PL_curcop;
    product_last_executed_line = (unsigned int)exec_line;
    product_last_block_line = 0;
    product_last_sub_line = 0;
    (void)product_visit_contexts(aTHX_ ~(UV)0, &product_check_context);
    if (!product_last_block_line)
        product_last_block_line = (unsigned int)(exec_line ? exec_line : 1);
    if (!product_last_sub_line)
        product_last_sub_line = (unsigned int)(exec_line ? exec_line : 1);
    if (bl_out)
        *bl_out = (UV)product_last_block_line;
    if (sl_out)
        *sl_out = (UV)product_last_sub_line;
}

void
product_emit_time_block_for_cop(pTHX_ COP *cop)
{
    UV fid;
    UV line;
    UV bl;
    UV sl;
    const char *file;
    SV *file_sv;

    if (product_sink == NULL || cop == NULL)
        return;
    file = OutCopFILE(cop);
    line = (UV)CopLINE(cop);
    if (line == 0)
        line = 1;
    (void)product_close_last_site();
    file_sv = newSVpv(file ? file : "-", 0);
    fid = product_fid_for_filename(aTHX_ file_sv);
    SvREFCNT_dec(file_sv);
    product_fill_block_sub(aTHX_ cop, line, &bl, &sl);
    /* Same last-site clock as TIME_LINE (PR-8); first hit seeds, overflow skips. */
    (void)product_emit_attributed_time_block((nytp_fid)fid, (nytp_line)line,
                                             (nytp_line)bl, (nytp_line)sl);
}

/* TIME_LINE from COP without a Perl SV / caller() (default stmts=1).
 * Only installed when stmts=1; do not get_sv PRODUCT_STMTS on this path. */
void
product_emit_time_line_for_cop(pTHX_ COP *cop)
{
    const char *file;
    UV line;
    UV fid;

    if (product_sink == NULL || cop == NULL)
        return;
    /* Close first so fid lookup is not charged to the previous line. */
    (void)product_close_last_site();
    file = OutCopFILE(cop);
    line = (UV)CopLINE(cop);
    if (line == 0)
        line = 1;
    fid = product_fid_for_file_ptr(aTHX_ file);
    (void)product_emit_attributed_time_line((nytp_fid)fid, (nytp_line)line);
}

static OP *
pp_product_stmt(pTHX)
{
    Perl_ppaddr_t orig = NULL;
    OP *ret;
    U16 type = PL_op ? PL_op->op_type : 0;

    if (type == OP_NEXTSTATE)
        orig = product_orig_pp_nextstate;
    else if (type == OP_DBSTATE)
        orig = product_orig_pp_dbstate;
    else if (type == OP_UNSTACK)
        orig = product_orig_pp_unstack;
#ifdef OP_LEAVELOOP
    else if (type == OP_LEAVELOOP)
        orig = product_orig_pp_leaveloop;
#endif
    else
        orig = product_orig_pp_dbstate;
    /* UNSTACK/LEAVELOOP: emit the loop statement (last stmt COP), then orig.
     * NEXTSTATE/DBSTATE: orig first (sets PL_curcop), then emit + remember. */
    if (type == OP_UNSTACK
#ifdef OP_LEAVELOOP
        || type == OP_LEAVELOOP
#endif
    ) {
        /* Unstack belongs to the current COP (the for-modifier), not the
         * callee's last statement (that would re-attribute leaf line 6). */
        if (product_sink != NULL)
            product_emit_time_block_for_cop(aTHX_ PL_curcop);
        if (type == OP_UNSTACK) {
            product_unstack_since_stmt = 1;
            product_unstack_cop = PL_curcop;
        }
        return orig ? orig(aTHX) : NORMAL;
    }
    ret = orig ? orig(aTHX) : NORMAL;
    if (product_sink != NULL) {
        /* After a for-modifier, 6.15 writes last_executed again when the
         * next COP is entered. Only replay if last stmt is still the loop
         * line (not a callee that returned into this unstack). */
        if (product_unstack_since_stmt && product_last_stmt_cop != NULL
            && product_unstack_cop != NULL
            && product_cop_in_same_file(product_last_stmt_cop,
                                        product_unstack_cop)
            && CopLINE(product_last_stmt_cop)
                   == CopLINE(product_unstack_cop)) {
            product_emit_time_block_for_cop(aTHX_ product_last_stmt_cop);
        }
        product_unstack_since_stmt = 0;
        product_unstack_cop = NULL;
        product_emit_time_block_for_cop(aTHX_ PL_curcop);
        product_last_stmt_cop = PL_curcop;
    }
    return ret;
}

static int
product_install_stmt_ops(pTHX)
{
    PERL_UNUSED_CONTEXT;
    if (product_stmt_ops_installed)
        return 0;
    product_orig_pp_dbstate = PL_ppaddr[OP_DBSTATE];
    product_orig_pp_nextstate = PL_ppaddr[OP_NEXTSTATE];
    product_orig_pp_unstack = PL_ppaddr[OP_UNSTACK];
    PL_ppaddr[OP_DBSTATE] = pp_product_stmt;
    PL_ppaddr[OP_NEXTSTATE] = pp_product_stmt;
    PL_ppaddr[OP_UNSTACK] = pp_product_stmt;
#ifdef OP_LEAVELOOP
    product_orig_pp_leaveloop = PL_ppaddr[OP_LEAVELOOP];
    PL_ppaddr[OP_LEAVELOOP] = pp_product_stmt;
#endif
    product_stmt_ops_installed = 1;
    return 0;
}

/* Default stmts=1: TIME_LINE on OP_DBSTATE only (same sites as Perl DB::DB).
 * Do not hook NEXTSTATE/UNSTACK here — that is the blocks=1 780 path.
 * Install at file= enable so later compile copies our op_ppaddr; stay
 * inactive until INIT so BEGIN/use do not emit (same as $DB::single=0). */
static int product_dbstate_line_installed = 0;
static int product_timeline_active = 0;
static Perl_ppaddr_t product_orig_pp_dbstate_line = NULL;

static OP *
pp_product_dbstate_line(pTHX)
{
    OP *ret;

    ret = product_orig_pp_dbstate_line ? product_orig_pp_dbstate_line(aTHX)
                                       : NORMAL;
    if (product_sink != NULL && product_timeline_active)
        product_emit_time_line_for_cop(aTHX_ PL_curcop);
    return ret;
}

static int
product_install_dbstate_timeline(pTHX)
{
    PERL_UNUSED_CONTEXT;
    /* blocks=1 already owns OP_DBSTATE for TIME_BLOCK. Do not steal it. */
    if (product_stmt_ops_installed)
        return 1;
    if (product_dbstate_line_installed)
        return 0;
    product_orig_pp_dbstate_line = PL_ppaddr[OP_DBSTATE];
    PL_ppaddr[OP_DBSTATE] = pp_product_dbstate_line;
    product_dbstate_line_installed = 1;
    return 0;
}

static int
product_activate_dbstate_timeline(pTHX)
{
    PERL_UNUSED_CONTEXT;
    if (!product_dbstate_line_installed)
        return 1;
    product_timeline_active = 1;
    return 0;
}

/* Thin slowops: OP_PRINT + OP_MATCH only (KD-26/35). Not full slowops.h. */
static int product_slowops_installed = 0;
static int product_slowops_full_installed = 0;
static Perl_ppaddr_t product_orig_pp_print = NULL;
static Perl_ppaddr_t product_orig_pp_match = NULL;
static Perl_ppaddr_t product_ppaddr_orig[OP_max];
static int product_ppaddr_orig_saved = 0;
static int product_in_slowop = 0;

static void
product_save_ppaddr_orig(pTHX)
{
    PERL_UNUSED_CONTEXT;
    if (product_ppaddr_orig_saved)
        return;
    Copy(PL_ppaddr, product_ppaddr_orig, OP_max, Perl_ppaddr_t);
    product_ppaddr_orig_saved = 1;
}

static OP *pp_slowop_profiler(pTHX);

IV
product_opt_calls(pTHX)
{
    SV *sv = get_sv("Devel::NYTProfM::PRODUCT_CALLS", 0);
    if (sv != NULL && SvOK(sv))
        return SvIV(sv);
    return 1;
}

IV
product_opt_stmts(pTHX)
{
    SV *sv = get_sv("Devel::NYTProfM::PRODUCT_STMTS", 0);
    if (sv != NULL && SvOK(sv))
        return SvIV(sv);
    return 1;
}

IV
product_opt_blocks(pTHX)
{
    SV *sv = get_sv("Devel::NYTProfM::PRODUCT_BLOCKS", 0);
    if (sv != NULL && SvOK(sv))
        return SvIV(sv);
    return 0;
}

static IV
product_opt_slowops(pTHX)
{
    SV *sv = get_sv("Devel::NYTProfM::PRODUCT_SLOWOPS", 0);
    if (sv != NULL && SvOK(sv))
        return SvIV(sv);
    return 2;
}

static void
product_fill_cv_name(pTHX_ CV *cv, char *buf, size_t buflen)
{
    GV *gv;
    HV *stash;
    const char *pkg;
    const char *name;

    if (buf == NULL || buflen == 0)
        return;
    buf[0] = '\0';
    if (cv == NULL) {
        (void)snprintf(buf, buflen, "main::RUNTIME");
        return;
    }
    gv = CvGV(cv);
    if (gv != NULL && !isGV_with_GP((SV *)gv)) {
        gv = NULL;
    }
    stash = CvSTASH(cv);
    pkg = (stash != NULL && HvNAME(stash)) ? HvNAME(stash) : "main";
    name = (gv != NULL && GvNAME(gv)) ? GvNAME(gv) : "?";
    (void)snprintf(buf, buflen, "%s::%s", pkg ? pkg : "main", name);
}

static void
product_fill_perl_caller(pTHX_ char *buf, size_t buflen)
{
    I32 i;

    (void)snprintf(buf, buflen, "main::RUNTIME");
    for (i = cxstack_ix; i >= 0; i--) {
        PERL_CONTEXT *cx = &cxstack[i];
        CV *cv;
        if (CxTYPE(cx) != CXt_SUB)
            continue;
        cv = cx->blk_sub.cv;
        if (cv == NULL)
            continue;
        if (PL_debstash != NULL && CvSTASH(cv) == PL_debstash)
            continue;
        product_fill_cv_name(aTHX_ cv, buf, buflen);
        return;
    }
}

static void
product_fill_slowop_name(pTHX_ U16 type, char *buf, size_t buflen)
{
    const char *pkg = CopSTASHPV(PL_curcop);
    const char *opn = (type < OP_max && PL_op_name[type]) ? PL_op_name[type]
                                                          : "unknown";
    if (pkg == NULL || pkg[0] == '\0')
        pkg = "main";
    (void)snprintf(buf, buflen, "%s::CORE:%s", pkg, opn);
}

/* One emit path so wrap mailbox and opcode credit stay in sync. */
static OP *
product_profile_one_slowop(pTHX_ Perl_ppaddr_t orig, U16 type)
{
    IV calls;
    char name[256];
    char caller[256];
    UV fid = 1;
    UV line = 1;
    OP *ret;
    nytp_ticks t0 = 0;
    nytp_ticks now = 0;
    nytp_ticks incl = 0;
    double incl_nv;
    nytp_status st;

    if (orig == NULL)
        return NORMAL;
    if (product_in_slowop || product_sink == NULL)
        return orig(aTHX);

    calls = product_opt_calls(aTHX);
    if (calls < 1)
        return orig(aTHX);

    product_in_slowop = 1;
    product_fill_slowop_name(aTHX_ type, name, sizeof(name));
    product_fill_perl_caller(aTHX_ caller, sizeof(caller));
    if (PL_curcop != NULL) {
        const char *file = OutCopFILE(PL_curcop);
        SV *file_sv = newSVpv(file ? file : "-", 0);
        fid = product_fid_for_filename(aTHX_ file_sv);
        SvREFCNT_dec(file_sv);
        line = (UV)CopLINE(PL_curcop);
        if (line == 0)
            line = 1;
    }
    if (calls >= 2)
        (void)nytp_emit_sub_entry(product_sink, (nytp_fid)fid, (nytp_line)line);

    /* Same nytp_clock_now as last-site / DB::sub — not a second clock. */
    st = nytp_clock_now(&t0);
    ret = orig(aTHX);
    if (st == NYTP_OK && nytp_clock_now(&now) == NYTP_OK) {
        incl = (now >= t0) ? (now - t0) : 0;
    }
    incl_nv = (double)incl;
    (void)nytp_emit_sub_return(product_sink, (nytp_depth)1, incl_nv,
                               incl_nv, nytp_sv_cstr(name));
    if (product_callers_add((nytp_fid)fid, (nytp_line)line, 1U, incl_nv,
                            incl_nv, 0.0, 0U, name, caller)
        != NYTP_OK) {
        croak("NYTProfM: SUB_CALLERS aggregate overflow");
    }
    /* Opcode credits the current subr_entry; wrap still uses mailbox. */
    product_credit_child_excl(incl_nv);
    product_in_slowop = 0;
    return ret;
}

static OP *
pp_product_slowop(pTHX)
{
    Perl_ppaddr_t orig = NULL;
    U16 type = PL_op ? PL_op->op_type : 0;
    IV slowops;

    if (type == OP_PRINT)
        orig = product_orig_pp_print;
    else if (type == OP_MATCH)
        orig = product_orig_pp_match;
    else
        orig = product_orig_pp_print;

    if (orig == NULL)
        return NORMAL;

    slowops = product_opt_slowops(aTHX);
    if (slowops != 2 && slowops != 3)
        return orig(aTHX);
    return product_profile_one_slowop(aTHX_ orig, type);
}

/* Thin orig + nytp_emit_* + product_credit_child_excl; not 6.15 savestack. */
static OP *
pp_slowop_profiler(pTHX)
{
    U16 type = PL_op ? PL_op->op_type : 0;
    Perl_ppaddr_t orig = NULL;
    IV slowops;
    OP *ret;

    if (type < OP_max && product_ppaddr_orig_saved)
        orig = product_ppaddr_orig[type];
    if (orig == NULL || orig == pp_slowop_profiler)
        return NORMAL;

    slowops = product_opt_slowops(aTHX);
    /* 2 = 6.15 default (full table, pkg::CORE:op). 3/full is the same table. */
    if (slowops != 2 && slowops != 3)
        return orig(aTHX);
    /* Restore product_in_slowop if orig() dies. */
    ENTER;
    SAVEINT(product_in_slowop);
    ret = product_profile_one_slowop(aTHX_ orig, type);
    LEAVE;
    return ret;
}

static int
product_install_slowops(pTHX)
{
    PERL_UNUSED_CONTEXT;
    product_save_ppaddr_orig(aTHX);
    if (product_slowops_installed || product_slowops_full_installed)
        return 0;
    product_orig_pp_print = product_ppaddr_orig[OP_PRINT];
    PL_ppaddr[OP_PRINT] = pp_product_slowop;
    product_orig_pp_match = product_ppaddr_orig[OP_MATCH];
    PL_ppaddr[OP_MATCH] = pp_product_slowop;
    product_slowops_installed = 1;
    return 0;
}

static int
product_install_slowops_full(pTHX)
{
    PERL_UNUSED_CONTEXT;
    product_save_ppaddr_orig(aTHX);
    if (product_slowops_full_installed)
        return 0;
    /* Pin table: default slowops=2 and explicit 3/full. */
#include "slowops.h"
    product_slowops_full_installed = 1;
    product_slowops_installed = 1;
    return 0;
}

static int product_rebind_match_hits = 0;

static void
product_rebind_op(pTHX_ OP *o)
{
    OP *kid;

    PERL_UNUSED_CONTEXT;
    if (o == NULL)
        return;
    if (product_slowops_full_installed) {
        if (o->op_type < OP_max
            && PL_ppaddr[o->op_type] == pp_slowop_profiler) {
            o->op_ppaddr = pp_slowop_profiler;
            if (o->op_type == OP_MATCH)
                product_rebind_match_hits++;
        }
    } else if (o->op_type == OP_PRINT) {
        o->op_ppaddr = pp_product_slowop;
    } else if (o->op_type == OP_MATCH) {
        o->op_ppaddr = pp_product_slowop;
        product_rebind_match_hits++;
    }
    if (o->op_flags & OPf_KIDS) {
        for (kid = cUNOPx(o)->op_first; kid != NULL; kid = OpSIBLING(kid))
            product_rebind_op(aTHX_ kid);
    }
}

static void
product_rebind_cv(pTHX_ CV *cv)
{
    OP *o;
    int n;

    if (cv == NULL || SvTYPE((SV *)cv) != SVt_PVCV)
        return;
    /* CvROOT/CvXSUB share a union. Walking an XSUB as an OP* is a SEGV. */
    if (CvISXSUB(cv))
        return;
    if (CvROOT(cv))
        product_rebind_op(aTHX_ CvROOT(cv));
    /* Execution chain (ck_match may leave MATCH off the kid walk). */
    o = CvSTART(cv);
    for (n = 0; o != NULL && n < 100000; n++, o = o->op_next) {
        if (product_slowops_full_installed) {
            if (o->op_type < OP_max
                && PL_ppaddr[o->op_type] == pp_slowop_profiler) {
                o->op_ppaddr = pp_slowop_profiler;
                if (o->op_type == OP_MATCH)
                    product_rebind_match_hits++;
            }
        } else if (o->op_type == OP_PRINT) {
            o->op_ppaddr = pp_product_slowop;
        } else if (o->op_type == OP_MATCH) {
            o->op_ppaddr = pp_product_slowop;
            product_rebind_match_hits++;
        }
    }
}

static int
product_rebind_stash_slowops(pTHX_ const char *name)
{
    HV *stash;
    HE *he;
    int cvs = 0;

    product_rebind_match_hits = 0;
    if (name == NULL || name[0] == '\0')
        return 0;
    stash = gv_stashpv(name, 0);
    if (stash == NULL)
        return -1;
    hv_iterinit(stash);
    while ((he = hv_iternext(stash)) != NULL) {
        CV *cv = product_stash_val_cv(aTHX_ HeVAL(he));
        if (cv == NULL)
            continue;
        cvs++;
        product_rebind_cv(aTHX_ cv);
    }
    (void)cvs;
    return 0;
}

MODULE = Devel::NYTProfM  PACKAGE = DB

PROTOTYPES: DISABLE

int
init_profiler()
    CODE:
        /* G03a: hold in-memory v5 sink; never a path, never nytprof.out. */
        {
            nytp_status st = nytp_product_sink_hold(NULL, 0, 0);
            if (st != NYTP_OK || product_sink == NULL) {
                croak("DB::init_profiler: nytp_v5_sink_create(NULL) failed");
            }
        }
        RETVAL = 1;
    OUTPUT:
        RETVAL

int
enable_sink(path, compress_level = 0, durable = 0)
    const char *path
    int compress_level
    int durable
    CODE:
        if (path == NULL || path[0] == '\0') {
            croak("DB::enable_sink requires a non-empty path");
        }
        {
            nytp_status st = nytp_product_sink_hold(path, compress_level, durable);
            if (st != NYTP_OK || product_sink == NULL) {
                croak("DB::enable_sink: nytp_v5_sink_create(%s) failed", path);
            }
            product_last_site_reset();
            RETVAL = (int)product_emit_header_and_pid_start(aTHX);
        }
    OUTPUT:
        RETVAL

int
durable_seal_now()
    CODE:
        RETVAL = (int)product_durable_publish();
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
            product_fid_reset(aTHX);
            product_wrap_reset();
            product_callers_reset();
            product_sink = nytp_v6_sink_create(path);
            if (product_sink == NULL) {
                croak("DB::enable_sink_v6: nytp_v6_sink_create(%s) failed",
                      path);
            }
            product_last_site_reset();
            RETVAL = (int)product_emit_header_and_pid_start(aTHX);
        }
#else
        croak("format=v6 requires v6-enabled build (install v6_collect "
              "package or rebuild with --with v6_collect)");
        RETVAL = (int)NYTP_ERR_UNSUPPORTED;
#endif
    OUTPUT:
        RETVAL

NV
take_pending_child_excl()
    CODE:
        RETVAL = product_take_pending_child_excl();
    OUTPUT:
        RETVAL

UV
clock_now_ticks()
    PREINIT:
        nytp_ticks ticks = 0;
        nytp_status st;
    CODE:
        st = nytp_clock_now(&ticks);
        if (st != NYTP_OK) {
            croak("DB::clock_now_ticks: nytp_clock_now failed (status=%d)",
                  (int)st);
        }
        if (ticks < 0) {
            croak("DB::clock_now_ticks: negative ticks");
        }
        RETVAL = (UV)ticks;
    OUTPUT:
        RETVAL

int
wrap_push(called)
    SV *called
    PREINIT:
        product_wrap_frame *fr;
        COP *pin;
        const char *file;
        const char *called_pv;
        STRLEN called_len = 0;
        nytp_ticks ticks = 0;
        nytp_status st;
        double pending;
        IV calls;
    CODE:
        if (product_entersub_emit_enabled())
            croak("NYTProfM: opcode entersub and wrap would both emit");
        if (product_wrap_sp >= product_wrap_cap)
            product_wrap_grow(aTHX);
        pending = product_take_pending_child_excl();
        if (product_wrap_sp > 0)
            product_wrap_stack[product_wrap_sp - 1].child_excl += pending;
        pin = product_wrap_pin_cop(aTHX);
        file = pin ? OutCopFILE(pin) : NULL;
        fr = &product_wrap_stack[product_wrap_sp];
        fr->line = pin ? (UV)CopLINE(pin) : 1;
        if (fr->line == 0)
            fr->line = 1;
        fr->fid = product_fid_for_file_ptr(aTHX_ file);
        fr->child_excl = 0.0;
        if (called != NULL && SvOK(called))
            called_pv = SvPVbyte(called, called_len);
        else
            called_pv = NULL;
        product_wrap_copy_name(fr->called, sizeof(fr->called), called_pv);
        if (product_wrap_sp > 0) {
            product_wrap_copy_name(fr->caller, sizeof(fr->caller),
                                   product_wrap_stack[product_wrap_sp - 1].called);
        } else {
            product_wrap_copy_name(fr->caller, sizeof(fr->caller),
                                   "main::RUNTIME");
        }
        st = nytp_clock_now(&ticks);
        if (st != NYTP_OK) {
            croak("DB::wrap_push: nytp_clock_now failed (status=%d)", (int)st);
        }
        if (ticks < 0)
            croak("DB::wrap_push: negative ticks");
        fr->t0 = ticks;
        product_wrap_sp++;
        calls = product_opt_calls(aTHX);
        if (calls >= 2 && product_sink != NULL) {
            (void)nytp_emit_sub_entry(product_sink, (nytp_fid)fr->fid,
                                      (nytp_line)fr->line);
        }
        RETVAL = 0;
    OUTPUT:
        RETVAL

int
wrap_pop()
    PREINIT:
        product_wrap_frame *fr;
        nytp_ticks now = 0;
        nytp_ticks incl_ticks;
        nytp_status st;
        double pending;
        double incl;
        double excl;
        int depth;
    CODE:
        if (product_wrap_sp <= 0) {
            RETVAL = 0;
        } else {
            pending = product_take_pending_child_excl();
            st = nytp_clock_now(&now);
            if (st != NYTP_OK) {
                croak("DB::wrap_pop: nytp_clock_now failed (status=%d)",
                      (int)st);
            }
            if (now < 0)
                croak("DB::wrap_pop: negative ticks");
            fr = &product_wrap_stack[product_wrap_sp - 1];
            fr->child_excl += pending;
            incl_ticks = (now >= fr->t0) ? (now - fr->t0) : 0;
            incl = (double)incl_ticks;
            excl = incl - fr->child_excl;
            if (excl < 0.0)
                excl = 0.0;
            depth = product_wrap_sp;
            if (product_wrap_sp > 1)
                product_wrap_stack[product_wrap_sp - 2].child_excl += incl;
            product_wrap_sp--;
            if (product_sink != NULL) {
                (void)nytp_emit_sub_return(product_sink, (nytp_depth)depth,
                                           incl, excl,
                                           nytp_sv_cstr(fr->called));
                if (product_callers_add((nytp_fid)fr->fid, (nytp_line)fr->line,
                                        1U, incl, excl, 0.0, 0U, fr->called,
                                        fr->caller)
                    != NYTP_OK) {
                    croak("NYTProfM: SUB_CALLERS aggregate overflow");
                }
            }
            RETVAL = 0;
        }
    OUTPUT:
        RETVAL

int
emit_attributed_time_line(fid, line)
    UV fid
    UV line
    CODE:
        RETVAL = (int)product_emit_attributed_time_line((nytp_fid)fid,
                                                        (nytp_line)line);
    OUTPUT:
        RETVAL

int
flush_last_site()
    CODE:
        RETVAL = (int)product_flush_last_site();
    OUTPUT:
        RETVAL

int
set_savesrc(on)
    int on
    CODE:
        product_savesrc = on ? 1 : 0;
        product_apply_savesrc_flags(aTHX);
        RETVAL = product_savesrc;
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

UV
fid_for_filename(path)
    SV *path
    CODE:
        RETVAL = product_fid_for_filename(aTHX_ path);
    OUTPUT:
        RETVAL

void
block_and_sub_lines()
    PREINIT:
        UV bl = 1;
        UV sl = 1;
        COP *pin;
        UV line;
    PPCODE:
        pin = product_db_pin_cop(aTHX);
        line = pin ? (UV)CopLINE(pin) : 1;
        if (line == 0)
            line = 1;
        product_fill_block_sub(aTHX_ pin, line, &bl, &sl);
        EXTEND(SP, 2);
        PUSHs(sv_2mortal(newSVuv(bl)));
        PUSHs(sv_2mortal(newSVuv(sl)));

int
install_product_stmt_ops()
    CODE:
        RETVAL = product_install_stmt_ops(aTHX);
    OUTPUT:
        RETVAL

int
install_product_dbstate_timeline()
    CODE:
        RETVAL = product_install_dbstate_timeline(aTHX);
    OUTPUT:
        RETVAL

int
activate_product_dbstate_timeline()
    CODE:
        RETVAL = product_activate_dbstate_timeline(aTHX);
    OUTPUT:
        RETVAL

int
install_product_slowops()
    CODE:
        RETVAL = product_install_slowops(aTHX);
    OUTPUT:
        RETVAL

int
install_product_slowops_full()
    CODE:
        RETVAL = product_install_slowops_full(aTHX);
    OUTPUT:
        RETVAL

int
install_product_entersub()
    CODE:
        RETVAL = product_install_entersub(aTHX);
    OUTPUT:
        RETVAL

int
uninstall_product_entersub()
    CODE:
        RETVAL = product_uninstall_entersub(aTHX);
    OUTPUT:
        RETVAL

int
entersub_is_installed()
    CODE:
        RETVAL = product_entersub_is_installed();
    OUTPUT:
        RETVAL

int
entersub_goto_is_installed()
    CODE:
        RETVAL = product_goto_is_installed();
    OUTPUT:
        RETVAL

int
entersub_emit_enabled()
    CODE:
        RETVAL = product_entersub_emit_enabled();
    OUTPUT:
        RETVAL

int
entersub_set_emit_enabled(on)
    int on
    CODE:
        product_entersub_set_emit_enabled(on);
        RETVAL = product_entersub_emit_enabled();
    OUTPUT:
        RETVAL

int
install_product_leave()
    CODE:
        RETVAL = product_install_leave(aTHX);
    OUTPUT:
        RETVAL

int
uninstall_product_leave()
    CODE:
        RETVAL = product_uninstall_leave(aTHX);
    OUTPUT:
        RETVAL

int
leave_is_installed()
    CODE:
        RETVAL = product_leave_is_installed();
    OUTPUT:
        RETVAL

int
leave_emit_enabled()
    CODE:
        RETVAL = product_leave_emit_enabled();
    OUTPUT:
        RETVAL

int
leave_set_emit_enabled(on)
    int on
    CODE:
        product_leave_set_emit_enabled(on);
        RETVAL = product_leave_emit_enabled();
    OUTPUT:
        RETVAL

int
rebind_stash_slowops(name)
    const char *name
    CODE:
        RETVAL = product_rebind_stash_slowops(aTHX_ name);
    OUTPUT:
        RETVAL

int
nodebug_stash(name)
    const char *name
    CODE:
        RETVAL = product_nodebug_stash(aTHX_ name);
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
            RETVAL = nytp_v5_sink_is_deflating(product_v5_sink());
        }
    OUTPUT:
        RETVAL

int
finish_profiler()
    CODE:
        {
            /* KD-P / KD-N / A.5: flush last-site TIME_LINE/TIME_BLOCK
             * while ACTIVE, then finalize metadata, then PID_END. */
            nytp_status st = product_flush_last_site();
            if (st != NYTP_OK && st != NYTP_ERR_NULL) {
                /* Still close the sink so the file is not left incomplete. */
            }
            if (product_sink != NULL) {
                (void)product_callers_flush(product_sink);
                (void)nytp_sink_begin_finalize(product_sink);
                if (product_savesrc) {
                    product_emit_src_lines(aTHX);
                }
                product_emit_sub_infos(aTHX);
                (void)nytp_emit_pid_end(product_sink, (nytp_pid)getpid(), 0.0);
            }
            nytp_product_sink_drop();
            product_fid_reset(aTHX);
            product_wrap_reset();
            product_callers_reset();
            RETVAL = 1;
        }
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

SV *
name_cv(cvsv)
    SV *cvsv
    CODE:
        {
            char buf[256];
            CV *cv = NULL;
            if (cvsv != NULL && SvROK(cvsv)
                && SvTYPE(SvRV(cvsv)) == SVt_PVCV) {
                cv = (CV *)SvRV(cvsv);
            }
            else if (cvsv != NULL && SvTYPE(cvsv) == SVt_PVCV) {
                cv = (CV *)cvsv;
            }
            product_fill_cv_name(aTHX_ cv, buf, sizeof(buf));
            RETVAL = newSVpv(buf, 0);
        }
    OUTPUT:
        RETVAL

BOOT:
    /* Full table at load so BEGIN/use ops copy pp_slowop_profiler.
     * Emit stays off until file= sink + INIT. */ 
    product_install_slowops_full(aTHX);
