#!/usr/bin/env bash
# RPM-01 / PR-A1 — real staged NYTProfM-6.15.tar.gz builds xs-nytprof.
#
# Drives scripts/packaging/make_nytprofm_dist.sh (not a reimplemented tar).
# Honest skip of the compile half without a C compiler / ParseXS.
# Never puts crates/ on oracle PERL5LIB. Not BUILD-003-FULL. Not mock.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

DIST_SH="$ROOT/scripts/packaging/make_nytprofm_dist.sh"

usage() {
  cat <<'EOF'
Usage: rpm01_make_dist_smoke.sh

RPM-01: staged NYTProfM-6.15.tar.gz; unpack; make -C collector xs-nytprof;
no baseline/ crates/ target/.
EOF
}

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    *) printf 'ERROR: unknown flag: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

echo "rpm01_make_dist_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary (S2 not claimed)"
echo "never crates/ on PERL5LIB; not BUILD-003-FULL"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -x "$DIST_SH" ]] || fail "missing $DIST_SH"
[[ -f "$ROOT/t/workload-calls1.pl" ]] || fail "missing t/workload-calls1.pl"
[[ -f "$ROOT/t/installed_attach.t" ]] || fail "missing t/installed_attach.t"

WORK="$(mktemp -d "${TMPDIR:-/tmp}/rpm01-dist.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

echo "running: $DIST_SH $WORK"
OUT="$("$DIST_SH" "$WORK")"
[[ -f "$OUT" ]] || fail "dist script did not write a tarball: $OUT"
base="$(basename "$OUT")"
[[ "$base" == "NYTProfM-6.15.tar.gz" ]] || fail "expected NYTProfM-6.15.tar.gz, got $base"
ok "wrote $base"

# Leak check on the archive members (not the host tree).
if tar -tzf "$OUT" | grep -E -q '(^|/)(baseline|crates|target|prefix|fixtures|collector/build)(/|$)'; then
  fail "tarball contains baseline/ crates/ target/ prefix/ fixtures/ or collector/build/"
fi
ok "tarball has no baseline/crates/target/prefix/fixtures/collector/build"

tar -C "$WORK" -xzf "$OUT"
UNPACK="$WORK/NYTProfM-6.15"
[[ -d "$UNPACK/collector" ]] || fail "unpack missing collector/"
[[ -f "$UNPACK/collector/xs/Devel/NYTProfM.pm" ]] || fail "unpack missing NYTProfM.pm"
[[ -f "$UNPACK/collector/xs/NYTProf.xs" ]] || fail "unpack missing NYTProf.xs"
[[ -f "$UNPACK/collector/xs/nytprof_pp.h" ]] || fail "unpack missing nytprof_pp.h (graft header)"
[[ -f "$UNPACK/collector/xs/pp_entersub.c" ]] || fail "unpack missing pp_entersub.c"
[[ -f "$UNPACK/collector/xs/pp_leave.c" ]] || fail "unpack missing pp_leave.c"
[[ -f "$UNPACK/collector/xs/product_callers.c" ]] || fail "unpack missing product_callers.c"
[[ -f "$UNPACK/collector/xs/slowops.h" ]] || fail "unpack missing slowops.h"
# Workflow must not pin a NEVRA other than spec Release (v0.2.18 leftover -10).
WF="$ROOT/.github/workflows/release-el8-rpm.yml"
rel=$(sed -n 's/^Release:[[:space:]]*\([0-9][0-9]*\).*/\1/p' \
  "$ROOT/packaging/rpm/perl-NYTProfM.spec" | head -1)
[[ -n "$rel" ]] || fail "could not parse Release from perl-NYTProfM.spec"
grep -q 's/^Release:' "$WF" \
  || fail "release-el8-rpm.yml must parse spec Release (do not pin 6.15-N)"
