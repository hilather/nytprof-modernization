#!/usr/bin/env bash
# Live tokenize-shaped exclusive split: parent excl ≠ CORE:match excl.
#
# Drives real `perl -d:NYTProfM` (product tree) on a named sub that performs
# many regex matches. Dump/report (not HTML rescale) must show:
#   CORE:match excl > 0
#   parent excl > 0
#   parent incl > parent excl
#   parent excl is not within ~10% of match excl when match dominates incl
#
# Isolated product PERL5LIB=collector/build/xs-nytprof. Never crates/.
# collection_default stays v5.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

COLLECTOR="$ROOT/collector"
MAKEFILE="$COLLECTOR/Makefile"
NYTP_DEST="$COLLECTOR/build/xs-nytprof"
NYTP_SO="$NYTP_DEST/auto/Devel/NYTProfM/NYTProfM.so"
NYTP_PM_SRC="$COLLECTOR/xs/Devel/NYTProfM.pm"
NYTP_XS="$COLLECTOR/xs/NYTProf.xs"

usage() {
  cat <<'EOF'
Usage: g09_tokenize_excl_smoke.sh

MATCH-inside-sub exclusive split under perl -d:NYTProfM.
EOF
}

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

# g17 overlays entersub=1 (these smokes hardcode file= only).
nytprof_attach() {
  local spec="$1"
  if [[ -n "${NYTPROF_ATTACH_OPTS:-}" ]]; then
    printf '%s:%s' "$spec" "${NYTPROF_ATTACH_OPTS}"
  else
    printf '%s' "$spec"
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    *) printf 'ERROR: unknown flag: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

echo "g09_tokenize_excl_smoke: repo root $ROOT"
echo "never crates/ on PERL5LIB; collection_default remains v5"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$MAKEFILE" ]] || fail "missing $MAKEFILE"
[[ -f "$NYTP_XS" ]] || fail "missing $NYTP_XS"
[[ -f "$NYTP_PM_SRC" ]] || fail "missing $NYTP_PM_SRC"
grep -q 'take_pending_child_excl' "$NYTP_XS" \
  || fail "NYTProf.xs missing take_pending_child_excl"
grep -q 'take_pending_child_excl' "$NYTP_PM_SRC" \
  || fail "NYTProfM.pm missing take_pending_child_excl"
if grep -E -q 'emit_sub_callers\(\s*1,\s*1,' "$NYTP_PM_SRC"; then
  fail "NYTProfM.pm still hardcodes emit_sub_callers(1, 1, …)"
fi
ok "PR-9 sources: pending child excl + real caller sites"

resolve_cc() {
  if [[ -n "${CC-}" ]] && command -v "$CC" >/dev/null 2>&1; then
    printf '%s\n' "$CC"
    return 0
  fi
  for c in cc gcc clang; do
    if command -v "$c" >/dev/null 2>&1; then
      printf '%s\n' "$c"
      return 0
    fi
  done
  return 1
}

print_residuals() {
  echo "NOT-YET: full 6.15 opcode/entersub / full slowops.h"
}

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain — G09 debugger .so not built"
  print_residuals
  ok "g09_tokenize_excl_smoke completed (skip — no CC)"
  exit 0
fi
have_xs_headers=0
if command -v perl >/dev/null 2>&1; then
  if perl -MConfig -e 'exit((-f "$Config{archlibexp}/CORE/EXTERN.h") ? 0 : 1)'; then
    have_xs_headers=1
  fi
fi
if [[ "$have_xs_headers" -ne 1 ]]; then
  echo "SKIP: perl XS headers not present — G09 debugger .so not built"
  print_residuals
  ok "g09_tokenize_excl_smoke completed (skip — no XS headers)"
  exit 0
fi

echo "make -C collector xs-nytprof"
make -C "$COLLECTOR" xs-nytprof
[[ -f "$NYTP_SO" ]] || fail "xs-nytprof did not produce $NYTP_SO"
ok "xs-nytprof produced .so"

CLI_CMD=()
if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
  CLI_CMD=("${NYTPROF_NATIVE_CLI}")
