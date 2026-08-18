/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * In-process SUB_CALLERS aggregation (not a Perl HV, not FileHandle).
 * Per-return SUB_RETURN stays on the wire; callers rows are one emit
 * per distinct (fid, line, called, caller) at finish so default zlib
 * does not compress a 1:1 copy of every return.
 */
#define PERL_NO_GET_CONTEXT
#include "EXTERN.h"
#include "perl.h"
#include "XSUB.h"

#include "nytprof_pp.h"

#include <stdlib.h>
#include <string.h>

#define PRODUCT_CALLERS_MIN 64u
#define PRODUCT_CALLERS_MAX (1u << 20)
#define PRODUCT_CALLERS_NAME_MAX NYTP_MAX_SUB_NAME_LEN

typedef struct product_callers_slot {
    uint32_t used;
    nytp_fid fid;
    nytp_line line;
    uint32_t count;
    uint32_t rec_depth;
    double incl;
    double excl;
    double reci;
    char *called;
    char *caller;
} product_callers_slot;

static product_callers_slot *product_callers_tab = NULL;
static uint32_t product_callers_cap = 0;
static uint32_t product_callers_used = 0;

static uint32_t
product_callers_hash(nytp_fid fid, nytp_line line, const char *called,
                     const char *caller)
{
    uint32_t h = 2166136261u;
    const unsigned char *p;

    h ^= (uint32_t)fid;
    h *= 16777619u;
    h ^= (uint32_t)line;
    h *= 16777619u;
    for (p = (const unsigned char *)called; *p; p++) {
        h ^= *p;
        h *= 16777619u;
    }
    h ^= 0xFFu;
    h *= 16777619u;
    for (p = (const unsigned char *)caller; *p; p++) {
        h ^= *p;
        h *= 16777619u;
    }
    return h;
}

static char *
product_callers_dup(const char *s, size_t n)
{
    char *out = (char *)malloc(n + 1);
    if (out == NULL)
        return NULL;
    memcpy(out, s, n);
    out[n] = '\0';
    return out;
}

static void
product_callers_free_tab(product_callers_slot *tab, uint32_t cap)
{
    uint32_t i;

    if (tab == NULL)
        return;
    for (i = 0; i < cap; i++) {
        if (tab[i].used) {
            free(tab[i].called);
            free(tab[i].caller);
        }
    }
    free(tab);
}

void
product_callers_reset(void)
{
    product_callers_free_tab(product_callers_tab, product_callers_cap);
    product_callers_tab = NULL;
    product_callers_cap = 0;
    product_callers_used = 0;
}

uint32_t
product_callers_len(void)
{
    return product_callers_used;
}

static nytp_status
product_callers_grow(void)
{
    uint32_t ncap;
    uint32_t i;
    product_callers_slot *ntab;

    if (product_callers_cap == 0)
        ncap = PRODUCT_CALLERS_MIN;
    else if (product_callers_cap >= PRODUCT_CALLERS_MAX)
        return NYTP_ERR_OVERFLOW;
    else
        ncap = product_callers_cap * 2u;
    if (ncap > PRODUCT_CALLERS_MAX)
        ncap = PRODUCT_CALLERS_MAX;

    ntab = (product_callers_slot *)calloc(ncap, sizeof(*ntab));
    if (ntab == NULL)
        return NYTP_ERR_OVERFLOW;

    for (i = 0; i < product_callers_cap; i++) {
        product_callers_slot *src = &product_callers_tab[i];
        uint32_t mask;
        uint32_t j;

        if (!src->used)
            continue;
        mask = ncap - 1u;
        j = product_callers_hash(src->fid, src->line, src->called, src->caller)
            & mask;
        while (ntab[j].used)
            j = (j + 1u) & mask;
        ntab[j] = *src;
    }
    free(product_callers_tab);
    product_callers_tab = ntab;
    product_callers_cap = ncap;
    return NYTP_OK;
}

nytp_status
product_callers_add(nytp_fid fid, nytp_line line, uint32_t count, double incl,
                    double excl, double reci, uint32_t rec_depth,
                    const char *called, const char *caller)
{
    size_t called_n;
    size_t caller_n;
    uint32_t mask;
    uint32_t i;
    nytp_status st;

    if (called == NULL)
        called = "";
    if (caller == NULL)
        caller = "";
    called_n = strlen(called);
    caller_n = strlen(caller);
    if (called_n >= PRODUCT_CALLERS_NAME_MAX
        || caller_n >= PRODUCT_CALLERS_NAME_MAX)
        return NYTP_ERR_OVERFLOW;
    if (count == 0)
        count = 1;

    if (product_callers_cap == 0) {
        st = product_callers_grow();
        if (st != NYTP_OK)
            return st;
    }
    if (product_callers_used * 10u >= product_callers_cap * 7u) {
        st = product_callers_grow();
        if (st != NYTP_OK)
            return st;
    }

    mask = product_callers_cap - 1u;
    i = product_callers_hash(fid, line, called, caller) & mask;
    for (;;) {
        product_callers_slot *s = &product_callers_tab[i];

        if (!s->used) {
            s->called = product_callers_dup(called, called_n);
            s->caller = product_callers_dup(caller, caller_n);
            if (s->called == NULL || s->caller == NULL) {
                free(s->called);
                free(s->caller);
                s->called = NULL;
                s->caller = NULL;
                return NYTP_ERR_OVERFLOW;
            }
            s->used = 1;
            s->fid = fid;
            s->line = line;
            s->count = count;
            s->incl = incl;
            s->excl = excl;
            s->reci = reci;
            s->rec_depth = rec_depth;
            product_callers_used++;
            return NYTP_OK;
        }
        if (s->fid == fid && s->line == line && strcmp(s->called, called) == 0
            && strcmp(s->caller, caller) == 0) {
            s->count += count;
            s->incl += incl;
            s->excl += excl;
            s->reci += reci;
            if (rec_depth > s->rec_depth)
                s->rec_depth = rec_depth;
            return NYTP_OK;
        }
        i = (i + 1u) & mask;
    }
}

nytp_status
product_callers_flush(nytp_sink *sink)
{
    uint32_t i;
    nytp_status first = NYTP_OK;

    if (sink == NULL) {
        product_callers_reset();
        return NYTP_ERR_NULL;
    }
    for (i = 0; i < product_callers_cap; i++) {
        product_callers_slot *s = &product_callers_tab[i];
        nytp_status st;

        if (!s->used)
            continue;
        st = nytp_emit_sub_callers(sink, s->fid, s->line, s->count, s->incl,
                                   s->excl, s->reci, s->rec_depth,
                                   nytp_sv_cstr(s->called),
                                   nytp_sv_cstr(s->caller));
        if (st != NYTP_OK && first == NYTP_OK)
            first = st;
    }
    product_callers_reset();
    return first;
}
