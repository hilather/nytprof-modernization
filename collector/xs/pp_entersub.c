/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * Graft of Devel::NYTProf 6.15 pp_entersub_profiler / subr_entry_* onto
 * nytp_emit_* (DI-03 E1a). Pin source: baseline/6.15 archives NYTProf.xs
 * ~1959–2928 (read-only extract). Not a wrap-stack; savestack + destructor
 * are required for XS exceptions.
 *
 * Product adaptations (binding): nytp_clock_now; product_fid_for_file_ptr;
 * emit SUB_RETURN in ticks at return; SUB_CALLERS aggregated in C
 * (product_callers_add) and flushed at finish; no Perl callers HV;
 * last-site hook overhead subtracted from incl (KD-E13
 * superseded); keep cumulative_subr_ticks (g14 remainder);
 * wrap recursion (full incl/excl, reci=0); skip DB::*
 * and Devel::NYTProfM. E2 ports pin OP_GOTO (goto &CV keeps the original
 * caller and the goto site's fid:line). Wrap-list goto stays wrap=1 only.
 */
#define PERL_NO_GET_CONTEXT
#include "EXTERN.h"
#include "perl.h"
#include "XSUB.h"

#include "nytprof_pp.h"

#include <stdio.h>
#include <string.h>

#ifndef OutCopFILE
#define OutCopFILE CopFILE
#endif

/* FileHandle-only pin fields omitted; initial_overhead_ticks
 * snapshots product_overhead_ticks at entry. */
typedef struct subr_entry_st subr_entry_t;
struct subr_entry_st {
    unsigned int already_counted;
    U32 subr_prof_depth;
    SSize_t prev_subr_entry_ix;
    nytp_ticks initial_call_ticks;
    NV initial_overhead_ticks;
    NV initial_subr_ticks;
    unsigned int caller_fid;
    int caller_line;
    const char *caller_subpkg_pv;
    SV *caller_subnam_sv;
    CV *called_cv;
    int called_cv_depth;
    const char *called_is_xs;
    const char *called_subpkg_pv;
    SV *called_subnam_sv;
};

static SSize_t product_subr_entry_ix = -1;
static NV product_cumulative_subr_ticks = 0.0;
static int product_entersub_installed = 0;
static int product_entersub_emit_on = 0;
static Perl_ppaddr_t product_orig_pp_entersub = NULL;
static Perl_ppaddr_t product_orig_pp_goto = NULL;
#ifdef MULTIPLICITY
static PerlInterpreter *product_entersub_orig_my_perl = NULL;
#endif

#define product_subr_entry_ix_ptr(ix) \
    ((ix != -1) ? SSPTR(ix, subr_entry_t *) : NULL)

int
product_entersub_is_installed(void)
{
    return product_entersub_installed ? 1 : 0;
}

int
product_goto_is_installed(void)
{
    return (product_entersub_installed && product_orig_pp_goto != NULL)
               ? 1
               : 0;
}

static OP *
product_run_orig_op(pTHX_ OPCODE op_type)
{
    if (op_type == OP_GOTO) {
        if (product_orig_pp_goto == NULL)
            return NORMAL;
        return product_orig_pp_goto(aTHX);
    }
    if (product_orig_pp_entersub == NULL)
        return NORMAL;
    return product_orig_pp_entersub(aTHX);
}

int
product_entersub_emit_enabled(void)
{
    return (product_sink != NULL && product_entersub_installed
            && product_entersub_emit_on)
               ? 1
               : 0;
}

void
product_entersub_set_emit_enabled(int on)
{
    product_entersub_emit_on = on ? 1 : 0;
    if (on)
        product_cumulative_subr_ticks = 0.0;
}

void *
product_current_subr_entry(void)
{
    dTHX;
    if (product_subr_entry_ix == -1)
        return NULL;
    return product_subr_entry_ix_ptr(product_subr_entry_ix);
}

void
product_subr_add_child_incl(void *se, NV incl_nv)
{
    /* Slowop incl == excl. Fold into the 6.15 cumulative so parent
     * remainder stays incl − Σ child inclusive (g09). */
    PERL_UNUSED_ARG(se);
    if (incl_nv > 0.0)
        product_cumulative_subr_ticks += incl_nv;
}

void
product_credit_child_excl(NV incl_nv)
{
    if (product_entersub_is_installed() && product_current_subr_entry() != NULL)
        product_subr_add_child_incl(product_current_subr_entry(), incl_nv);
    else
        product_add_pending_child_excl(incl_nv);
}

static int
product_pkg_is_internal(const char *pkg)
{
    if (pkg == NULL)
        return 0;
    if (pkg[0] == 'D' && pkg[1] == 'B' && pkg[2] == '\0')
        return 1;
    if (strncmp(pkg, "Devel::NYTProfM", 15) == 0
        && (pkg[15] == '\0' || pkg[15] == ':'))
        return 1;
    return 0;
}

static int
product_compose_fqname(pTHX_ char *buf, size_t buflen, const char *pkg, SV *nam_sv)
{
    const char *name;
    STRLEN nlen = 0;
    int n;

    if (buf == NULL || buflen == 0)
        return -1;
    if (pkg == NULL || pkg[0] == '\0')
        pkg = "main";
    if (nam_sv != NULL && SvOK(nam_sv))
        name = SvPV(nam_sv, nlen);
    else {
        name = "(null)";
        nlen = 6;
    }
    n = snprintf(buf, buflen, "%s::%.*s", pkg, (int)nlen, name);
    if (n < 0 || (size_t)n >= buflen || (size_t)n >= NYTP_MAX_SUB_NAME_LEN)
        return -1;
    return n;
}

static CV *
product_resolve_sub_to_cv(pTHX_ SV *sv, GV **subname_gv_ptr)
{
    GV *dummy_gv;
    HV *stash;
    CV *cv;

    if (subname_gv_ptr == NULL)
        subname_gv_ptr = &dummy_gv;
    else
        *subname_gv_ptr = NULL;

    if (sv == NULL || sv == &PL_sv_yes)
        return NULL;

    if (SvGMAGICAL(sv))
        mg_get(sv);

    if (SvROK(sv)) {
        cv = (CV *)SvRV(sv);
        if (SvTYPE(cv) == SVt_PVCV)
            goto got_cv;
        return NULL;
    }

    switch (SvTYPE(sv)) {
    case SVt_PVCV:
        cv = (CV *)sv;
        break;
    case SVt_PVGV:
        if (!(isGV_with_GP(sv) && (cv = GvCVu((GV *)sv))))
            cv = sv_2cv(sv, &stash, subname_gv_ptr, FALSE);
        if (cv == NULL)
            return NULL;
        break;
    default:
        if (SvPOK(sv) || SvPOKp(sv)) {
            if (PL_op != NULL && (PL_op->op_private & HINT_STRICT_REFS))
                return NULL;
            cv = get_cv(SvPV_nolen(sv), TRUE);
            break;
        }
        return NULL;
    }

got_cv:
    if (cv != NULL && *subname_gv_ptr == NULL && CvGV(cv) != NULL
        && isGV_with_GP((SV *)CvGV(cv))) {
        *subname_gv_ptr = CvGV(cv);
    }
    return cv;
}

static CV *
product_current_cv(pTHX_ I32 ix, PERL_SI *si)
{
    PERL_CONTEXT *cx;

    if (si == NULL)
        si = PL_curstackinfo;
    if (ix < 0) {
        if (si->si_type != PERLSI_MAIN && si->si_prev != NULL)
            return product_current_cv(aTHX_ si->si_prev->si_cxix, si->si_prev);
        return NULL;
    }
    cx = &si->si_cxstack[ix];
    if (CxTYPE(cx) == CXt_SUB || CxTYPE(cx) == CXt_FORMAT)
        return cx->blk_sub.cv;
    if (CxTYPE(cx) == CXt_EVAL && !CxTRYBLOCK(cx))
        return product_current_cv(aTHX_ ix - 1, si);
    if (ix == 0 && si->si_type == PERLSI_MAIN)
        return PL_main_cv;
    if (ix > 0)
        return product_current_cv(aTHX_ ix - 1, si);
    if (si->si_type != PERLSI_MAIN && si->si_prev != NULL)
        return product_current_cv(aTHX_ si->si_prev->si_cxix, si->si_prev);
    return NULL;
}

static void
product_subr_entry_destroy(pTHX_ subr_entry_t *subr_entry)
{
    if (subr_entry == NULL)
        return;
    if (subr_entry->caller_subnam_sv) {
        sv_free(subr_entry->caller_subnam_sv);
        subr_entry->caller_subnam_sv = NULL;
    }
    if (subr_entry->called_subnam_sv) {
        sv_free(subr_entry->called_subnam_sv);
        subr_entry->called_subnam_sv = NULL;
    }
    if (subr_entry->prev_subr_entry_ix <= product_subr_entry_ix)
        product_subr_entry_ix = subr_entry->prev_subr_entry_ix;
}

static void
product_incr_sub_inclusive_time(pTHX_ subr_entry_t *subr_entry)
{
    int saved_errno = errno;
    char called_buf[NYTP_MAX_SUB_NAME_LEN];
    char caller_buf[NYTP_MAX_SUB_NAME_LEN];
    NV called_sub_ticks;
    NV incl_subr_ticks;
    NV excl_subr_ticks;
    nytp_ticks now = 0;
    nytp_status st;

    if (subr_entry == NULL)
        return;

    if (subr_entry->called_subnam_sv && !SvOK(subr_entry->called_subnam_sv))
        subr_entry->already_counted++;

    if (subr_entry->already_counted) {
        product_subr_entry_destroy(aTHX_ subr_entry);
        return;
    }
    subr_entry->already_counted++;

    called_sub_ticks =
        product_cumulative_subr_ticks - subr_entry->initial_subr_ticks;

    st = nytp_clock_now(&now);
    if (st != NYTP_OK || now < subr_entry->initial_call_ticks)
        incl_subr_ticks = 0.0;
    else
        incl_subr_ticks =
            (NV)(now - subr_entry->initial_call_ticks);

    /* Last-site close-to-seed gap is not statement time. */
    {
        NV overhead_ticks =
            product_overhead_ticks() - subr_entry->initial_overhead_ticks;
        if (overhead_ticks > 0.0)
            incl_subr_ticks -= overhead_ticks;
        if (incl_subr_ticks < 0.0)
            incl_subr_ticks = 0.0;
    }
    excl_subr_ticks = incl_subr_ticks - called_sub_ticks;
    if (excl_subr_ticks < 0.0)
        excl_subr_ticks = 0.0;

    if (product_compose_fqname(aTHX_ called_buf, sizeof(called_buf),
                               subr_entry->called_subpkg_pv,
                               subr_entry->called_subnam_sv)
        < 0) {
        croak("NYTProfM: called sub name exceeds NYTP_MAX_SUB_NAME_LEN");
    }
    if (product_compose_fqname(aTHX_ caller_buf, sizeof(caller_buf),
                               subr_entry->caller_subpkg_pv,
                               subr_entry->caller_subnam_sv)
        < 0) {
        croak("NYTProfM: caller sub name exceeds NYTP_MAX_SUB_NAME_LEN");
    }

    if (product_sink != NULL) {
        /* Ticks at return. Callers rows wait for finish (one emit per edge). */
        (void)nytp_emit_sub_return(product_sink,
                                   (nytp_depth)subr_entry->subr_prof_depth,
                                   incl_subr_ticks, excl_subr_ticks,
                                   nytp_sv_cstr(called_buf));
        if (product_callers_add((nytp_fid)subr_entry->caller_fid,
                                (nytp_line)(subr_entry->caller_line
                                                ? subr_entry->caller_line
                                                : 1),
                                1U, incl_subr_ticks, excl_subr_ticks, 0.0, 0U,
                                called_buf, caller_buf)
            != NYTP_OK) {
            croak("NYTProfM: SUB_CALLERS aggregate overflow");
        }
    }

    product_subr_entry_destroy(aTHX_ subr_entry);
    product_cumulative_subr_ticks += excl_subr_ticks;
    SETERRNO(saved_errno, 0);
}

static void
product_incr_sub_inclusive_time_ix(pTHX_ void *subr_entry_ix_void)
{
    SSize_t save_ix = (SSize_t)PTR2IV(subr_entry_ix_void);
    product_incr_sub_inclusive_time(aTHX_ product_subr_entry_ix_ptr(save_ix));
}

static SSize_t
product_subr_entry_setup(pTHX_ COP *prev_cop, subr_entry_t *tmpl,
                        OPCODE op_type, SV *subr_sv)
{
    int saved_errno = errno;
    subr_entry_t *subr_entry;
    SSize_t prev_ix;
    subr_entry_t *caller_se;
    const char *file;
    nytp_ticks t0 = 0;

    prev_ix = product_subr_entry_ix;
    product_subr_entry_ix = SSNEWa(sizeof(*subr_entry), MEM_ALIGNBYTES);
    if (product_subr_entry_ix <= prev_ix) {
        product_entersub_emit_on = 0;
        product_subr_entry_ix = prev_ix;
        SETERRNO(saved_errno, 0);
        return -1;
    }

    subr_entry = product_subr_entry_ix_ptr(product_subr_entry_ix);
    Zero(subr_entry, 1, subr_entry_t);
    subr_entry->prev_subr_entry_ix = prev_ix;
    caller_se = product_subr_entry_ix_ptr(prev_ix);
    subr_entry->subr_prof_depth =
        caller_se ? caller_se->subr_prof_depth + 1 : 1;

    if (nytp_clock_now(&t0) != NYTP_OK)
        t0 = 0;
    subr_entry->initial_call_ticks = t0;
    subr_entry->initial_overhead_ticks = product_overhead_ticks();
    subr_entry->initial_subr_ticks = product_cumulative_subr_ticks;

    if (op_type == OP_ENTERSUB || op_type == OP_GOTO) {
        GV *called_gv = NULL;
        subr_entry->called_cv =
            product_resolve_sub_to_cv(aTHX_ subr_sv, &called_gv);
        if (called_gv != NULL && GvSTASH(called_gv) != NULL) {
            subr_entry->called_subpkg_pv = HvNAME(GvSTASH(called_gv));
            subr_entry->called_subnam_sv = newSVpv(GvNAME(called_gv), 0);
        } else {
            /* Undef marker: incr ignores if still unnamed at return. */
            subr_entry->called_subnam_sv = newSV(0);
        }
        subr_entry->called_is_xs = NULL;
    }

    /* fid:line from prev_cop — GOTO passes the goto-site COP. */
    file = prev_cop ? OutCopFILE(prev_cop) : NULL;
    subr_entry->caller_fid = (unsigned int)product_fid_for_file_ptr(aTHX_ file);
    subr_entry->caller_line = prev_cop ? (int)CopLINE(prev_cop) : 1;
    if (subr_entry->caller_line <= 0)
        subr_entry->caller_line = 1;

    if (tmpl != NULL && tmpl->caller_subnam_sv != NULL) {
        /* OP_GOTO: keep the original caller; do not inherit jumper. */
        subr_entry->caller_subpkg_pv = tmpl->caller_subpkg_pv;
        subr_entry->caller_subnam_sv = SvREFCNT_inc(tmpl->caller_subnam_sv);
    } else if (caller_se != NULL && caller_se->called_subpkg_pv != NULL
        && caller_se->called_subnam_sv != NULL
        && SvOK(caller_se->called_subnam_sv)) {
        subr_entry->caller_subpkg_pv = caller_se->called_subpkg_pv;
        subr_entry->caller_subnam_sv =
            SvREFCNT_inc(caller_se->called_subnam_sv);
    } else {
        CV *caller_cv = product_current_cv(aTHX_ cxstack_ix, NULL);
        subr_entry->caller_subnam_sv = newSV(0);
        if (caller_cv == PL_main_cv || caller_cv == NULL) {
            subr_entry->caller_subpkg_pv = "main";
            sv_setpvs(subr_entry->caller_subnam_sv, "RUNTIME");
        } else {
            GV *gv = CvGV(caller_cv);
            HV *stash_hv = NULL;
            if (gv != NULL && (stash_hv = GvSTASH(gv)) != NULL) {
                subr_entry->caller_subpkg_pv = HvNAME(stash_hv);
                sv_setpvn(subr_entry->caller_subnam_sv, GvNAME(gv),
                          GvNAMELEN(gv));
            } else {
                subr_entry->caller_subpkg_pv = "main";
                sv_setpvs(subr_entry->caller_subnam_sv, "RUNTIME");
            }
        }
    }

    save_destructor_x(product_incr_sub_inclusive_time_ix,
                      INT2PTR(void *, (IV)product_subr_entry_ix));

    SETERRNO(saved_errno, 0);
    return product_subr_entry_ix;
}

static OP *
pp_product_entersub(pTHX)
{
    int saved_errno = errno;
    OP *op;
    COP *prev_cop = PL_curcop;
    OP *next_op = PL_op ? PL_op->op_next : NULL;
    OPCODE op_type = OP_ENTERSUB;
    CV *called_cv = NULL;
    dSP;
    SV *sub_sv;
    SSize_t this_ix;
    subr_entry_t *subr_entry;
    const char *is_xs = NULL;
    char *stash_name = NULL;

    if (PL_op == NULL)
        return NORMAL;

    /* Pin: pp_entersub can be called with op_type==0; treat as ENTERSUB.
     * OP_GOTO shares this hook; never run orig ENTERSUB for a goto. */
    op_type = ((opcode)PL_op->op_type == OP_GOTO) ? OP_GOTO : OP_ENTERSUB;
    if (op_type == OP_GOTO && product_orig_pp_goto == NULL)
        return NORMAL;
    if (op_type != OP_GOTO && product_orig_pp_entersub == NULL)
        return NORMAL;

#ifdef MULTIPLICITY
    if (product_entersub_orig_my_perl != NULL
        && my_perl != product_entersub_orig_my_perl)
        return product_run_orig_op(aTHX_ op_type);
#endif

    /* Install at file=; emit only after INIT (di02 27). */
    if (!product_entersub_emit_enabled())
        return product_run_orig_op(aTHX_ op_type);

    /* Fail closed: opcode + $^P 0x01 wrap would double SUB_RETURN. */
    if (PL_perldb & PERLDBf_SUB)
        croak("NYTProfM: opcode entersub and wrap would both emit");

    sub_sv = *SP;
    if (op_type == OP_ENTERSUB && sub_sv == &PL_sv_yes)
        return product_run_orig_op(aTHX_ op_type);

    /* Only profile goto &CV. Other gotos (label) run orig only.
     * Goto out of a sub whose entry was not profiled: orig only. */
    if (op_type == OP_GOTO
        && (!(SvROK(sub_sv) && SvTYPE(SvRV(sub_sv)) == SVt_PVCV)
            || product_subr_entry_ix == -1))
        return product_run_orig_op(aTHX_ op_type);

    if (op_type != OP_GOTO) {
        this_ix = product_subr_entry_setup(aTHX_ prev_cop, NULL, op_type,
                                           sub_sv);
        if (this_ix < 0)
            return product_run_orig_op(aTHX_ op_type);

        SETERRNO(saved_errno, 0);
        op = product_run_orig_op(aTHX_ op_type);
        saved_errno = errno;
    } else {
        /* goto &sub is return+call: copy current frame as template so
         * the goto'd sub keeps the original caller; fid:line is the
         * goto site (prev_cop). Port pin REFCNT_inc/mortalize (KD leak). */
        COP prev_cop_copy;
        subr_entry_t goto_subr_entry;
        subr_entry_t *src = product_subr_entry_ix_ptr(product_subr_entry_ix);

        if (src == NULL)
            return product_run_orig_op(aTHX_ op_type);

        prev_cop_copy = *prev_cop;
        Copy(src, &goto_subr_entry, 1, subr_entry_t);

        /* XXX if the goto op or goto'd xsub croaks then this'll leak */
        (void)SvREFCNT_inc(goto_subr_entry.caller_subnam_sv);
        (void)SvREFCNT_inc(goto_subr_entry.called_subnam_sv);
        (void)SvREFCNT_inc(sub_sv);

        called_cv = (CV *)SvRV(sub_sv);

        SETERRNO(saved_errno, 0);
        op = product_run_orig_op(aTHX_ op_type);
        saved_errno = errno;

        sv_2mortal(goto_subr_entry.caller_subnam_sv);
        sv_2mortal(goto_subr_entry.called_subnam_sv);
        this_ix = product_subr_entry_setup(aTHX_ &prev_cop_copy,
                                           &goto_subr_entry, op_type, sub_sv);
        SvREFCNT_dec(sub_sv);
        if (this_ix < 0)
            goto skip_sub_profile;
    }

    subr_entry = product_subr_entry_ix_ptr(this_ix);
    if (subr_entry == NULL)
        goto skip_sub_profile;

    if (subr_entry->already_counted)
        goto skip_sub_profile;

    if (op_type == OP_GOTO) {
        is_xs = (called_cv != NULL && CvISXSUB(called_cv)) ? "xsub" : NULL;
    } else if (op != next_op) {
        called_cv = cxstack[cxstack_ix].blk_sub.cv;
        is_xs = NULL;
    } else {
        GV *gv = NULL;
        called_cv = product_resolve_sub_to_cv(aTHX_ sub_sv, &gv);
        if (called_cv == NULL && gv != NULL && GvSTASH(gv) != NULL) {
            stash_name = HvNAME(GvSTASH(gv));
            sv_setpv(subr_entry->called_subnam_sv, GvNAME(gv));
        }
        is_xs = "xsub";
    }

    if (called_cv != NULL && CvGV(called_cv) != NULL) {
        GV *gv = CvGV(called_cv);
        if (SvTYPE(gv) == SVt_PVGV && GvSTASH(gv) != NULL) {
            stash_name = HvNAME(GvSTASH(gv));
            sv_setpv(subr_entry->called_subnam_sv, GvNAME(gv));
        }
    }

    if (subr_entry->called_subnam_sv != NULL
        && !SvOK(subr_entry->called_subnam_sv)) {
        if (called_cv == NULL) {
            stash_name = CopSTASHPV(PL_curcop);
            sv_setpvs(subr_entry->called_subnam_sv, "__UNKNOWN__");
        } else {
            HV *cstash = CvSTASH(called_cv);
            stash_name = (cstash != NULL) ? HvNAME(cstash) : "main";
            sv_setpvf(subr_entry->called_subnam_sv, "__UNKNOWN__[0x%p]",
                      (void *)called_cv);
        }
    }

    if (stash_name != NULL)
        subr_entry->called_subpkg_pv = stash_name;
    subr_entry->called_cv_depth =
        called_cv ? (int)CvDEPTH(called_cv) + (is_xs ? 1 : 0) : 0;
    subr_entry->called_cv = called_cv;
    subr_entry->called_is_xs = is_xs;

    if (product_pkg_is_internal(subr_entry->called_subpkg_pv)) {
        subr_entry->already_counted++;
        goto skip_sub_profile;
    }

    /* Name is known and not skipped. Match wrap: SUB_ENTRY after resolve. */
    if (product_opt_calls(aTHX) >= 2 && product_sink != NULL)
        (void)nytp_emit_sub_entry(product_sink,
                                  (nytp_fid)subr_entry->caller_fid,
                                  (nytp_line)subr_entry->caller_line);

    if (subr_entry->called_is_xs)
        product_incr_sub_inclusive_time(aTHX_ subr_entry);
    else
        save_destructor_x(product_incr_sub_inclusive_time_ix,
                          INT2PTR(void *, (IV)this_ix));

skip_sub_profile:
    SETERRNO(saved_errno, 0);
    return op;
}

int
product_install_entersub(pTHX)
{
    if (product_entersub_installed)
        return 0;
    product_orig_pp_entersub = PL_ppaddr[OP_ENTERSUB];
    product_orig_pp_goto = PL_ppaddr[OP_GOTO];
    PL_ppaddr[OP_ENTERSUB] = pp_product_entersub;
    PL_ppaddr[OP_GOTO] = pp_product_entersub;
    product_entersub_installed = 1;
#ifdef MULTIPLICITY
    product_entersub_orig_my_perl = my_perl;
#endif
    return 0;
}

int
product_uninstall_entersub(pTHX)
{
    if (!product_entersub_installed)
        return 0;
    if (product_orig_pp_entersub != NULL)
        PL_ppaddr[OP_ENTERSUB] = product_orig_pp_entersub;
    if (product_orig_pp_goto != NULL)
        PL_ppaddr[OP_GOTO] = product_orig_pp_goto;
    product_orig_pp_entersub = NULL;
    product_orig_pp_goto = NULL;
    product_entersub_installed = 0;
    product_entersub_emit_on = 0;
    return 0;
}