elif [[ -x "$ROOT/target/debug/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/target/debug/nytprof-cli")
elif [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
  CLI_CMD=("$ROOT/target/debug/nytprof-dump")
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/prefix/bin/nytprof-cli")
elif command -v cargo >/dev/null 2>&1 && [[ -f "$ROOT/Cargo.toml" ]]; then
  CLI_CMD=(cargo run -q -p nytprof-cli --)
else
  fail "no shipped dump/report CLI"
fi

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g09-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

unset PERL5OPT || true
export PERL5LIB="$NYTP_DEST"

PROFILE="$WORKDIR/nytprof.out"
DUMP="$WORKDIR/dump.jsonl"
REPORT_TXT="$WORKDIR/report.txt"
SCRIPT="$WORKDIR/tokenize_match.pl"

cat >"$SCRIPT" <<'PERL'
use strict;
use warnings;

sub tokenize {
    my ($text) = @_;
    my $n = 0;
    for (1 .. 4000) {
        $n++ if $text =~ /foo/;
    }
    return $n;
}

my $hits = tokenize("xxxxxxxxxxfoo");
print "g09_hits=$hits\n";
PERL

set +e
RUN_OUT="$(
  cd "$WORKDIR" && NYTPROF="$(nytprof_attach "file=${PROFILE}")" perl -I"$NYTP_DEST" -d:NYTProfM "$SCRIPT" 2>&1
)"
RUN_RC=$?
set -e
printf '%s\n' "$RUN_OUT"
[[ "$RUN_RC" -eq 0 ]] || fail "perl -d:NYTProfM tokenize script exited $RUN_RC"
grep -q '^g09_hits=' <<<"$RUN_OUT" || fail "script missing g09_hits="
[[ -s "$PROFILE" ]] || fail "missing profile"
head -c 9 "$PROFILE" | grep -q 'NYTProf 5' || fail "not NYTProf 5"

"${CLI_CMD[@]}" dump "$PROFILE" >"$DUMP" 2>"$DUMP.err" \
  || { cat "$DUMP.err" >&2; fail "dump failed"; }
"${CLI_CMD[@]}" report "$PROFILE" >"$REPORT_TXT" 2>"$REPORT_TXT.err" \
  || { cat "$REPORT_TXT.err" >&2; fail "report failed"; }

TIMES="$(perl - "$DUMP" "$REPORT_TXT" <<'PERL'
use strict;
use warnings;
use JSON::PP;

my ($dump, $report) = @ARGV;
my %sub;
open my $fh, "<", $dump or die "open $dump: $!\n";
while (<$fh>) {
    my $j = decode_json($_);
    next unless ($j->{tag} // "") eq "SUB_RETURN";
    my $args = $j->{args} // [];
    my $name = $args->[3] // next;
    my $incl = $args->[1] // 0;
    my $excl = $args->[2] // 0;
    $sub{$name}{incl} += $incl;
    $sub{$name}{excl} += $excl;
    $sub{$name}{n}++;
}
close $fh;

my ($pname, $p) = (undef, undef);
for my $n (keys %sub) {
    next unless $n =~ /(?:^|::)tokenize\z/;
    $pname = $n;
    $p = $sub{$n};
    last;
}
my ($mname, $m) = (undef, undef);
for my $n (sort keys %sub) {
    next unless $n =~ /CORE:match/;
    $mname = $n;
    $m = $sub{$n};
    last;
}
if (!$p || !$m) {
    print "PARSE_OK=0\n";
    print "HAVE_PARENT=", ($p ? 1 : 0), "\n";
    print "HAVE_MATCH=", ($m ? 1 : 0), "\n";
    exit 0;
}
printf "PARENT %s returns=%d incl=%s excl=%s\n", $pname, $p->{n}, $p->{incl}, $p->{excl};
printf "MATCH %s returns=%d incl=%s excl=%s\n", $mname, $m->{n}, $m->{incl}, $m->{excl};
print "PARSE_OK=1\n";
print "PARENT_INCL=$p->{incl}\n";
print "PARENT_EXCL=$p->{excl}\n";
print "MATCH_EXCL=$m->{excl}\n";
PERL
)"
printf '%s\n' "$TIMES"
[[ "$(perl -ne 'print $1 if /^PARSE_OK=(\d+)/' <<<"$TIMES")" == "1" ]] \
  || fail "dump missing tokenize and/or CORE:match SUB_RETURN"

PARENT_INCL="$(perl -ne 'print $1 if /^PARENT_INCL=(.*)/' <<<"$TIMES")"
PARENT_EXCL="$(perl -ne 'print $1 if /^PARENT_EXCL=(.*)/' <<<"$TIMES")"
MATCH_EXCL="$(perl -ne 'print $1 if /^MATCH_EXCL=(.*)/' <<<"$TIMES")"

perl -e '
  my ($pi, $pe, $me) = @ARGV;
  die "parent incl must be > 0 (got $pi)\n" unless $pi > 0;
  die "parent excl must be > 0 (got $pe)\n" unless $pe > 0;
  die "match excl must be > 0 (got $me)\n" unless $me > 0;
  die "parent incl ($pi) must exceed parent excl ($pe)\n" unless $pi > $pe;
  if ($me > 0.5 * $pi) {
    my $rel = abs($pe - $me) / $me;
    die "parent excl ($pe) is within 10% of match excl ($me) while match dominates incl ($pi)\n"
      unless $rel > 0.10;
  }
  print "SPLIT_OK pe=$pe me=$me pi=$pi\n";
' "$PARENT_INCL" "$PARENT_EXCL" "$MATCH_EXCL" \
  || fail "exclusive split failed (see dump $DUMP)"

grep -E -q 'tokenize' "$REPORT_TXT" || fail "report missing tokenize"
grep -E -q 'CORE:match' "$REPORT_TXT" || fail "report missing CORE:match"
ok "tokenize excl split vs CORE:match (dump/report, not HTML)"

SITES="$(perl - "$DUMP" <<'PERL'
use strict;
use warnings;
use JSON::PP;
my $dump = $ARGV[0];
my ($non_stub, $stub11, $total) = (0, 0, 0);
open my $fh, "<", $dump or die $!;
while (<$fh>) {
    my $j = decode_json($_);
    next unless ($j->{tag} // "") eq "SUB_CALLERS";
    my $a = $j->{args} // [];
    my $fid = $a->[0] // 0;
    my $line = $a->[1] // 0;
    my $called = $a->[7] // "";
    my $caller = $a->[8] // "";
    next unless $called =~ /tokenize|CORE:match/ || $caller =~ /tokenize/;
    $total++;
    if ($fid == 1 && $line == 1) { $stub11++; }
    else { $non_stub++; }
    print "SITE fid=$fid line=$line caller=$caller called=$called\n" if $total <= 8;
}
print "SITE_TOTAL=$total\nSITE_NONSTUB=$non_stub\nSITE_STUB11=$stub11\n";
PERL
)"
printf '%s\n' "$SITES"
NONSTUB="$(perl -ne 'print $1 if /^SITE_NONSTUB=(\d+)/' <<<"$SITES")"
[[ -n "$NONSTUB" && "$NONSTUB" -gt 0 ]] \
  || fail "Perl/slowop SUB_CALLERS still all stub (1,1) for tokenize/match"

HTML_DIR="$WORKDIR/html"
"${CLI_CMD[@]}" html "$PROFILE" --out-dir "$HTML_DIR" \
  >"$WORKDIR/html.out" 2>"$WORKDIR/html.err" \
  || { cat "$WORKDIR/html.err" >&2; fail "html --out-dir failed"; }
[[ -f "$HTML_DIR/index.html" ]] || fail "missing html index"
SRC_HTML="$(ls -1 "$HTML_DIR"/file-*.html "$HTML_DIR"/source.html 2>/dev/null | head -1 || true)"
[[ -n "$SRC_HTML" ]] || fail "no source pages"
if grep -q 'class="calls' "$HTML_DIR"/file-*.html "$HTML_DIR"/source.html 2>/dev/null; then
  if grep -E 'href="file-1.html#L1"' "$HTML_DIR"/file-*.html 2>/dev/null | grep -q 'warnings'; then
    fail "call-in/out linked to warnings.pm L1"
  fi
  ok "HTML has .calls annotations from usable sites"
else
  echo "note: no .calls on this tiny script (sites may land on opcode stubs only)"
  grep -q 'class="calls' "$HTML_DIR"/*.html && ok "calls on a page" \
    || fail "html missing .calls despite non-stub sites"
fi

print_residuals
ok "G09 tokenize exclusive split"
exit 0
