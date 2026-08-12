#!/usr/bin/env bash
# Product XS Data / ReadStream packaging smoke (PR-A06 / OQ-2 / PERL-XS-DATA-READSTREAM-MVP).
#
# Spec: docs/schemas/perl-xs-data-readstream-mvp-v0.md
#
# 1. Prove golden JSONL product path: leaf=15, mid=3, mid→leaf=15
# 2. Prove blocks-calls1 A4/A4b greppable ints
# 3. Optional: binary from_profile / filename when native CLI available
#
# Does NOT put crates/ on oracle PERL5LIB. Does NOT require oracle Devel::NYTProf.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/perl_xs_data_readstream_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

LIB="perl/lib"
DATA_PM="perl/lib/Devel/NYTProf/Data.pm"
RS_PM="perl/lib/Devel/NYTProf/ReadStream.pm"
T_DATA="perl/t/data_product_default_calls1.t"
T_RS="perl/t/readstream_product_default_calls1.t"
T_BLOCKS="perl/t/data_product_blocks_calls1.t"
GOLDEN="fixtures/v5/default-calls1/readstream.jsonl"
PROFILE="fixtures/v5/default-calls1/nytprof.out"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$DATA_PM" ]] || fail "missing $DATA_PM"
[[ -f "$ROOT/$RS_PM" ]] || fail "missing $RS_PM"
[[ -f "$ROOT/$T_DATA" ]] || fail "missing $T_DATA"
[[ -f "$ROOT/$T_RS" ]] || fail "missing $T_RS"
[[ -f "$ROOT/$T_BLOCKS" ]] || fail "missing $T_BLOCKS"
[[ -f "$ROOT/$GOLDEN" ]] || fail "missing $GOLDEN"

echo "=== Product Data/ReadStream: unit tests (golden + optional binary) ==="
prove -I"$LIB" "$T_DATA" || fail "prove $T_DATA failed"
ok "prove $T_DATA"
prove -I"$LIB" "$T_RS" || fail "prove $T_RS failed"
ok "prove $T_RS"
prove -I"$LIB" "$T_BLOCKS" || fail "prove $T_BLOCKS failed"
ok "prove $T_BLOCKS"

echo "=== Product Data: golden aggregation print ==="
AGG_OUT="$(
  perl -I"$LIB" -MDevel::NYTProf::Data -e '
    my $d = Devel::NYTProf::Data->from_jsonl(shift);
    printf "backend=%s\n", $d->backend;
    printf "materializer=%s\n", $d->materializer;
    printf "compat007=%d\n", $d->claims_compat007_shapes;
    printf "main::leaf returns=%d\n", $d->sub_returns("main::leaf");
    printf "main::mid returns=%d\n",  $d->sub_returns("main::mid");
    printf "mid->leaf edge=%d\n",
      $d->call_edge_count("main::mid", "main::leaf");
    printf "discount_events=%d\n", $d->discount_events;
  ' "$GOLDEN"
)"
echo "$AGG_OUT"
echo "$AGG_OUT" | grep -qE 'backend=jsonl-file' \
  || fail "expected backend=jsonl-file"
echo "$AGG_OUT" | grep -qE 'compat007=0' \
  || fail "expected claims_compat007_shapes=0"
echo "$AGG_OUT" | grep -qE 'main::leaf returns=15' \
  || fail "golden missing main::leaf returns=15"
echo "$AGG_OUT" | grep -qE 'main::mid returns=3' \
  || fail "golden missing main::mid returns=3"
echo "$AGG_OUT" | grep -qE 'mid->leaf edge=15' \
  || fail "golden missing mid->leaf edge=15"
echo "$AGG_OUT" | grep -qE 'discount_events=818' \
  || fail "golden missing discount_events=818"
ok "golden product Data: leaf=15 mid=3 mid→leaf=15 discount=818 compat007=0"

