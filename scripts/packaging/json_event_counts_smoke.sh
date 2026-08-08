#!/usr/bin/env bash
# JSON-EVENT-COUNTS-MVP: expose dump/model-derived stream tag multiplicities on
# shipped JSON surfaces.
#
# Specs:
#   docs/schemas/perl-engine-dispatch-mvp-v0.md
#   docs/schemas/native-aggregates-json-mvp-v0.md
#   docs/schemas/perl-jsonl-data-mvp-v0.md
# Board: JSON-EVENT-COUNTS-MVP
#
# Contract (real CLIs only; no re-aggregation) on fixtures/v5/default-calls1:
#   sub_return_events  == 27
#   new_fid_events     == 3
#   sub_callers_events == 13
#   src_line_events    == 632
#   sub_info_events    == 31
#     (match stream recount / ProfileModel / JsonlData)
#
# Surfaces:
#   1) Perl   nytprof-engine query --json --jsonl <readstream.jsonl>  (required)
#   2) native nytprof-cli report --json <profile.out>               (optional)
#   3) optional golden tag recount from readstream.jsonl
#
# Never puts crates/ on oracle PERL5LIB. No XS.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/json_event_counts_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

DEFAULT_DIR="fixtures/v5/default-calls1"
DEFAULT_GOLDEN="$DEFAULT_DIR/readstream.jsonl"
DEFAULT_PROFILE="$DEFAULT_DIR/nytprof.out"
ENGINE_BIN="perl/bin/nytprof-engine"
ENGINE_LIB="perl/lib"
DISPATCH_PM="$ENGINE_LIB/Devel/NYTProf/EngineDispatch.pm"
DATA_PM="$ENGINE_LIB/Devel/NYTProf/JsonlData.pm"

# default-calls1 stream-recount contract
WANT_SUB_RETURN=27
WANT_NEW_FID=3
WANT_SUB_CALLERS=13
WANT_SRC_LINE=632
WANT_SUB_INFO=31

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$DEFAULT_GOLDEN" ]] || fail "missing golden dump $DEFAULT_GOLDEN"
[[ -f "$ROOT/$ENGINE_BIN" ]] || fail "missing $ENGINE_BIN"
[[ -f "$ROOT/$DISPATCH_PM" ]] || fail "missing $DISPATCH_PM"
[[ -f "$ROOT/$DATA_PM" ]] || fail "missing $DATA_PM"
command -v perl >/dev/null 2>&1 || fail "perl not on PATH"

# Sanity: never inject crates/ into oracle PERL5LIB from this smoke.
if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ (got: $PERL5LIB)"
fi

ENGINE=(perl -I"$ENGINE_LIB" "$ENGINE_BIN")
TMPDIR_SMOKE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT

# ---------------------------------------------------------------------------
# JsonlData expected values (source of truth for pure-Perl path)
# ---------------------------------------------------------------------------
eval "$(
  perl -I"$ENGINE_LIB" -MDevel::NYTProf::JsonlData -e '
    use strict; use warnings;
    my $d = Devel::NYTProf::JsonlData->from_jsonl($ARGV[0]);
    my %want = (
      sub_return_events  => 0+$ARGV[1],
      new_fid_events     => 0+$ARGV[2],
      sub_callers_events => 0+$ARGV[3],
      src_line_events    => 0+$ARGV[4],
      sub_info_events    => 0+$ARGV[5],
    );
    for my $k (sort keys %want) {
      my $got = 0 + ($d->$k // 0);
      die "JsonlData $k=$got want $want{$k}\n" unless $got == $want{$k};
      print "EXPECT_\U$k\E=$got\n";
    }
  ' "$DEFAULT_GOLDEN" \
    "$WANT_SUB_RETURN" "$WANT_NEW_FID" "$WANT_SUB_CALLERS" \
    "$WANT_SRC_LINE" "$WANT_SUB_INFO"
)" || fail "failed to load JsonlData event counts from golden"

ok "JsonlData expect: sub_return=$WANT_SUB_RETURN new_fid=$WANT_NEW_FID sub_callers=$WANT_SUB_CALLERS src_line=$WANT_SRC_LINE sub_info=$WANT_SUB_INFO"

# ---------------------------------------------------------------------------
# Optional independent golden tag recount
# ---------------------------------------------------------------------------
if command -v python3 >/dev/null 2>&1; then
  python3 -c '
