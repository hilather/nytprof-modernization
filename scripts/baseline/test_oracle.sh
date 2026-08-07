#!/usr/bin/env bash
# Run upstream tests (or a documented subset) against the installed oracle.
set -euo pipefail
# shellcheck source=common.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

if [[ ! -f "$BASELINE_DIR/oracle-perl5lib.txt" ]]; then
  echo "Run build_oracle.sh first" >&2
  exit 1
fi

export PERL5LIB="$(cat "$BASELINE_DIR/oracle-perl5lib.txt")"
# Optional local test-only deps (Capture::Tiny, Test::Differences)
if [[ -d "$BASELINE_DIR/test-deps/lib/perl5" ]]; then
  export PERL5LIB="$BASELINE_DIR/test-deps/lib/perl5:$PERL5LIB"
fi
# Isolation: empty candidate influence; do not inherit profiling env into upstream tests
unset PERL_LOCAL_LIB_ROOT PERL_MB_OPT PERL_MM_OPT
unset NYTPROF NYTPROF_MOD_ROOT
# Avoid leftover profile files confusing tests that use fixed names
rm -f nytprof.out t/nytprof*.out 2>/dev/null || true

TEST_LOG="$LOG_DIR/test_oracle.log"
cd "$SRC_DIR"
rm -f nytprof.out t/nytprof*.out 2>/dev/null || true

{
  echo "=== test_oracle $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="
  echo "PERL5LIB=$PERL5LIB"
  perl -MDevel::NYTProf -e 'print "Using ", $INC{"Devel/NYTProf.pm"}, " v$Devel::NYTProf::VERSION\n"'
  LOADED="$(perl -MDevel::NYTProf -e 'print $INC{"Devel/NYTProf.pm"}')"
  case "$LOADED" in
    "$INSTALL_DIR"/*) ;;
    *) echo "ERROR: not using install-tree module: $LOADED" >&2; exit 1 ;;
  esac

  # Prefer in-tree make test with blib (matches source pin) while still recording install path
  if [[ -f Makefile ]]; then
    echo "=== make test ==="
    if [[ -n "${NYTPROF_ORACLE_TEST_FILES:-}" ]]; then
      # shellcheck disable=SC2086
      prove -b $NYTPROF_ORACLE_TEST_FILES
    else
      make test
    fi
  else
    echo "No Makefile; running prove on t/ with installed module"
    prove -I"$PERL5LIB" t/
  fi
} 2>&1 | tee "$TEST_LOG"

echo "Tests finished. Log: $TEST_LOG"
