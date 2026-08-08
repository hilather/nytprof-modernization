#!/usr/bin/env bash
# JSON-TOTAL-EVENTS-MVP + JSON-ATTR-BASETIME-MVP: expose dump/model-derived
# total_events and greppable attribute_basetime on shipped JSON surfaces.
#
# Specs:
#   docs/schemas/perl-engine-dispatch-mvp-v0.md
#   docs/schemas/native-aggregates-json-mvp-v0.md
# Board: JSON-TOTAL-EVENTS-MVP / JSON-ATTR-BASETIME-MVP
#
# Contract (real CLIs only; no re-aggregation) on fixtures/v5/default-calls1:
#   total_events       == 2474
#     (golden readstream.jsonl / nytprof-cli dump line count including synthetic
#      _END / JsonlData.records_seen; native JSON uses model.total_events+1;
#      ProfileModel.total_events alone is 2473 decoded binary tags)
#   attribute_basetime == dump/model ATTRIBUTE basetime (string;
#     golden often "1786111723"; greppable sample, not wall-clock freeze)
#
# Surfaces:
#   1) Perl   nytprof-engine query --json --jsonl <readstream.jsonl>  (required)
#   2) native nytprof-cli report --json <profile.out>               (optional)
#   3) optional golden line-count + ATTRIBUTE basetime recount
#
# Never puts crates/ on oracle PERL5LIB. No XS.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/json_total_basetime_smoke.sh
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

# default-calls1 contract
WANT_TOTAL_EVENTS=2474

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
EXPECT_TOTAL=""
EXPECT_BASETIME=""

eval "$(
  perl -I"$ENGINE_LIB" -MDevel::NYTProf::JsonlData -e '
    use strict; use warnings;
    my $d = Devel::NYTProf::JsonlData->from_jsonl($ARGV[0]);
    my $want_total = 0 + $ARGV[1];
    my $total = 0 + ($d->records_seen // 0);
    die "JsonlData records_seen=$total want $want_total\n" unless $total == $want_total;
    my $base = $d->attribute("basetime");
    die "JsonlData missing attribute(basetime)\n"
      unless defined $base && length $base;
    $base =~ s/'\''/'\''\\'\'''\''/g;
    print "EXPECT_TOTAL=$total\n";
    print "EXPECT_BASETIME='\''$base'\'';\n";
  ' "$DEFAULT_GOLDEN" "$WANT_TOTAL_EVENTS"
)" || fail "failed to load JsonlData total/basetime from golden"

[[ -n "$EXPECT_TOTAL" ]] || fail "empty EXPECT_TOTAL"
[[ -n "$EXPECT_BASETIME" ]] || fail "empty EXPECT_BASETIME"
[[ "$EXPECT_TOTAL" == "$WANT_TOTAL_EVENTS" ]] \
  || fail "EXPECT_TOTAL=$EXPECT_TOTAL want $WANT_TOTAL_EVENTS"
ok "JsonlData expect: total_events=$EXPECT_TOTAL attribute_basetime=$EXPECT_BASETIME"

# ---------------------------------------------------------------------------
# Optional independent golden line count + ATTRIBUTE basetime recount
# ---------------------------------------------------------------------------
if command -v python3 >/dev/null 2>&1; then
  python3 -c '
import json, sys
path = sys.argv[1]
want_total = int(sys.argv[2])
want_base = sys.argv[3]
n = 0
basetime = None
with open(path, encoding="utf-8") as fh:
    for line in fh:
        line = line.strip()
        if not line:
            continue
        n += 1
        o = json.loads(line)
        if o.get("tag") == "ATTRIBUTE":
            args = o.get("args") or []
            if len(args) >= 2 and args[0] == "basetime":
                basetime = str(args[1])
if n != want_total:
    raise SystemExit("%s: line count %s want %s" % (path, n, want_total))
if basetime is None:
    raise SystemExit("%s: missing ATTRIBUTE basetime" % path)
if basetime != want_base:
    raise SystemExit("%s: basetime %r want %r" % (path, basetime, want_base))
print("golden_recount_ok lines=%s basetime=%s" % (n, basetime))
' "$DEFAULT_GOLDEN" "$WANT_TOTAL_EVENTS" "$EXPECT_BASETIME" \
    || fail "golden line/basetime recount mismatch"
  ok "golden recount: lines=$WANT_TOTAL_EVENTS basetime=$EXPECT_BASETIME"
