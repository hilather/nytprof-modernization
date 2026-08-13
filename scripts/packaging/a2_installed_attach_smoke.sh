#!/usr/bin/env bash
# PR-A2 / RPM-03 — installed-tree %check (not repo scripts/, not nytprof-cli).
#
# Installs product XS into a temp prefix (same layout as perl-NYTProfM %install)
# and runs t/installed_attach.t against that prefix only.
# Dual_path stays oracle-primary. collection_default stays v5.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

echo "a2_installed_attach_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary (S2 not claimed)"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$ROOT/t/installed_attach.t" ]] || fail "missing t/installed_attach.t"
[[ -f "$ROOT/t/nytprof_v5_tag_table.inc" ]] || fail "missing t/nytprof_v5_tag_table.inc"
[[ -f "$ROOT/t/workload-calls1.pl" ]] || fail "missing t/workload-calls1.pl"
grep -F -q 'collector/build' "$ROOT/t/installed_attach.t" \
  || fail "installed_attach.t must refuse collector/build in @INC"

if ! command -v cc >/dev/null 2>&1 && ! command -v gcc >/dev/null 2>&1; then
  echo "SKIP: no C compiler — A2 installed attach not built"
  ok "a2 layout (compile skipped)"
  exit 0
fi

echo "make -C collector xs-nytprof"
make -C "$ROOT/collector" xs-nytprof
SRC="$ROOT/collector/build/xs-nytprof"
[[ -f "$SRC/Devel/NYTProfM.pm" ]] || fail "xs-nytprof missing NYTProfM.pm"
[[ -f "$SRC/auto/Devel/NYTProfM/NYTProfM.so" ]] || fail "xs-nytprof missing .so"

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-a2-XXXXXX")"
trap 'rm -rf "$WORKDIR"' EXIT
INST="$WORKDIR/lib/perl5"
mkdir -p "$INST/Devel/NYTProfM" "$INST/auto/Devel/NYTProfM"
install -m 644 "$SRC/Devel/NYTProfM.pm" "$INST/Devel/NYTProfM.pm"
install -m 644 "$SRC/Devel/NYTProfM/Core.pm" "$INST/Devel/NYTProfM/Core.pm"
install -m 755 "$SRC/auto/Devel/NYTProfM/NYTProfM.so" \
  "$INST/auto/Devel/NYTProfM/NYTProfM.so"

unset PERL5OPT || true
export PERL5LIB="$INST"
echo "PERL5LIB=$PERL5LIB"
echo "running: perl t/installed_attach.t"
set +e
OUT="$(perl "$ROOT/t/installed_attach.t" 2>&1)"
RC=$?
set -e
printf '%s\n' "$OUT"
[[ "$RC" -eq 0 ]] || fail "t/installed_attach.t exited $RC"
grep -F -q 'OK: installed attach leaf=15 mid=3 edge=15' <<<"$OUT" \
  || fail "missing 15/3/15 from installed parser"
grep -F -q 'OK: installed format=v6 fail-closed' <<<"$OUT" \
  || fail "missing format=v6 fail-closed"
ok "A2 installed-tree attach 15/3/15 + v6 fail-closed"