if grep -Eoe 'perl-NYTProfM-6\.15-[0-9]+\.el8\.x86_64\.rpm' "$WF" \
    | grep -vq "6.15-${rel}.el8"; then
  fail "release-el8-rpm.yml hardcodes a NEVRA that is not spec Release $rel"
fi
ok "workflow NEVRA matches spec Release $rel (or is derived)"
[[ -f "$UNPACK/t/workload-calls1.pl" ]] || fail "unpack missing t/workload-calls1.pl"
[[ -f "$UNPACK/t/installed_attach.t" ]] || fail "unpack missing t/installed_attach.t"
[[ -f "$UNPACK/Makefile.PL" ]] || fail "unpack missing staged Makefile.PL"
[[ -f "$UNPACK/perl/bin/nytprofhtml" ]] \
  || fail "unpack missing perl/bin/nytprofhtml (I03 wrappers must ship in Source0)"
[[ -f "$UNPACK/perl/bin/nytprof-engine" ]] \
  || fail "unpack missing perl/bin/nytprof-engine"
[[ -f "$UNPACK/perl/lib/Devel/NYTProf/EngineDispatch.pm" ]] \
  || fail "unpack missing EngineDispatch.pm"
[[ -f "$UNPACK/t/installed_scripts.t" ]] \
  || fail "unpack missing t/installed_scripts.t"
[[ -x "$UNPACK/prebuilt/el8-x86_64/nytprof-cli" ]] \
  || fail "unpack missing prebuilt/el8-x86_64/nytprof-cli (Rocky 8 nytprof-cli)"
ok "unpacked NYTProfM-6.15 layout"

JSONL="$ROOT/fixtures/v5/default-calls1/readstream.jsonl"
[[ -f "$JSONL" ]] || fail "missing $JSONL"
echo "running: staged t/installed_scripts.t (query --json --jsonl 15/3/15)"
PERL5LIB="$UNPACK/perl/lib" NYTPROF_BINDDIR="$UNPACK/perl/bin" \
  NYTPROF_JSONL="$JSONL" perl "$UNPACK/t/installed_scripts.t" \
  || fail "staged installed_scripts.t failed"
ok "staged nytprof-engine query 15/3/15"

CLI="$UNPACK/prebuilt/el8-x86_64/nytprof-cli"
FIX="$ROOT/fixtures/v5/default-calls1/nytprof.out"
[[ -f "$FIX" ]] || fail "missing $FIX"
echo "running: staged EL8 nytprof-cli report --json (15/3/15)"
CLI_OUT="$("$CLI" report --json "$FIX")"
grep -E -q '"leaf_returns"[[:space:]]*:[[:space:]]*15|leaf_returns=15' <<<"$CLI_OUT" \
  || fail "prebuilt nytprof-cli missing leaf_returns=15"
grep -E -q '"mid_returns"[[:space:]]*:[[:space:]]*3|mid_returns=3' <<<"$CLI_OUT" \
  || fail "prebuilt nytprof-cli missing mid_returns=3"
ok "staged EL8 nytprof-cli report 15/3/15"

if ! command -v cc >/dev/null 2>&1 && ! command -v gcc >/dev/null 2>&1; then
  echo "SKIP: no C compiler — tarball layout asserts hold (not BUILD-003-FULL)"
  ok "rpm01 layout (compile skipped)"
  exit 0
fi

echo "running: make -C $UNPACK/collector xs-nytprof"
if ! make -C "$UNPACK/collector" xs-nytprof; then
  fail "make -C collector xs-nytprof failed inside staged tarball"
fi
[[ -f "$UNPACK/collector/build/xs-nytprof/Devel/NYTProfM.pm" ]] \
  || fail "xs-nytprof did not install Devel/NYTProfM.pm"
[[ -f "$UNPACK/collector/build/xs-nytprof/auto/Devel/NYTProfM/NYTProfM.so" ]] \
  || fail "xs-nytprof did not build NYTProfM.so"
ok "staged tree built xs-nytprof (D1-B)"
ok "rpm01_make_dist_smoke"
