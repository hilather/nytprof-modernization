/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * Graft of Devel::NYTProf 6.15 pp_leave_profiler / DB_leave onto
 * nytp_emit_discount + product last-site flush (DI-03 E3). Pin source:
 * baseline/6.15 archives NYTProf.xs ~1666–1728 and ~2940–2946
 * (read-only extract). Not a FileHandle writer.
 *
 * Product adaptations (binding): nytp_clock_now lives inside the last-site
 * helpers (do not add a second clock); nytp_emit_discount is the only
 * DISCOUNT writer; do not emit TIME_* here (flush/seed via existing
 * product_emit_attributed_*); install only when leave=1; UNSTACK/LEAVELOOP
 * stay on pp_product_stmt when PRODUCT_BLOCKS (KD-E14). Default leave=0.
 */
#define PERL_NO_GET_CONTEXT
#include "EXTERN.h"
#include "perl.h"
#include "XSUB.h"

#include "nytprof_pp.h"

#ifndef OutCopFILE
#define OutCopFILE CopFILE
#endif

static int product_leave_installed = 0;
static int product_leave_emit_on = 0;
#ifdef MULTIPLICITY
static PerlInterpreter *product_leave_orig_my_perl = NULL;
#endif

enum {
    PRODUCT_LEAVE_SLOT_LEAVESUB = 0,
#ifdef OP_LEAVESUBLV
    PRODUCT_LEAVE_SLOT_LEAVESUBLV,
#endif
    PRODUCT_LEAVE_SLOT_LEAVE,
#ifdef OP_LEAVELOOP
    PRODUCT_LEAVE_SLOT_LEAVELOOP,
#endif
#ifdef OP_LEAVEWRITE
    PRODUCT_LEAVE_SLOT_LEAVEWRITE,
#endif
#ifdef OP_LEAVEEVAL
    PRODUCT_LEAVE_SLOT_LEAVEEVAL,
#endif
#ifdef OP_LEAVETRY
    PRODUCT_LEAVE_SLOT_LEAVETRY,
#endif
    PRODUCT_LEAVE_SLOT_RETURN,
    PRODUCT_LEAVE_SLOT_UNSTACK,
    PRODUCT_LEAVE_SLOT_COUNT
};

static Perl_ppaddr_t product_leave_orig[PRODUCT_LEAVE_SLOT_COUNT];

static int
product_leave_slot_for(U16 type)
{
    switch (type) {
    case OP_LEAVESUB:
        return PRODUCT_LEAVE_SLOT_LEAVESUB;
#ifdef OP_LEAVESUBLV
    case OP_LEAVESUBLV:
        return PRODUCT_LEAVE_SLOT_LEAVESUBLV;
#endif
    case OP_LEAVE:
        return PRODUCT_LEAVE_SLOT_LEAVE;
#ifdef OP_LEAVELOOP
    case OP_LEAVELOOP:
        return PRODUCT_LEAVE_SLOT_LEAVELOOP;
#endif
#ifdef OP_LEAVEWRITE
    case OP_LEAVEWRITE:
        return PRODUCT_LEAVE_SLOT_LEAVEWRITE;
#endif
#ifdef OP_LEAVEEVAL
    case OP_LEAVEEVAL:
        return PRODUCT_LEAVE_SLOT_LEAVEEVAL;
#endif
#ifdef OP_LEAVETRY
    case OP_LEAVETRY:
        return PRODUCT_LEAVE_SLOT_LEAVETRY;
#endif
    case OP_RETURN:
        return PRODUCT_LEAVE_SLOT_RETURN;
    case OP_UNSTACK:
        return PRODUCT_LEAVE_SLOT_UNSTACK;
    default:
        return -1;
    }
}

int
product_leave_is_installed(void)
{
    return product_leave_installed ? 1 : 0;
}

int
product_leave_emit_enabled(void)
{
    return (product_sink != NULL && product_leave_installed
            && product_leave_emit_on)
               ? 1
               : 0;
}

void
product_leave_set_emit_enabled(int on)
{
    product_leave_emit_on = on ? 1 : 0;
}

/* 6.15 DB_leave: write previous statement via DB_stmt, then DISCOUNT so
 * the next TIME_* is a continuation (count must not increment). Product
 * attributed emit is that DB_stmt write-site (flush last + seed outer). */
static void
product_db_leave(pTHX_ OP *op)
{
    COP *cop;
    const char *file;
    UV line;
    UV fid;
    nytp_status st;

    PERL_UNUSED_ARG(op);
    if (!product_leave_emit_enabled())
        return;
#ifdef MULTIPLICITY
    if (product_leave_orig_my_perl != NULL
        && my_perl != product_leave_orig_my_perl)
        return;
#endif
    /* 6.15: if (!is_profiling || !out || !profile_stmts) return; */
    if (product_opt_stmts(aTHX) == 0)
        return;

    cop = PL_curcop;
    file = cop ? OutCopFILE(cop) : NULL;
    line = cop ? (UV)CopLINE(cop) : 1;
    if (line == 0)
        line = 1;
    fid = product_fid_for_file_ptr(aTHX_ file);

    if (product_opt_blocks(aTHX))
        st = product_emit_attributed_time_block((nytp_fid)fid, (nytp_line)line,
                                                (nytp_line)line,
                                                (nytp_line)line);
    else
        st = product_emit_attributed_time_line((nytp_fid)fid, (nytp_line)line);
    if (st != NYTP_OK)
        return;
    (void)nytp_emit_discount(product_sink);
}

