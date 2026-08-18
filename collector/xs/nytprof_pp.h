/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * Shared C symbols for NYTProf.xs + grafted pp_entersub.c (DI-03 E1a).
 * Include after perl.h. Compile pp_entersub.c with Embed ccopts, not
 * the collector src/%.c sink rule.
 */
#ifndef NYTPROF_PP_H
#define NYTPROF_PP_H

#include "nytp_sink.h"
#include "nytp_clock.h"
#include "nytp_types.h"

#ifndef NYTP_MAX_SUB_NAME_LEN
#define NYTP_MAX_SUB_NAME_LEN 500
#endif

/* Unstatic from NYTProf.xs so the graft can emit on the one product sink. */
extern nytp_sink *product_sink;
nytp_fid product_fid_for_file_ptr(pTHX_ const char *file);
IV       product_opt_calls(pTHX);
void     product_add_pending_child_excl(NV);
NV       product_take_pending_child_excl(void);

int  product_install_entersub(pTHX);     /* OP_ENTERSUB + OP_GOTO (E2) */
int  product_uninstall_entersub(pTHX);
int  product_entersub_is_installed(void);
int  product_goto_is_installed(void);
int  product_entersub_emit_enabled(void);
void product_entersub_set_emit_enabled(int on);
void *product_current_subr_entry(void);
void product_subr_add_child_incl(void *se, NV incl_nv);
void product_credit_child_excl(NV incl_nv);

#endif /* NYTPROF_PP_H */
