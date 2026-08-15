#!/usr/bin/env bash
# Rocky 8 Docker profile lab — integration smoke.
#
# Spec:  docs/schemas/rocky8-docker-profile-lab-mvp-v0.md
# Demo:  scripts/field/rocky8_docker_profile_demo.sh --lab
#
# Always (no docker):
#   - demo + scanner exist and parse
#   - host perl -c + 1s unprofiled scanner run
#
# When docker is usable:
#   - drives the real demo --lab --engine both path (native NYTProfM +
#     isolated 6.15 oracle containers)
#   - asserts NYTProf 5, html/index.html (symlink to native), verify OK,
#     scanner rc=0; oracle half SKIP-capable
#
# Honest SKIP of the docker half when docker is absent or the daemon
# is unreachable. Does NOT join offline_gate (network + yum + image pull).
#
# Usage:
#   ./scripts/field/rocky8_docker_profile_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

DEMO="$ROOT/scripts/field/rocky8_docker_profile_demo.sh"
SCANNER="$ROOT/scripts/field/workloads/minute_text_scanner.pl"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
skip() { printf 'SKIP: %s\n' "$*"; }

[[ -f "$DEMO" ]] || fail "missing $DEMO"
[[ -f "$SCANNER" ]] || fail "missing $SCANNER"
chmod +x "$DEMO" "$SCANNER" 2>/dev/null || true

bash -n "$DEMO" || fail "bash -n rocky8_docker_profile_demo.sh"
"$DEMO" --help >/dev/null || fail "demo --help failed"
"$DEMO" --help | grep -q -- '--engine' \
  || fail "demo --help must document --engine native|oracle|both"
ok "demo script parses"

perl -c "$SCANNER" >/dev/null || fail "perl -c minute_text_scanner.pl"
TMP_HOST="$(mktemp -d)"
trap 'rm -rf "$TMP_HOST"' EXIT
echo 'It is a truth universally acknowledged' >"$TMP_HOST/a.txt"
HOST_OUT="$(perl "$SCANNER" "$TMP_HOST" 1)"
echo "$HOST_OUT" | grep -q '^passes=' \
  || fail "host scanner 1s missing passes= (got: $HOST_OUT)"
ok "host scanner 1s: $HOST_OUT"

if ! command -v docker >/dev/null 2>&1; then
  skip "docker not on PATH — Rocky 8 container half not run"
  ok "rocky8_docker_profile_smoke host checks (docker SKIP)"
  exit 0
fi
if ! docker info >/dev/null 2>&1; then
  skip "docker daemon not reachable — Rocky 8 container half not run"
  ok "rocky8_docker_profile_smoke host checks (docker SKIP)"
  exit 0
fi

PACK="$TMP_HOST/lab"
log() { printf '%s\n' "$*"; }
log "=== Rocky 8 docker lab: $DEMO --lab --engine both --out $PACK ==="
bash "$DEMO" --lab --engine both --out "$PACK" --seconds 3 \
  || fail "rocky8_docker_profile_demo.sh --lab --engine both failed"

[[ -s "$PACK/nytprof.out" ]] || fail "missing nytprof.out"
[[ -f "$PACK/html/index.html" ]] || fail "missing html/index.html"
[[ -f "$PACK/meta/timings.txt" ]] || fail "missing meta/timings.txt"
[[ -L "$PACK/html" ]] || fail "KD-LAYOUT: html must be a symlink after --engine both"
[[ -d "$PACK/native/html" ]] || fail "missing native/html after --engine both"
if [[ -f "$PACK/oracle/html/index.html" ]]; then
  grep -q 'Performance Profile Index' "$PACK/oracle/html/index.html" \
    || fail "oracle index missing Performance Profile Index"
  if grep -qi 'crates/' "$PACK/oracle/meta/perl5lib.txt" 2>/dev/null; then
    fail "oracle PERL5LIB leaked crates/"
  fi
  ok "oracle HTML present (isolated 6.15)"
elif [[ -f "$PACK/oracle/meta/oracle-skip.txt" ]]; then
  skip "oracle half SKIP ($(cat "$PACK/oracle/meta/oracle-skip.txt"))"
else
  skip "oracle half produced no html/index.html (compile/image residual)"
fi

head -c 16 "$PACK/nytprof.out" | grep -q 'NYTProf 5' \
  || fail "nytprof.out is not NYTProf 5 (got: $(head -c 32 "$PACK/nytprof.out" | tr -c '[:print:]' '.'))"

grep -q 'time_line_events' "$PACK/html/index.html" \
  || fail "html/index.html missing time_line_events"
grep -q 'main::tokenize' "$PACK/html/index.html" \
  || fail "html/index.html missing main::tokenize"

grep -q 'profiled_scanner_rc=0' "$PACK/meta/timings.txt" \
  || fail "scanner profile rc was not 0 (see $PACK/meta/timings.txt)"
grep -q 'primary_profile=minute_text_scanner' "$PACK/meta/timings.txt" \
  || fail "timings missing primary_profile=minute_text_scanner"
grep -q '^lab=1$' "$PACK/meta/timings.txt" \
  || fail "timings missing lab=1"