echo "=== Product ReadStream: golden SUB_RETURN recount ==="
RS_OUT="$(
  perl -I"$LIB" -MDevel::NYTProf::ReadStream=for_chunks,SUB_RETURN_SUBNAME_INDEX -e '
    my %r;
    for_chunks(
      sub {
        my ($tag, $args) = @_;
        if ($tag eq "SUB_RETURN" && @$args > SUB_RETURN_SUBNAME_INDEX) {
          my $n = $args->[SUB_RETURN_SUBNAME_INDEX];
          $r{$n}++ if defined $n;
        }
      },
      jsonl => shift,
    );
    printf "main::leaf returns=%d\n", $r{"main::leaf"} // 0;
    printf "main::mid returns=%d\n",  $r{"main::mid"}  // 0;
  ' "$GOLDEN"
)"
echo "$RS_OUT"
echo "$RS_OUT" | grep -qE 'main::leaf returns=15' \
  || fail "ReadStream golden missing leaf=15"
echo "$RS_OUT" | grep -qE 'main::mid returns=3' \
  || fail "ReadStream golden missing mid=3"
ok "golden product ReadStream: leaf=15 mid=3"

# ---------------------------------------------------------------------------
# Optional binary path (native CLI)
# ---------------------------------------------------------------------------
find_cli_path() {
  if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
    echo "${NYTPROF_NATIVE_CLI}"
    return 0
  fi
  for p in \
    prefix/bin/nytprof-cli \
    prefix/bin/nytprof-dump \
    target/release/nytprof-dump \
    target/debug/nytprof-dump
  do
    if [[ -x "$ROOT/$p" || -f "$ROOT/$p" ]]; then
      echo "$ROOT/$p"
      return 0
    fi
  done
  return 1
}

if [[ -f "$ROOT/$PROFILE" ]] && find_cli_path >/dev/null; then
  echo "=== Product Data: binary from_profile path ==="
  BIN_OUT="$(
    perl -I"$LIB" -MDevel::NYTProf::Data -e '
      my ($repo, $profile) = @ARGV;
      my $d = Devel::NYTProf::Data->from_profile($profile, repo_root => $repo);
      printf "backend=%s\n", $d->backend;
      printf "materializer=%s\n", $d->materializer;
      printf "main::leaf returns=%d\n", $d->sub_returns("main::leaf");
      printf "main::mid returns=%d\n",  $d->sub_returns("main::mid");
      printf "mid->leaf edge=%d\n",
        $d->call_edge_count("main::mid", "main::leaf");
    ' "$ROOT" "$PROFILE"
  )"
  echo "$BIN_OUT"
  echo "$BIN_OUT" | grep -qE 'backend=native-cli-jsonl' \
    || fail "binary path expected backend=native-cli-jsonl"
  echo "$BIN_OUT" | grep -qE 'main::leaf returns=15' \
    || fail "binary path missing leaf=15"
  echo "$BIN_OUT" | grep -qE 'mid->leaf edge=15' \
    || fail "binary path missing mid→leaf=15"
  ok "binary product Data: native-cli-jsonl leaf=15 mid→leaf=15"

  echo "=== Product ReadStream: binary filename path ==="
  BRS_OUT="$(
    perl -I"$LIB" -MDevel::NYTProf::ReadStream=for_chunks,SUB_RETURN_SUBNAME_INDEX -e '
      my ($repo, $profile) = @ARGV;
      my %r;
      for_chunks(
        sub {
          my ($tag, $args) = @_;
          if ($tag eq "SUB_RETURN" && @$args > SUB_RETURN_SUBNAME_INDEX) {
            my $n = $args->[SUB_RETURN_SUBNAME_INDEX];
            $r{$n}++ if defined $n;
          }
        },
        filename => $profile,
        repo_root => $repo,
      );
      printf "main::leaf returns=%d\n", $r{"main::leaf"} // 0;
    ' "$ROOT" "$PROFILE"
  )"
  echo "$BRS_OUT"
  echo "$BRS_OUT" | grep -qE 'main::leaf returns=15' \
    || fail "binary ReadStream missing leaf=15"
  ok "binary product ReadStream: leaf=15"
else
  echo "SKIP: binary product path (no native CLI binary or missing profile)"
  echo "  (golden JSONL product path above is required and passed)"
fi

ok "perl_xs_data_readstream_smoke complete (PR-A06 / PERL-XS-DATA-READSTREAM-MVP)"
