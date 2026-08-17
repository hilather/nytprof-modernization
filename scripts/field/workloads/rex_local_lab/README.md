# Rex local lab workload

Pinned Rexfile for [`complex_app_docker_profile.sh`](https://github.com/hilather/nytprof-modernization/blob/main/scripts/field/complex_app_docker_profile.sh).

The profiled process is **`run_lab.pl`**: `use Rex` + DateTime + YAML, then `main::lab_tick` / `main::lab_run`. `rex -T` on `Rexfile` is an unprofiled load canary (local connection, no SSH).

```sh
rex -q -f Rexfile -T
NYTPROF_DEMO_SECONDS=3 perl run_lab.pl
```