import json,sys
path = sys.argv[1]
want = {
    "SUB_RETURN": int(sys.argv[2]),
    "NEW_FID": int(sys.argv[3]),
    "SUB_CALLERS": int(sys.argv[4]),
    "SRC_LINE": int(sys.argv[5]),
    "SUB_INFO": int(sys.argv[6]),
}
c = {k: 0 for k in want}
with open(path, encoding="utf-8") as fh:
    for line in fh:
        line = line.strip()
        if not line:
            continue
        o = json.loads(line)
        t = o.get("tag")
        if t in c:
            c[t] += 1
for t, w in want.items():
    if c[t] != w:
        raise SystemExit("%s: %s tag count %s want %s" % (path, t, c[t], w))
    print("golden_recount_ok", t, c[t])
' "$DEFAULT_GOLDEN" \
    "$WANT_SUB_RETURN" "$WANT_NEW_FID" "$WANT_SUB_CALLERS" \
    "$WANT_SRC_LINE" "$WANT_SUB_INFO" \
    || fail "golden tag recount mismatch"
  ok "golden tag recount: SUB_RETURN=$WANT_SUB_RETURN NEW_FID=$WANT_NEW_FID SUB_CALLERS=$WANT_SUB_CALLERS SRC_LINE=$WANT_SRC_LINE SUB_INFO=$WANT_SUB_INFO"
else
  log "NOTE: no python3 — skip independent golden tag recount"
fi

# ---------------------------------------------------------------------------
# Assert helpers
# ---------------------------------------------------------------------------
json_assert_event_counts() {
  local f="$1"
  local label="$2"
  if command -v python3 >/dev/null 2>&1; then
    python3 -c '
import json,sys
path, label = sys.argv[1], sys.argv[2]
want = {
    "sub_return_events": int(sys.argv[3]),
    "new_fid_events": int(sys.argv[4]),
    "sub_callers_events": int(sys.argv[5]),
    "src_line_events": int(sys.argv[6]),
    "sub_info_events": int(sys.argv[7]),
}
o = json.load(open(path, encoding="utf-8"))
if o.get("ok") is not True:
    raise SystemExit("%s: ok must be true, got %r" % (label, o.get("ok")))
for k, w in want.items():
    got = o.get(k)
    if got != w:
        raise SystemExit("%s: %s must be %s, got %r" % (label, k, w, got))
print("event_counts_ok", label,
      "sub_return=%s new_fid=%s sub_callers=%s src_line=%s sub_info=%s" % (
          o["sub_return_events"], o["new_fid_events"], o["sub_callers_events"],
          o["src_line_events"], o["sub_info_events"]))
' "$f" "$label" \
      "$WANT_SUB_RETURN" "$WANT_NEW_FID" "$WANT_SUB_CALLERS" \
      "$WANT_SRC_LINE" "$WANT_SUB_INFO" \
      || fail "$label: event counts assert failed
$(cat "$f")"
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      use strict; use warnings;
      my ($f, $label, $sr, $nf, $sc, $sl, $si) = @ARGV;
      open my $fh, "<", $f or die "$label: $!";
      local $/; my $obj = JSON::PP->new->decode(<$fh>);
      die "$label: ok\n" unless $obj->{ok};
      my %want = (
        sub_return_events  => 0+$sr,
        new_fid_events     => 0+$nf,
        sub_callers_events => 0+$sc,
        src_line_events    => 0+$sl,
        sub_info_events    => 0+$si,
      );
      for my $k (sort keys %want) {
        my $got = $obj->{$k};
        die "$label: $k missing\n" unless defined $got;
        die "$label: $k want $want{$k} got $got\n" unless 0+$got == $want{$k};
      }
      printf "event_counts_ok %s sub_return=%s new_fid=%s sub_callers=%s src_line=%s sub_info=%s\n",
        $label, $obj->{sub_return_events}, $obj->{new_fid_events},
        $obj->{sub_callers_events}, $obj->{src_line_events}, $obj->{sub_info_events};
    ' "$f" "$label" \
      "$WANT_SUB_RETURN" "$WANT_NEW_FID" "$WANT_SUB_CALLERS" \
      "$WANT_SRC_LINE" "$WANT_SUB_INFO" \
      || fail "$label: event counts assert failed (perl)
$(cat "$f")"
  else
    grep -qE "\"sub_return_events\"[[:space:]]*:[[:space:]]*${WANT_SUB_RETURN}" "$f" \
      || fail "$label: missing sub_return_events:$WANT_SUB_RETURN\n$(cat "$f")"
    grep -qE "\"new_fid_events\"[[:space:]]*:[[:space:]]*${WANT_NEW_FID}" "$f" \
      || fail "$label: missing new_fid_events:$WANT_NEW_FID\n$(cat "$f")"
    grep -qE "\"sub_callers_events\"[[:space:]]*:[[:space:]]*${WANT_SUB_CALLERS}" "$f" \
      || fail "$label: missing sub_callers_events:$WANT_SUB_CALLERS\n$(cat "$f")"
    grep -qE "\"src_line_events\"[[:space:]]*:[[:space:]]*${WANT_SRC_LINE}" "$f" \
      || fail "$label: missing src_line_events:$WANT_SRC_LINE\n$(cat "$f")"
    grep -qE "\"sub_info_events\"[[:space:]]*:[[:space:]]*${WANT_SUB_INFO}" "$f" \
      || fail "$label: missing sub_info_events:$WANT_SUB_INFO\n$(cat "$f")"
    log "NOTE: no python3/JSON::PP; used greps for $label"
  fi
}