static OP *
pp_product_leave(pTHX)
{
    OP *prev_op = PL_op;
    U16 type = prev_op ? prev_op->op_type : 0;
    int slot = product_leave_slot_for(type);
    Perl_ppaddr_t orig = (slot >= 0) ? product_leave_orig[slot] : NULL;
    OP *op;

#ifdef MULTIPLICITY
    if (product_leave_orig_my_perl != NULL
        && my_perl != product_leave_orig_my_perl)
        return orig ? orig(aTHX) : NORMAL;
#endif

    /* 6.15: run original, then DB_leave(next_op, prev_op). */
    if (orig == NULL)
        return NORMAL;
    op = orig(aTHX);
    product_db_leave(aTHX_ op);
    return op;
}

static void
product_leave_hook(pTHX_ U16 type, int slot)
{
    PERL_UNUSED_CONTEXT;
    if (slot < 0 || slot >= PRODUCT_LEAVE_SLOT_COUNT)
        return;
    product_leave_orig[slot] = PL_ppaddr[type];
    PL_ppaddr[type] = pp_product_leave;
}

int
product_install_leave(pTHX)
{
    int blocks;

    if (product_leave_installed)
        return 0;

    blocks = product_opt_blocks(aTHX) ? 1 : 0;

    product_leave_hook(aTHX_ OP_LEAVESUB, PRODUCT_LEAVE_SLOT_LEAVESUB);
#ifdef OP_LEAVESUBLV
    product_leave_hook(aTHX_ OP_LEAVESUBLV, PRODUCT_LEAVE_SLOT_LEAVESUBLV);
#endif
    product_leave_hook(aTHX_ OP_LEAVE, PRODUCT_LEAVE_SLOT_LEAVE);
#ifdef OP_LEAVEWRITE
    product_leave_hook(aTHX_ OP_LEAVEWRITE, PRODUCT_LEAVE_SLOT_LEAVEWRITE);
#endif
#ifdef OP_LEAVEEVAL
    product_leave_hook(aTHX_ OP_LEAVEEVAL, PRODUCT_LEAVE_SLOT_LEAVEEVAL);
#endif
#ifdef OP_LEAVETRY
    product_leave_hook(aTHX_ OP_LEAVETRY, PRODUCT_LEAVE_SLOT_LEAVETRY);
#endif
    product_leave_hook(aTHX_ OP_RETURN, PRODUCT_LEAVE_SLOT_RETURN);

    /* KD-E14: blocks=1 keeps UNSTACK/LEAVELOOP on pp_product_stmt (780/810). */
    if (!blocks) {
#ifdef OP_LEAVELOOP
        product_leave_hook(aTHX_ OP_LEAVELOOP, PRODUCT_LEAVE_SLOT_LEAVELOOP);
#endif
        product_leave_hook(aTHX_ OP_UNSTACK, PRODUCT_LEAVE_SLOT_UNSTACK);
    }

    product_leave_installed = 1;
#ifdef MULTIPLICITY
    product_leave_orig_my_perl = my_perl;
#endif
    return 0;
}

int
product_uninstall_leave(pTHX)
{
    int i;

    PERL_UNUSED_CONTEXT;
    if (!product_leave_installed)
        return 0;

#define PRODUCT_LEAVE_RESTORE(type, slot)                                      \
    do {                                                                       \
        if (product_leave_orig[slot] != NULL)                                  \
            PL_ppaddr[type] = product_leave_orig[slot];                        \
    } while (0)

    PRODUCT_LEAVE_RESTORE(OP_LEAVESUB, PRODUCT_LEAVE_SLOT_LEAVESUB);
#ifdef OP_LEAVESUBLV
    PRODUCT_LEAVE_RESTORE(OP_LEAVESUBLV, PRODUCT_LEAVE_SLOT_LEAVESUBLV);
#endif
    PRODUCT_LEAVE_RESTORE(OP_LEAVE, PRODUCT_LEAVE_SLOT_LEAVE);
#ifdef OP_LEAVELOOP
    PRODUCT_LEAVE_RESTORE(OP_LEAVELOOP, PRODUCT_LEAVE_SLOT_LEAVELOOP);
#endif
#ifdef OP_LEAVEWRITE
    PRODUCT_LEAVE_RESTORE(OP_LEAVEWRITE, PRODUCT_LEAVE_SLOT_LEAVEWRITE);
#endif
#ifdef OP_LEAVEEVAL
    PRODUCT_LEAVE_RESTORE(OP_LEAVEEVAL, PRODUCT_LEAVE_SLOT_LEAVEEVAL);
#endif
#ifdef OP_LEAVETRY
    PRODUCT_LEAVE_RESTORE(OP_LEAVETRY, PRODUCT_LEAVE_SLOT_LEAVETRY);
#endif
    PRODUCT_LEAVE_RESTORE(OP_RETURN, PRODUCT_LEAVE_SLOT_RETURN);
    PRODUCT_LEAVE_RESTORE(OP_UNSTACK, PRODUCT_LEAVE_SLOT_UNSTACK);
#undef PRODUCT_LEAVE_RESTORE

    for (i = 0; i < PRODUCT_LEAVE_SLOT_COUNT; i++)
        product_leave_orig[i] = NULL;
    product_leave_installed = 0;
    product_leave_emit_on = 0;
#ifdef MULTIPLICITY
    product_leave_orig_my_perl = NULL;
#endif
    return 0;
}
