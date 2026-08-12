/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * COL-015 — Fork / PID protocol with buffered sinks.
 */
#include "nytp_fork.h"

#include <stdio.h>
#include <string.h>

nytp_fork_policy nytp_fork_policy_default(void)
{
    nytp_fork_policy p;
    p.flush_before_fork = 1;
    p.require_empty_buffer = 0;
    p.discard_child_buffer = 1;
    p.fail_if_child_residual = 0;
    return p;
}

void nytp_fork_metrics_clear(nytp_fork_metrics *m)
{
    if (m) {
        memset(m, 0, sizeof(*m));
    }
}

size_t nytp_fork_discard_batch_residual(nytp_sink *sink,
                                        nytp_fork_metrics *metrics)
{
    nytp_batch *b;
    size_t n;
    size_t arena;
    if (!sink) {
        return 0;
    }
    b = nytp_batch_sink_batch(sink);
    if (!b) {
        return 0;
    }
    n = b->count;
    arena = b->arena_used;
    if (n == 0 && arena == 0) {
        return 0;
    }
    nytp_batch_discard_pending(b);
    if (metrics) {
        metrics->child_discard_events += (uint64_t)n;
        metrics->child_discard_arena += (uint64_t)arena;
    }
    return n;
}

nytp_status nytp_fork_prepare(nytp_sink *root, const nytp_fork_policy *pol,
                              nytp_fork_metrics *metrics)
{
    nytp_fork_policy use;
    nytp_status st;
    nytp_batch *b;

    if (!root) {
        return NYTP_ERR_NULL;
    }
    use = pol ? *pol : nytp_fork_policy_default();
    if (metrics) {
        metrics->prepare_calls++;
    }

    if (nytp_sink_get_state(root) != NYTP_SINK_ACTIVE) {
        return NYTP_ERR_STATE;
    }

    if (use.flush_before_fork) {
        st = nytp_sink_flush(root);
        if (st != NYTP_OK) {
            if (metrics) {
                metrics->preflush_fail++;
            }
            /* Sticky fail already applied by public flush for hard errors. */
            return st;
        }
        if (metrics) {
            metrics->preflush_ok++;
        }
    } else if (metrics) {
        metrics->preflush_skipped++;
    }

    if (use.require_empty_buffer) {
        b = nytp_batch_sink_batch(root);
        if (b && b->count > 0) {
            return NYTP_ERR_STATE;
        }
    }

    st = nytp_sink_begin_fork(root);
    if (st != NYTP_OK) {
        if (metrics) {
            metrics->begin_fork_fail++;
        }
        return st;
    }
    if (metrics) {
        metrics->begin_fork_ok++;
    }
    return NYTP_OK;
}

nytp_status nytp_fork_resume_parent(nytp_sink *root, nytp_fork_metrics *metrics)
{
    nytp_status st;
    if (!root) {
        return NYTP_ERR_NULL;
    }
    st = nytp_sink_end_fork_parent(root);
    if (st == NYTP_OK && metrics) {
        metrics->parent_resume++;
    }
    return st;
}

nytp_status nytp_fork_resume_child(nytp_sink *root, const nytp_fork_policy *pol,
                                   nytp_fork_metrics *metrics)
{
    nytp_fork_policy use;
    nytp_status st;
    size_t residual;

    if (!root) {
        return NYTP_ERR_NULL;
    }
    use = pol ? *pol : nytp_fork_policy_default();

    if (nytp_sink_get_state(root) != NYTP_SINK_FORK_SPLIT) {
        return NYTP_ERR_STATE;
    }

    residual = 0;
    {
        nytp_batch *b = nytp_batch_sink_batch(root);
        if (b) {
            residual = b->count;
        }
    }

    if (residual > 0 && use.fail_if_child_residual) {
        return NYTP_ERR_STATE;
    }

    if (use.discard_child_buffer) {
        (void)nytp_fork_discard_batch_residual(root, metrics);
    }

    st = nytp_sink_end_fork_child(root);
    if (st == NYTP_OK && metrics) {
        metrics->child_resume++;
    }
    return st;
}

int nytp_fork_addpid_path(const char *base, nytp_pid pid, char *buf,
                          size_t buflen)
{
    int n;
    if (!base || base[0] == '\0') {
        return -1;
    }
    /* snprintf returns would-be length excluding NUL on truncation? C99:
     * returns number of chars that would have been written (excl NUL). */
    n = snprintf(buf, buflen, "%s.%u", base, (unsigned)pid);
    return n;
}
