#!/usr/bin/env bash
# Pure-Perl JsonlData A9 sub_defs + NEW_FID files smoke (PERL-SUBDEFS-JSONL).
#
# Spec: docs/schemas/perl-jsonl-data-mvp-v0.md
#
# 1. Prove golden JSONL path: leaf 1/3–7, mid 1/8–12, file 1 → workload.pl
# 2. Optional: regenerate dump via native CLI and re-query the same ranges
#
# Does NOT put crates/ on oracle PERL5LIB. Does NOT require oracle Devel::NYTProf.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/perl_subdefs_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE_DIR="fixtures/v5/default-calls1"
GOLDEN="$FIXTURE_DIR/readstream.jsonl"
PROFILE="$FIXTURE_DIR/nytprof.out"
PM="perl/lib/Devel/NYTProf/JsonlData.pm"
STREAM_PM="perl/lib/Devel/NYTProf/JsonlReadStream.pm"
T="perl/t/jsonl_data_subdefs_default_calls1.t"
LIB="perl/lib"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$GOLDEN" ]] || fail "missing golden dump $GOLDEN"
[[ -f "$ROOT/$PM" ]] || fail "missing $PM"
[[ -f "$ROOT/$STREAM_PM" ]] || fail "missing $STREAM_PM"
[[ -f "$ROOT/$T" ]] || fail "missing $T"

# ---------------------------------------------------------------------------
# 1. Pure-Perl test against committed golden JSONL (no native CLI required)
# ---------------------------------------------------------------------------
echo "=== JsonlData subdefs: golden JSONL path ==="
prove -I"$LIB" "$T" || fail "prove $T failed"
ok "prove $T (golden readstream.jsonl)"

# Explicit aggregation print for operator evidence
AGG_OUT="$(
  perl -I"$LIB" -MDevel::NYTProf::JsonlData -e '
    my $d = Devel::NYTProf::JsonlData->from_jsonl(shift);
    my $leaf = $d->sub_def("main::leaf") // {};
    my $mid  = $d->sub_def("main::mid")  // {};
    printf "main::leaf fid=%s first=%s last=%s\n",
      $leaf->{fid} // "?", $leaf->{first_line} // "?", $leaf->{last_line} // "?";
    printf "main::mid fid=%s first=%s last=%s\n",
      $mid->{fid} // "?", $mid->{first_line} // "?", $mid->{last_line} // "?";
    printf "file(1)=%s\n", $d->file(1) // "";
    printf "file_basename(1)=%s\n", $d->file_basename(1) // "";
    printf "records_seen=%d\n", $d->records_seen;
  ' "$GOLDEN"
)"
echo "$AGG_OUT"
echo "$AGG_OUT" | grep -qE 'main::leaf fid=1 first=3 last=7' \
  || fail "golden missing main::leaf fid=1 first=3 last=7"
echo "$AGG_OUT" | grep -qE 'main::mid fid=1 first=8 last=12' \
  || fail "golden missing main::mid fid=1 first=8 last=12"
echo "$AGG_OUT" | grep -qE 'file_basename\(1\)=workload\.pl' \
  || fail "golden missing file_basename(1)=workload.pl"
echo "$AGG_OUT" | grep -qE 'workload\.pl' \
  || fail "golden file path missing workload.pl"
ok "golden JsonlData: leaf 1/3-7 mid 1/8-12 file basename workload.pl"

# ---------------------------------------------------------------------------
# 2. Native CLI dump path (when cargo or a built binary is available)
# ---------------------------------------------------------------------------
TMPDIR_SMOKE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT
NATIVE_JSONL="$TMPDIR_SMOKE/native_dump.jsonl"

find_cli() {
  if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
    echo "path:${NYTPROF_NATIVE_CLI}"
    return 0
  fi
  for p in \
    prefix/bin/nytprof-cli \
    prefix/bin/nytprof-dump \
    target/release/nytprof-dump \
    target/debug/nytprof-dump
  do
    if [[ -x "$ROOT/$p" || -f "$ROOT/$p" ]]; then
      echo "path:$ROOT/$p"
      return 0
    fi
  done
  if command -v cargo >/dev/null 2>&1; then
    echo "cargo"
    return 0
  fi
  return 1
}

if CLI_SPEC="$(find_cli)"; then
  echo "=== JsonlData subdefs: native CLI dump path ($CLI_SPEC) ==="
  [[ -f "$ROOT/$PROFILE" ]] || fail "missing profile $PROFILE"

  set +e
  if [[ "$CLI_SPEC" == cargo ]]; then
    cargo run -q -p nytprof-cli -- dump "$PROFILE" >"$NATIVE_JSONL" 2>"$TMPDIR_SMOKE/dump.err"
    DUMP_RC=$?
  else
    CLI_PATH="${CLI_SPEC#path:}"
    "$CLI_PATH" dump "$PROFILE" >"$NATIVE_JSONL" 2>"$TMPDIR_SMOKE/dump.err"
    DUMP_RC=$?
  fi
  set -e

  if [[ "$DUMP_RC" -ne 0 ]]; then
    cat "$TMPDIR_SMOKE/dump.err" >&2 || true
    fail "native dump failed (rc=$DUMP_RC)"
  fi
  [[ -s "$NATIVE_JSONL" ]] || fail "native dump produced empty JSONL"

  NATIVE_AGG="$(
    perl -I"$LIB" -MDevel::NYTProf::JsonlData -e '
      my $d = Devel::NYTProf::JsonlData->from_jsonl(shift);
      my $leaf = $d->sub_def("main::leaf") // {};
      my $mid  = $d->sub_def("main::mid")  // {};
      printf "main::leaf fid=%s first=%s last=%s\n",
        $leaf->{fid} // "?", $leaf->{first_line} // "?", $leaf->{last_line} // "?";
      printf "main::mid fid=%s first=%s last=%s\n",
        $mid->{fid} // "?", $mid->{first_line} // "?", $mid->{last_line} // "?";
      printf "file_basename(1)=%s\n", $d->file_basename(1) // "";
    ' "$NATIVE_JSONL"
  )"
  echo "$NATIVE_AGG"
  echo "$NATIVE_AGG" | grep -qE 'main::leaf fid=1 first=3 last=7' \
    || fail "native dump missing main::leaf fid=1 first=3 last=7"
  echo "$NATIVE_AGG" | grep -qE 'main::mid fid=1 first=8 last=12' \
    || fail "native dump missing main::mid fid=1 first=8 last=12"
  echo "$NATIVE_AGG" | grep -qE 'file_basename\(1\)=workload\.pl' \
    || fail "native dump missing file_basename(1)=workload.pl"
  ok "native dump JsonlData: leaf 1/3-7 mid 1/8-12 basename workload.pl"
else
  echo "NOTE: no native CLI / cargo; skipped live dump path (golden path still required)"
fi

ok "perl JsonlData subdefs packaging smoke passed"
exit 0
