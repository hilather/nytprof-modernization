#!/usr/bin/env bash
# Complex-app (Rex) Docker lab — integration smoke.
#
# Spec:  docs/schemas/complex-app-docker-profile-mvp-v0.md
# Demo:  scripts/field/complex_app_docker_profile.sh --lab
#
# Always (no docker):
#   - demo + Rexfile exist and parse
#   - demo --help
#
# When docker is usable:
#   - drives the real demo --lab path (in-tree xs-nytprof + rex CLI)
#   - asserts NYTProf 5, html/index.html, rex_lab_ok, no BHES/Getopt abort
#
# Honest SKIP of the docker half when docker is absent or the daemon
# is unreachable. Does NOT join offline_gate (network + yum + CPAN + image).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
# shellcheck source=lib/attach_survival.sh
source "$ROOT/scripts/field/lib/attach_survival.sh"

DEMO="$ROOT/scripts/field/complex_app_docker_profile.sh"
REXFILE="$ROOT/scripts/field/workloads/rex_local_lab/Rexfile"
DRIVER="$ROOT/scripts/field/workloads/rex_local_lab/run_lab.pl"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
skip() { printf 'SKIP: %s\n' "$*"; }

[[ -f "$DEMO" ]] || fail "missing $DEMO"
[[ -f "$REXFILE" ]] || fail "missing $REXFILE"
[[ -f "$DRIVER" ]] || fail "missing $DRIVER"
chmod +x "$DEMO" 2>/dev/null || true

bash -n "$DEMO" || fail "bash -n complex_app_docker_profile.sh"
"$DEMO" --help >/dev/null || fail "demo --help failed"
"$DEMO" --help | grep -q -- '--lab' \
  || fail "demo --help must document --lab"
"$DEMO" --help | grep -q -- '--engine' \
  || fail "demo --help must document --engine native|oracle|both"
"$DEMO" --help | grep -q -- '--app' \
  || fail "demo --help must document --app"
grep -q 'use DateTime' "$DRIVER" || fail "run_lab.pl must use DateTime"
grep -q 'use Rex' "$DRIVER" || fail "run_lab.pl must use Rex"
grep -q 'use DateTime' "$REXFILE" || fail "Rexfile must use DateTime"
grep -q 'DateTime::Duration' "$REXFILE" || fail "Rexfile must use DateTime::Duration"
grep -q 'use YAML' "$REXFILE" || fail "Rexfile must use YAML"
grep -q 'connection => '\''Local'\''' "$REXFILE" \
  || fail "Rexfile must set connection Local (no SSH)"
grep -q '::import' "$ROOT/collector/xs/Devel/NYTProfM.pm" \
  || fail "NYTProfM.pm must goto inherited ::import (Rex Shared::Var)"
grep -q 'XSLoader' "$ROOT/collector/xs/Devel/NYTProfM.pm" \
  || fail "NYTProfM.pm must goto XSLoader (DB.so bootstrap)"
ok "demo script + Rexfile parse"

if ! command -v docker >/dev/null 2>&1; then
  skip "docker not on PATH — Rex container half not run"
  ok "complex_app_docker_profile_smoke host checks (docker SKIP)"
  exit 0
fi
if ! docker info >/dev/null 2>&1; then
  skip "docker daemon not reachable — Rex container half not run"
  ok "complex_app_docker_profile_smoke host checks (docker SKIP)"
  exit 0
fi

PACK="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-rex-lab-XXXXXX")"
cleanup() { rm -rf "$PACK"; }
trap cleanup EXIT

echo "=== Rex docker lab: $DEMO --app rex --lab --engine both --out $PACK --seconds 3 ==="
bash "$DEMO" --app rex --lab --engine both --out "$PACK" --seconds 3 \
  || fail "complex_app_docker_profile.sh --lab --engine both failed"

[[ -s "$PACK/nytprof.out" ]] || fail "missing nytprof.out"
head -c 9 "$PACK/nytprof.out" | grep -q 'NYTProf 5' \
  || fail "nytprof.out is not NYTProf 5"
[[ -f "$PACK/html/index.html" ]] || fail "missing html/index.html"
[[ -L "$PACK/html" ]] || fail "html must be a symlink to native after --engine both"
[[ -f "$PACK/meta/rex-profiled.txt" ]] || fail "missing meta/rex-profiled.txt"
grep -q 'rex_lab_ok' "$PACK/meta/rex-profiled.txt" \
  || fail "meta/rex-profiled.txt missing rex_lab_ok"
attach_fail_if_killed "$PACK/meta/rex-profiled.txt" \
  || fail "lab attach hit shipped attach-kill string"
grep -q 'time_line_events' "$PACK/html/index.html" \
  || fail "html/index.html missing time_line_events"
if [[ -f "$PACK/meta/report.txt" ]]; then
  if ! grep -E -q 'Rex::|DateTime::' "$PACK/meta/report.txt"; then
    fail "meta/report.txt has no Rex:: or DateTime:: (attach did not see the app)"
  fi
fi
if [[ -f "$PACK/meta/verify.txt" ]]; then
  grep -q '^OK' "$PACK/meta/verify.txt" \
    || fail "meta/verify.txt missing OK"
fi
ok "native Rex lab: NYTProf 5 + html/index.html + rex_lab_ok"

if [[ -f "$PACK/oracle/meta/oracle-skip.txt" ]]; then
  skip "oracle half SKIP ($(cat "$PACK/oracle/meta/oracle-skip.txt"))"
elif [[ -s "$PACK/oracle/nytprof.out" ]]; then
  head -c 9 "$PACK/oracle/nytprof.out" | grep -q 'NYTProf 5' \
    || fail "oracle nytprof.out is not NYTProf 5"
  grep -q 'rex_lab_ok' "$PACK/oracle/meta/rex-profiled.txt" \
    || fail "oracle missing rex_lab_ok (6.15 should run this driver)"
  if grep -qi 'crates/' "$PACK/oracle/meta/perl5lib.txt" 2>/dev/null; then
    fail "oracle PERL5LIB leaked crates/"
  fi
  ok "oracle Rex lab: NYTProf 5 + rex_lab_ok (attach survival)"
else
  skip "oracle half produced no nytprof.out"
fi
[[ -f "$PACK/COMPARE.txt" ]] || fail "missing COMPARE.txt after --engine both"
ok "complex_app_docker_profile_smoke"
exit 0
