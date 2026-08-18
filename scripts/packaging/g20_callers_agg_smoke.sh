#!/usr/bin/env bash
# SUB_CALLERS are aggregated in C and emitted once per distinct edge at
# finish. SUB_RETURN stays 1:1 at return. Drives real perl -d:NYTProfM.
# Never crates/. collection_default stays v5. claim: none.
#
# Exit 0 pass or honest skip; 1 fail; 2 misuse.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

COLLECTOR="$ROOT/collector"
MAKEFILE="$COLLECTOR/Makefile"
NYTP_DEST="$COLLECTOR/build/xs-nytprof"
NYTP_SO="$NYTP_DEST/auto/Devel/NYTProfM/NYTProfM.so"
NYTP_PM_SRC="$COLLECTOR/xs/Devel/NYTProfM.pm"
NYTP_CORE_SRC="$COLLECTOR/xs/Devel/NYTProfM/Core.pm"
NYTP_XS="$COLLECTOR/xs/NYTProf.xs"
CALLERS_C="$COLLECTOR/xs/product_callers.c"
PP_C="$COLLECTOR/xs/pp_entersub.c"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

echo "g20_callers_agg_smoke: repo root $ROOT"
echo "collection_default remains v5; never crates/ on PERL5LIB"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$MAKEFILE" ]] || fail "missing $MAKEFILE"
[[ -f "$CALLERS_C" ]] || fail "missing $CALLERS_C"
[[ -f "$PP_C" ]] || fail "missing $PP_C"
[[ -f "$NYTP_XS" ]] || fail "missing $NYTP_XS"
[[ -f "$NYTP_PM_SRC" ]] || fail "missing $NYTP_PM_SRC"
[[ -f "$NYTP_CORE_SRC" ]] || fail "missing $NYTP_CORE_SRC"

grep -q 'product_callers_add' "$CALLERS_C" \
  || fail "product_callers.c missing product_callers_add"
grep -q 'product_callers_flush' "$CALLERS_C" \
  || fail "product_callers.c missing product_callers_flush"
grep -q 'sub_callers_hv' "$CALLERS_C" && fail "must not use Perl sub_callers_hv"
grep -q 'product_callers_add' "$PP_C" \
  || fail "pp_entersub.c must add callers (not emit per return)"
if grep -q 'nytp_emit_sub_callers' "$PP_C"; then
  fail "pp_entersub.c must not emit SUB_CALLERS per return"
fi
grep -q 'product_callers_flush' "$NYTP_XS" \
  || fail "NYTProf.xs must flush callers at finish"
grep -q 'product_callers.o' "$MAKEFILE" \
  || fail "Makefile missing product_callers.o"
ok "sources: C callers table + finish flush"

print_residuals() {
  echo "g04 15/3/15 still SUB_RETURN counts"
  echo "g17 unit-ratio still sums SUB_CALLERS excl vs leaf SUB_RETURN"
}

resolve_cc() {
  if [[ -n "${CC-}" ]] && command -v "$CC" >/dev/null 2>&1; then
    printf '%s\n' "$CC"; return 0
  fi
  for c in cc gcc clang; do
    command -v "$c" >/dev/null 2>&1 && { printf '%s\n' "$c"; return 0; }
  done
  return 1
}

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain"
  print_residuals
  ok "g20_callers_agg_smoke completed (skip — no CC)"
  exit 0
fi
if ! perl -MConfig -e 'exit((-f "$Config{archlibexp}/CORE/EXTERN.h") ? 0 : 1)'; then
  echo "SKIP: perl XS headers not present"
  print_residuals
  ok "g20_callers_agg_smoke completed (skip — no XS headers)"
  exit 0
