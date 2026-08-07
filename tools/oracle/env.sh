#!/usr/bin/env bash
# Load oracle PERL5LIB isolation for tools under tools/oracle/.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/../.."
ROOT="$(cd "$ROOT" && pwd)"
export NYTPROF_MOD_ROOT="$ROOT"
BASELINE="$ROOT/baseline/6.15"
if [[ ! -f "$BASELINE/oracle-perl5lib.txt" ]]; then
  echo "Oracle not built; run scripts/baseline/run_all.sh" >&2
  exit 1
fi
export PERL5LIB="$(cat "$BASELINE/oracle-perl5lib.txt")"
if [[ -d "$BASELINE/test-deps/lib/perl5" ]]; then
  export PERL5LIB="$BASELINE/test-deps/lib/perl5:$PERL5LIB"
fi
export PATH="$BASELINE/install/bin:$PATH"
export NYTPROF_ORACLE_MODULE="$(cat "$BASELINE/oracle-module-path.txt")"
# Prove isolation without starting the profiler (do not -MDevel::NYTProf)
LOADED="$(perl -e 'foreach my $d (split /:/, $ENV{PERL5LIB}//"") {
  my $p = "$d/Devel/NYTProf.pm";
  if (-f $p) { print $p; exit 0 }
}
die "Devel/NYTProf.pm not found on PERL5LIB\n"')"
case "$LOADED" in
  "$BASELINE/install"/*) ;;
  *)
    echo "ERROR: oracle NYTProf.pm not under install tree: $LOADED" >&2
    exit 1
    ;;
esac