[[ -s "$PACK/meta/scanner-profiled.txt" ]] \
  || fail "missing scanner-profiled.txt"
grep -q '^passes=' "$PACK/meta/scanner-profiled.txt" \
  || fail "scanner stdout missing passes="

if [[ -f "$PACK/meta/verify.txt" ]]; then
  grep -q '^OK:' "$PACK/meta/verify.txt" \
    || fail "nytprof-cli verify was not OK (see $PACK/meta/verify.txt)"
  ok "verify: $(head -1 "$PACK/meta/verify.txt")"
fi

if [[ -f "$PACK/meta/nytprofm-version.txt" ]]; then
  grep -q '6.15' "$PACK/meta/nytprofm-version.txt" \
    || fail "Devel::NYTProfM version is not 6.15"
fi

# Re-render HTML with the in-tree CLI so operator v1 (seconds/heat/sort/union)
# is asserted even when the testdrive RPM CLI is older.
html_cli=()
if [[ -x "$ROOT/target/debug/nytprof-cli" ]]; then
  html_cli=("$ROOT/target/debug/nytprof-cli")
elif [[ -x "$ROOT/target/release/nytprof-cli" ]]; then
  html_cli=("$ROOT/target/release/nytprof-cli")
elif command -v cargo >/dev/null 2>&1 && [[ -f "$ROOT/Cargo.toml" ]]; then
  html_cli=(cargo run -q -p nytprof-cli --)
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  html_cli=("$ROOT/prefix/bin/nytprof-cli")
fi
if [[ ${#html_cli[@]} -gt 0 ]]; then
  "${html_cli[@]}" html "$PACK/nytprof.out" --out-dir "$PACK/html" \
    >"$PACK/meta/html-v1.out" 2>"$PACK/meta/html-v1.err" \
    || fail "in-tree html --out-dir failed (see meta/html-v1.err)"
  "${html_cli[@]}" report "$PACK/nytprof.out" \
    >"$PACK/meta/report.txt" 2>"$PACK/meta/report.err" || true
fi

IDX="$PACK/html/index.html"
SRC="$(ls -1 "$PACK/html"/file-*.html 2>/dev/null | head -1 || true)"
[[ -f "$IDX" ]] || fail "missing $IDX after v1 render"
grep -q 'main::tokenize' "$IDX" || fail "index missing main::tokenize"
# Workload sub times: not all-zero (KD-O: do not require CORE::match).
if [[ -f "$PACK/meta/report.txt" ]]; then
  set +e
  perl -e '
    my $ok = 0;
    while (<>) {
      if (/main::tokenize\s+returns=\d+\s+excl=(\S+)\s+incl=(\S+)/) {
        print "tokenize excl=$1 incl=$2\n";
        exit(($1 ne "0" && $2 ne "0") ? 0 : 3);
      }
    }
    exit 2;
  ' "$PACK/meta/report.txt"
  tok_rc=$?
  set -e
  [[ "$tok_rc" -eq 0 ]] \
    || fail "main::tokenize incl/excl must be non-zero (rc=$tok_rc; see meta/report.txt)"
  ok "main::tokenize incl/excl non-zero"
fi
if [[ -n "$SRC" && -f "$SRC" ]]; then
  grep -q '<tr' "$SRC" || fail "$SRC has no source <tr> rows"
  grep -q 'id="L' "$SRC" || fail "$SRC missing id=\"L anchors"
  ok "source page has rows + #Ln ($SRC)"
fi
[[ -f "$PACK/html/style.css" ]] || fail "missing html/style.css"
grep -q 'heat-hot' "$PACK/html/style.css" \
  || fail "style.css missing heat-hot"
ok "style.css has heat-hot"
[[ -f "$PACK/html/nytprof-sort.js" ]] || fail "missing html/nytprof-sort.js"
grep -qi 'jquery' "$PACK/html/nytprof-sort.js" \
  && fail "sort js must not mention jquery"
grep -qi 'tablesorter' "$PACK/html/nytprof-sort.js" \
  && fail "sort js must not mention tablesorter"
grep -q 'nytprof-sort.js' "$IDX" \
  || fail "index.html does not reference nytprof-sort.js"
ok "vanilla nytprof-sort.js published (no jquery/tablesorter)"
grep -q 'Performance Profile Index' "$IDX" \
  || fail "v2 chrome missing Performance Profile Index"
grep -q 'id="subs_table"' "$IDX" \
  || fail "v2 index missing #subs_table"
grep -q 'id="filestable"' "$IDX" \
  || fail "v2 index missing #filestable"
grep -q 'href="source.html"' "$IDX" \
  || fail "v2 index missing href=source.html"
if [[ -f "$PACK/html/source.html" ]]; then
  grep -q 'minute_text_scanner' "$PACK/html/source.html" \
    || fail "source.html must be the scanner (not warnings.pm)"
fi
ok "HTML v2 chrome + IA markers"

ok "Rocky 8 docker lab: NYTProf 5 + HTML v1 + scanner rc=0"
ok "rocky8_docker_profile_smoke"
