# leave=1 UNSTACK writes a second TIME_LINE on the same for-body line

**Not a certification claim.** Same-host engineering sample, 2026-08-18.

Workload: `for (1 .. 400_000) { $pair = $all{$dst}{$src->{type}}{$src->{id}} }`  
Unprofiled wall: **0.06s**. Isolated 6.15 pin vs in-tree NYTProfM. Dump TIME_LINE summed by source line.

| Attach | `leave` | Line-11 events | Line-11 time | `do_hash` incl | Profiled wall |
|--------|---------|----------------|--------------|----------------|---------------|
| NYTProfM default | 0 | 400_000 | 0.077s | 0.077s | 0.19s |
| 6.15 `leave=0` | 0 | 400_000 | 0.075s | 0.075s | 0.33s |
| 6.15 default | 1 | **800_000** | 0.104s | 0.104s | 0.61s |
| NYTProfM `leave=1` | 1 | **800_000** | 0.239s | 0.239s | 0.51s |

6.15 HTML for the default profile: **400_000** calls (DISCOUNT applied to **count** only), **104ms** time (both TIME_LINE writes). Pin comment at `DB_leave`: `XXX OP_UNSTACK needs help` when last fid:line does not change.

`leave=0` on both engines agrees on the assignment line (~75–77ms). The extra 6.15 seconds are the UNSTACK close staying on the body line, not a second hash lookup.

Do **not** flip product default `leave=1` to “match HTML.” That reproduces the 2× event count and, on this host, **over**-charges (0.239s vs pin 0.104s) because the product leave hook is heavier and also fails to retarget.