else
  # Fallback: wc -l + grep for basetime presence
  lines="$(wc -l <"$DEFAULT_GOLDEN" | tr -d ' ')"
  [[ "$lines" == "$WANT_TOTAL_EVENTS" ]] \
    || fail "golden wc -l=$lines want $WANT_TOTAL_EVENTS"
  grep -q "basetime" "$DEFAULT_GOLDEN" \
    || fail "golden missing basetime token"
  log "NOTE: no python3 — used wc -l + grep for golden recount"
fi

# ---------------------------------------------------------------------------
# Assert helpers
# ---------------------------------------------------------------------------
json_assert_total_basetime() {
  local f="$1"
  local label="$2"
  if command -v python3 >/dev/null 2>&1; then
    EXPECT_TOTAL="$EXPECT_TOTAL" EXPECT_BASETIME="$EXPECT_BASETIME" python3 -c '
import json, os, sys
path, label = sys.argv[1], sys.argv[2]
want_total = int(os.environ["EXPECT_TOTAL"])
want_base = os.environ["EXPECT_BASETIME"]
o = json.load(open(path, encoding="utf-8"))
if o.get("ok") is not True:
    raise SystemExit("%s: ok must be true, got %r" % (label, o.get("ok")))
got_total = o.get("total_events")
if got_total != want_total:
    raise SystemExit("%s: total_events must be %s, got %r" % (label, want_total, got_total))
got_base = o.get("attribute_basetime")
if not isinstance(got_base, str) or not got_base:
    raise SystemExit("%s: attribute_basetime must be non-empty string, got %r" % (label, got_base))
if got_base != want_base:
    raise SystemExit("%s: attribute_basetime must be %r, got %r" % (label, want_base, got_base))
print("total_basetime_ok", label, "total_events=%s attribute_basetime=%s" % (got_total, got_base))
' "$f" "$label" \
      || fail "$label: total_events/attribute_basetime assert failed
$(cat "$f")"
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      use strict; use warnings;
      my ($f, $label, $want_total, $want_base) = @ARGV;
      open my $fh, "<", $f or die "$label: $!";
      local $/; my $obj = JSON::PP->new->decode(<$fh>);
      die "$label: ok\n" unless $obj->{ok};
      my $t = $obj->{total_events};
      die "$label: total_events missing\n" unless defined $t;
      die "$label: total_events want $want_total got $t\n" unless 0+$t == 0+$want_total;
      my $b = $obj->{attribute_basetime};
      die "$label: attribute_basetime missing\n" unless defined $b && length $b;
      die "$label: attribute_basetime want $want_base got $b\n" unless $b eq $want_base;
      printf "total_basetime_ok %s total_events=%s attribute_basetime=%s\n",
        $label, $t, $b;
    ' "$f" "$label" "$EXPECT_TOTAL" "$EXPECT_BASETIME" \
      || fail "$label: total_events/attribute_basetime assert failed (perl)
$(cat "$f")"
  else
    grep -qE "\"total_events\"[[:space:]]*:[[:space:]]*${EXPECT_TOTAL}" "$f" \
      || fail "$label: missing total_events:$EXPECT_TOTAL
$(cat "$f")"
    grep -qE "\"attribute_basetime\"[[:space:]]*:[[:space:]]*\"${EXPECT_BASETIME}\"" "$f" \
      || fail "$label: missing attribute_basetime:$EXPECT_BASETIME
$(cat "$f")"
    log "NOTE: no python3/JSON::PP; used greps for $label"
  fi
}

# ---------------------------------------------------------------------------
# 1. Perl query --json --jsonl default-calls1 (×2 consistency)
# ---------------------------------------------------------------------------
echo "=== query --json --jsonl default-calls1 (total_events + basetime) ×2 ==="
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
json_assert_total_basetime "$OUT1" "perl default-calls1 #1"
json_assert_total_basetime "$OUT2" "perl default-calls1 #2"
ok "perl query --json default-calls1 ×2: total_events=$EXPECT_TOTAL basetime=$EXPECT_BASETIME"

# ---------------------------------------------------------------------------
# 2. Optional native report --json
# ---------------------------------------------------------------------------
echo "=== optional native report --json total_events + basetime ==="
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
    json_assert_total_basetime "$NOUT" "native default-calls1"
    ok "native report --json default-calls1: total_events=$EXPECT_TOTAL basetime=$EXPECT_BASETIME"
  else
    log "SKIP: native default-calls1 (missing $DEFAULT_PROFILE)"
  fi
else
  log "SKIP: native report --json (no CLI found)"
fi

ok "json_total_basetime_smoke (JSON-TOTAL-EVENTS-MVP / JSON-ATTR-BASETIME-MVP) passed"
exit 0
