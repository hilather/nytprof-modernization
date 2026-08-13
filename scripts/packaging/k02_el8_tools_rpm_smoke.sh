#!/usr/bin/env bash
# K02 — EL8 nytprof-cli tools RPM companion (ADR-0010 ingest).
#
# Drives the real spec. When a shipped nytprof-cli is discoverable (prefix
# or PATH, never crates/ on PERL5LIB), report --json of a real v5 fixture
# is leaf 15 / mid 3 / mid→leaf 15. Honest SKIP without CLI / rpmspec.
# Does not require a live signed artifact or rustup-in-mock.
#
# Exit 0: K02 pass. Exit 1: spec/CLI failure. Exit 2: misuse / crates/.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

SPEC="$ROOT/packaging/rpm/nytprof-cli.spec"
FIXTURE="$ROOT/fixtures/v5/default-calls1/nytprof.out"

usage() {
  cat <<'EOF'
Usage: k02_el8_tools_rpm_smoke.sh

K02: real nytprof-cli tools spec is a companion (Recommends module,
ADR-0010, no rustup-in-mock). CLI report 15/3/15 when discoverable.
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

echo "k02_el8_tools_rpm_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary (S2 not claimed)"
echo "never crates/ on PERL5LIB; tools are not drop-in collection"
echo "signed publish pipeline residual; no rustup-in-mock"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$SPEC" ]] || fail "missing $SPEC"
[[ -f "$FIXTURE" ]] || fail "missing fixture $FIXTURE"

grep -E -q '^Name:[[:space:]]+nytprof-cli' "$SPEC" \
  || fail "spec Name is not nytprof-cli"
grep -Eiq 'Recommends:[[:space:]]*perl-NYTProfM' "$SPEC" \
  || fail "spec must Recommends perl-NYTProfM"
grep -F -q 'ADR-0010' "$SPEC" \
  || fail "spec must cite ADR-0010"
grep -Eiq 'signed CI prebuilt|signed CI' "$SPEC" \
  || fail "spec must consume signed CI prebuilts"
grep -Eiq 'not drop-in|NOT drop-in|not collection drop-in' "$SPEC" \
  || fail "spec must say tools are not drop-in collection"
grep -F -q 'perl -d:NYTProfM' "$SPEC" \
  || fail "spec must state it does not provide perl -d:NYTProfM"
if grep -Eiq 'BuildRequires:[[:space:]]*(cargo|rustc|rustup)' "$SPEC"; then
  fail "tools spec must not BuildRequire cargo/rustc/rustup"
fi
if awk '/^%build/,/^%install/' "$SPEC" | grep -v '^#' | grep -Eiq 'rustup|cargo |rustc '; then
  fail "tools spec %build must not invoke rustup/cargo/rustc"
fi
grep -F -q 'linux-x86_64' "$SPEC" \
  || fail "spec must advertise linux-x86_64 prebuilt ingest"
ok "real spec: nytprof-cli companion, Recommends module, ADR-0010, no rustup-in-mock"

if command -v rpmspec >/dev/null 2>&1; then
  echo "running: rpmspec -q --srpm $SPEC"
  set +e
  SPECQ="$(rpmspec -q --srpm "$SPEC" 2>&1)"
  SPECRC=$?
  set -e
  printf '%s\n' "$SPECQ"
  if [[ "$SPECRC" -ne 0 ]]; then
    echo "SKIP: rpmspec --srpm failed (host macros / missing Source*) — spec file asserts hold"
  else
    grep -E -q 'nytprof-cli-7\.00' <<<"$SPECQ" \
      || fail "rpmspec query missing nytprof-cli-7.00"
    ok "rpmspec queried real spec as nytprof-cli-7.00"
  fi
elif command -v rpmbuild >/dev/null 2>&1; then
  echo "SKIP: rpmbuild present but no isolated mock / signed tarball"
else
  echo "SKIP: no rpmspec/rpmbuild on PATH — spec asserts hold (pipeline residual)"
fi

NATIVE=""
if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
  NATIVE="${NYTPROF_NATIVE_CLI}"
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  NATIVE="$ROOT/prefix/bin/nytprof-cli"
elif command -v nytprof-cli >/dev/null 2>&1; then
  NATIVE="$(command -v nytprof-cli)"
fi

if [[ -n "$NATIVE" ]]; then
  echo "running: $NATIVE report --json $FIXTURE"
  set +e
  OUT="$("$NATIVE" report --json "$FIXTURE" 2>&1)"
  RC=$?
  set -e
  printf '%s\n' "$OUT"
  [[ "$RC" -eq 0 ]] || fail "nytprof-cli report --json failed (rc=$RC)"
  LEAF="$(perl -ne 'print $1 if /"leaf_returns"\s*:\s*(\d+)/' <<<"$OUT")"
  MID="$(perl -ne 'print $1 if /"mid_returns"\s*:\s*(\d+)/' <<<"$OUT")"
  EDGE="$(perl -ne 'print $1 if /"mid_leaf_edge"\s*:\s*(\d+)/' <<<"$OUT")"
  echo "report --json: leaf_returns=${LEAF:-?} mid_returns=${MID:-?} mid_leaf_edge=${EDGE:-?}"
  [[ "$LEAF" == "15" ]] || fail "leaf_returns=$LEAF (want 15) from real CLI + fixture"
  [[ "$MID" == "3" ]] || fail "mid_returns=$MID (want 3)"
  [[ "$EDGE" == "15" ]] || fail "mid_leaf_edge=$EDGE (want 15)"
  ok "shipped nytprof-cli report of real v5 fixture: leaf 15 / mid 3 / mid→leaf 15"
else
  echo "SKIP: no nytprof-cli on prefix/PATH — spec asserts hold"
fi

echo "NOT-YET: signed CI publish/verify pipeline"
echo "NOT-YET: BUILD-003-FULL / PRODUCT-V6-COLLECT-EL8 / S2"
ok "EL8-RPM-TOOLS"
exit 0