fi

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
elif [[ -x "$ROOT/target/release/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/target/release/nytprof-cli")
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/prefix/bin/nytprof-cli")
elif command -v cargo >/dev/null 2>&1 && [[ -f "$ROOT/Cargo.toml" ]]; then
  CLI_CMD=(cargo run -q -p nytprof-cli --)
else
  fail "no shipped dump (nytprof-cli / nytprof-dump)"
fi

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-g20-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

N=20000
cat >"$WORKDIR/loop.pl" <<END
use strict;
use warnings;
my \$N = $N;
sub leaf { \$_[0] + 1 }
sub mid {
    my \$s = 0;
    \$s += leaf(\$_) for 1 .. \$N;
    return \$s;
}
print "g20_sum=", mid(0), "\n";
END

probe() {
  local label="$1"
  local nyt="$2"
  local profile="$WORKDIR/${label}.out"
  local dump="$WORKDIR/${label}.jsonl"
  unset PERL5LIB || true
  export PERL5LIB="$NYTP_DEST"
  NYTPROF="$nyt" perl -I"$NYTP_DEST" -d:NYTProfM "$WORKDIR/loop.pl" \
    >"$WORKDIR/${label}.stdout" 2>"$WORKDIR/${label}.stderr" \
    || fail "$label attach failed"
  [[ -s "$profile" ]] || fail "$label missing profile"
  "${CLI_CMD[@]}" dump "$profile" >"$dump" 2>"$WORKDIR/${label}.dump.err" \
    || fail "$label dump failed"
  perl -e '
    use strict;
    use warnings;
    use JSON::PP;
    my ($dump, $n, $label) = @ARGV;
    my $sr_leaf = 0;
    my $sc_events = 0;
    my $sc_mid_leaf_count = 0;
    my $sc_mid_leaf_excl = 0;
    my $sr_leaf_excl = 0;
    open my $fh, "<", $dump or die "$dump: $!";
    while (<$fh>) {
      my $j = decode_json($_);
      my $tag = $j->{tag} // next;
      my $a = $j->{args} // [];
      if ($tag eq "SUB_RETURN") {
        my $name = $a->[3] // "";
        if ($name =~ /(?:^|::)leaf\z/) {
          $sr_leaf++;
          $sr_leaf_excl += ($a->[2] // 0);
        }
      }
      elsif ($tag eq "SUB_CALLERS") {
        $sc_events++;
        my $called = $a->[7] // "";
        my $caller = $a->[8] // "";
        my $count = $a->[2] // 0;
        if ($called =~ /(?:^|::)leaf\z/ && $caller =~ /(?:^|::)mid\z/) {
          $sc_mid_leaf_count += $count;
          $sc_mid_leaf_excl += ($a->[4] // 0);
        }
      }
    }
    print "SR_LEAF=$sr_leaf\n";
    print "SC_EVENTS=$sc_events\n";
    print "SC_MID_LEAF_COUNT=$sc_mid_leaf_count\n";
    die "$label: leaf SUB_RETURN=$sr_leaf want $n\n" unless $sr_leaf == $n;
    die "$label: SUB_CALLERS events=$sc_events not aggregated (want < 30)\n"
      unless $sc_events > 0 && $sc_events < 30;
    die "$label: mid→leaf count=$sc_mid_leaf_count want $n\n"
      unless $sc_mid_leaf_count == $n;
    die "$label: empty excl\n"
      unless $sr_leaf_excl > 0 && $sc_mid_leaf_excl > 0;
    my $r = $sc_mid_leaf_excl / $sr_leaf_excl;
    die "$label: unit-ratio $r not in (0.5, 2)\n" unless $r > 0.5 && $r < 2;
    print "UNIT_RATIO=$r\n";
  ' "$dump" "$N" "$label" || fail "$label dump checks failed"
  ok "$label: leaf SUB_RETURN=$N, SUB_CALLERS events aggregated, count=$N"
}

probe opcode "file=${WORKDIR}/opcode.out:stmts=0"
probe wrap "file=${WORKDIR}/wrap.out:stmts=0:wrap=1"

print_residuals
ok "g20_callers_agg_smoke completed"
exit 0