# ---------------------------------------------------------------------------
# 1. Perl query --json --jsonl default-calls1 (×2 consistency)
# ---------------------------------------------------------------------------
echo "=== query --json --jsonl default-calls1 (event counts 27/3/13/632/31) ×2 ==="
OUT1="$TMPDIR_SMOKE/perl_json_1.out"
OUT2="$TMPDIR_SMOKE/perl_json_2.out"
ERR1="$TMPDIR_SMOKE/perl_json_1.err"
ERR2="$TMPDIR_SMOKE/perl_json_2.err"

if ! "${ENGINE[@]}" query --json --jsonl "$DEFAULT_GOLDEN" >"$OUT1" 2>"$ERR1"; then
  cat "$ERR1" >&2 || true
  cat "$OUT1" >&2 || true
  fail "query --json --jsonl default-calls1 run #1 failed"
fi
if ! "${ENGINE[@]}" query --json --jsonl "$DEFAULT_GOLDEN" >"$OUT2" 2>"$ERR2"; then
  cat "$ERR2" >&2 || true
  cat "$OUT2" >&2 || true
  fail "query --json --jsonl default-calls1 run #2 failed"
fi
cat "$OUT1"
json_assert_event_counts "$OUT1" "perl default-calls1 #1"
json_assert_event_counts "$OUT2" "perl default-calls1 #2"
ok "perl query --json default-calls1 ×2: event counts 27/3/13/632/31"

# ---------------------------------------------------------------------------
# 2. Optional native report --json
# ---------------------------------------------------------------------------
echo "=== optional native report --json event counts ==="
CLI_CMD=()
if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
  CLI_CMD=("$NYTPROF_NATIVE_CLI")
elif command -v cargo >/dev/null 2>&1 && [[ -f "$ROOT/Cargo.toml" ]]; then
  CLI_CMD=(cargo run -q -p nytprof-cli --)
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  CLI_CMD=("$ROOT/prefix/bin/nytprof-cli")
elif [[ -x "$ROOT/prefix/bin/nytprof-dump" ]]; then
  CLI_CMD=("$ROOT/prefix/bin/nytprof-dump")
elif [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
  CLI_CMD=("$ROOT/target/debug/nytprof-dump")
elif [[ -x "$ROOT/target/release/nytprof-dump" ]]; then
  CLI_CMD=("$ROOT/target/release/nytprof-dump")
fi

if [[ ${#CLI_CMD[@]} -gt 0 ]]; then
  if [[ -f "$ROOT/$DEFAULT_PROFILE" ]]; then
    NOUT="$TMPDIR_SMOKE/native_default.json"
    NERR="$TMPDIR_SMOKE/native_default.err"
    if ! "${CLI_CMD[@]}" report --json "$DEFAULT_PROFILE" >"$NOUT" 2>"$NERR"; then
      cat "$NERR" >&2 || true
      cat "$NOUT" >&2 || true
      fail "native report --json default-calls1 failed"
    fi
    cat "$NOUT"
    json_assert_event_counts "$NOUT" "native default-calls1"
    ok "native report --json default-calls1: event counts 27/3/13/632/31"
  else
    log "SKIP: native default-calls1 (missing $DEFAULT_PROFILE)"
  fi
else
  log "SKIP: native report --json (no CLI found)"
fi

ok "json_event_counts_smoke (JSON-EVENT-COUNTS-MVP) passed"
exit 0
