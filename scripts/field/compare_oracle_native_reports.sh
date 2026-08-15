#!/usr/bin/env bash
# Paired oracle 6.15 + native NYTProfM HTML reports — apples-to-apples.
#
# Same scanner, same --seconds, same corpus tree, same host. Isolated
# oracle PERL5LIB (never crates/). Native uses in-tree xs-nytprof.
#
#   ./scripts/field/compare_oracle_native_reports.sh
#   ./scripts/field/compare_oracle_native_reports.sh --seconds 25 --out ~/Downloads/nytprof-compare-apples
#
# Not in offline_gate. Not a public perf claim.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCANNER="$ROOT/scripts/field/workloads/minute_text_scanner.pl"
DEFAULT_OUT="${HOME}/Downloads/nytprof-compare-apples"
SECONDS_N=25
OUT="$DEFAULT_OUT"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

usage() {
  cat <<'EOF'
Usage: compare_oracle_native_reports.sh [--out DIR] [--seconds N]

Profile scripts/field/workloads/minute_text_scanner.pl twice on one
shared corpus: Devel::NYTProf 6.15 (oracle pin) then NYTProfM (in-tree
XS). Write DIR/{oracle,native}/html plus COMPARE.txt.

  --out DIR      default ~/Downloads/nytprof-compare-apples
  --seconds N    scanner wall seconds (default 25; both sides)

Never puts crates/ on oracle PERL5LIB.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out)
      [[ $# -ge 2 ]] || fail "--out requires a path"
      OUT="$2"
      shift 2
      ;;
    --seconds)
      [[ $# -ge 2 ]] || fail "--seconds requires an integer"
      SECONDS_N="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "unknown argument: $1"
      ;;
  esac
done

[[ -f "$SCANNER" ]] || fail "missing $SCANNER"
[[ -f "$ROOT/tools/oracle/env.sh" ]] || fail "missing tools/oracle/env.sh"
[[ -f "$ROOT/baseline/6.15/install/bin/nytprofhtml" ]] \
  || fail "oracle pin missing nytprofhtml (build baseline/6.15)"

NYTP_DEST="$ROOT/collector/build/xs-nytprof"
NYTP_SO="$NYTP_DEST/auto/Devel/NYTProfM/NYTProfM.so"
if [[ ! -f "$NYTP_SO" ]]; then
  log "building in-tree xs-nytprof"
  make -C "$ROOT/collector" xs-nytprof
fi
[[ -f "$NYTP_SO" ]] || fail "missing $NYTP_SO"

DUMP=""
if [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
  DUMP="$ROOT/target/debug/nytprof-dump"
elif [[ -x "$ROOT/target/release/nytprof-dump" ]]; then
  DUMP="$ROOT/target/release/nytprof-dump"
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  DUMP="$ROOT/prefix/bin/nytprof-cli"
else
  fail "missing nytprof-dump / nytprof-cli (build nytprof-cli)"
fi

mkdir -p "$OUT"/{app,corpus,oracle/{html,meta},native/{html,meta}}
OUT="$(cd "$OUT" && pwd)"
cp -a "$SCANNER" "$OUT/app/minute_text_scanner.pl"
chmod 755 "$OUT/app/minute_text_scanner.pl"

SEED="$OUT/meta-seed.txt"
mkdir -p "$OUT"
perl -e '
  print "It is a truth universally acknowledged that a profiler in want of a report must be in search of a long-running Perl application.\n" x 400;
  for my $i (1..200) {
    print "Record $i: sub process { my (\$line) = @_; \$line =~ s/\\s+/ /g; return length \$line }\n";
  }
' >"$SEED"
[[ -s "$SEED" ]] || fail "empty shared seed"
rm -rf "$OUT/corpus"
mkdir -p "$OUT/corpus"
cp -a "$SEED" "$OUT/corpus/chapter-1.txt"
cp -a "$SEED" "$OUT/corpus/chapter-2.txt"
FILE_COUNT="$(find "$OUT/corpus" -type f | wc -l)"

assert_no_crates() {
  case ":${PERL5LIB-}:" in
    *"/crates/"*) fail "PERL5LIB must not contain crates/: $PERL5LIB" ;;
  esac
}

log "=== oracle 6.15: ${SECONDS_N}s, files=${FILE_COUNT} ==="
# Isolate: do not inherit a dirty PERL5LIB into the oracle pin.
unset PERL5LIB || true
# shellcheck disable=SC1091
source "$ROOT/tools/oracle/env.sh"
assert_no_crates
case ":${PERL5LIB-}:" in
  *"/crates/"*) fail "oracle env.sh leaked crates/" ;;
esac
command -v nytprofhtml >/dev/null || fail "nytprofhtml not on PATH after env.sh"

set +e
(
  cd "$OUT"
  NYTPROF="file=${OUT}/oracle/nytprof.out" \
    perl -d:NYTProf "$OUT/app/minute_text_scanner.pl" "$OUT/corpus" "$SECONDS_N" \
    >"$OUT/oracle/meta/scanner.out" 2>"$OUT/oracle/meta/scanner.err"
)
ORC=$?
set -e
[[ "$ORC" -eq 0 ]] || fail "oracle perl -d:NYTProf rc=$ORC (see oracle/meta/scanner.err)"
[[ -s "$OUT/oracle/nytprof.out" ]] || fail "oracle missing nytprof.out"
nytprofhtml -o "$OUT/oracle/html" -f "$OUT/oracle/nytprof.out" \
  >"$OUT/oracle/meta/nytprofhtml.out" 2>"$OUT/oracle/meta/nytprofhtml.err" \
  || fail "oracle nytprofhtml failed"
[[ -f "$OUT/oracle/html/index.html" ]] || fail "oracle missing html/index.html"
printf '%s\n' "$PERL5LIB" >"$OUT/oracle/meta/perl5lib.txt"
ok "oracle HTML $OUT/oracle/html/index.html"

log "=== native NYTProfM: ${SECONDS_N}s, same corpus ==="
unset PERL5LIB || true
export PERL5LIB="$NYTP_DEST"
assert_no_crates
set +e
(
  cd "$OUT"
  NYTPROF="file=${OUT}/native/nytprof.out" \
    perl -I"$NYTP_DEST" -d:NYTProfM "$OUT/app/minute_text_scanner.pl" \
    "$OUT/corpus" "$SECONDS_N" \
    >"$OUT/native/meta/scanner.out" 2>"$OUT/native/meta/scanner.err"
)
NRC=$?
set -e
[[ "$NRC" -eq 0 ]] || fail "native perl -d:NYTProfM rc=$NRC (see native/meta/scanner.err)"
[[ -s "$OUT/native/nytprof.out" ]] || fail "native missing nytprof.out"
"$DUMP" html "$OUT/native/nytprof.out" --out-dir "$OUT/native/html" --flame \
  >"$OUT/native/meta/html.out" 2>"$OUT/native/meta/html.err" \
  || fail "native html --out-dir --flame failed"
[[ -f "$OUT/native/html/index.html" ]] || fail "native missing html/index.html"
ok "native HTML $OUT/native/html/index.html"

{
  echo "NYTProf apples-to-apples comparison"
  echo "=================================="
  echo
  echo "Date (UTC):     $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "Seconds:        ${SECONDS_N}  (both sides)"
  echo "Scanner:        scripts/field/workloads/minute_text_scanner.pl"
  echo "Corpus files:   ${FILE_COUNT}  (shared $OUT/corpus)"
  echo "Host:           $(uname -srm)"
  echo
  echo "Oracle:  perl -d:NYTProf  + oracle nytprofhtml  (tools/oracle/env.sh)"
  echo "         $OUT/oracle/html/index.html"
  echo "Native:  perl -d:NYTProfM + nytprof-dump html   (PERL5LIB=$NYTP_DEST)"
  echo "         $OUT/native/html/index.html"
  echo
  echo "Oracle scanner stdout:"
  cat "$OUT/oracle/meta/scanner.out"
  echo "Native scanner stdout:"
  cat "$OUT/native/meta/scanner.out"
  echo
  echo "Oracle summary line:"
  grep -o 'Profile of[^<]*' "$OUT/oracle/html/index.html" | head -1 || true
  echo "Native summary line:"
  grep -o 'Profile of[^<]*' "$OUT/native/html/index.html" | head -1 || true
  echo
  echo "Same requested wall and corpus. Pass counts may differ because"
  echo "6.15 instrumentation is heavier (fewer passes in the same seconds)."
  echo "Do not compare this pair to a different --seconds or corpus size."
} >"$OUT/COMPARE.txt"

ok "COMPARE.txt $OUT/COMPARE.txt"
ok "open $OUT/oracle/html/index.html  and  $OUT/native/html/index.html"
cat "$OUT/COMPARE.txt"
