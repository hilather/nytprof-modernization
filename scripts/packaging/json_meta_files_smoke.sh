#!/usr/bin/env bash
# JSON-META-FILES-MVP: expose dump-derived ATTRIBUTE / OPTION / NEW_FID samples
# on shipped JSON surfaces.
#
# Specs:
#   docs/schemas/perl-engine-dispatch-mvp-v0.md
#   docs/schemas/native-aggregates-json-mvp-v0.md
# Board: JSON-META-FILES-MVP
#
# Asserts on fixtures/v5/default-calls1 (real CLI only; no re-aggregation):
#   attribute_ticks_per_sec: defined; golden often "10000000"
#   option_calls:            defined; golden often "1"
#   file_1:                  path string contains "workload.pl"
#
# Values must match JsonlData/model (or independent golden recount of
# ATTRIBUTE/OPTION/NEW_FID lines). Do not invent strings.
#
# Surfaces:
#   1) Perl   nytprof-engine query --json --jsonl <readstream.jsonl>  (required)
#   2) native nytprof-cli report --json <profile.out>               (optional)
#   3) optional Perl query --json <profile> when native dump path works
#
# Never puts crates/ on oracle PERL5LIB. No XS.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/json_meta_files_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE_DIR="fixtures/v5/default-calls1"
GOLDEN="$FIXTURE_DIR/readstream.jsonl"
PROFILE="$FIXTURE_DIR/nytprof.out"
ENGINE_BIN="perl/bin/nytprof-engine"
ENGINE_LIB="perl/lib"
DISPATCH_PM="$ENGINE_LIB/Devel/NYTProf/EngineDispatch.pm"
DATA_PM="$ENGINE_LIB/Devel/NYTProf/JsonlData.pm"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$GOLDEN" ]] || fail "missing golden dump $GOLDEN"
[[ -f "$ROOT/$PROFILE" ]] || fail "missing profile $PROFILE"
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
# Independent golden / JsonlData expected values (ATTRIBUTE/OPTION/NEW_FID).
# ---------------------------------------------------------------------------
# Prefer JsonlData APIs (source of truth for pure-Perl path); also cross-check
# against a light scan of golden JSONL tags when python3 is available.
EXPECT_TICKS=""
EXPECT_CALLS=""
EXPECT_FILE1=""

eval "$(
  perl -I"$ENGINE_LIB" -MDevel::NYTProf::JsonlData -e '
    use strict; use warnings;
    my $d = Devel::NYTProf::JsonlData->from_jsonl($ARGV[0]);
    my $ticks = $d->attribute("ticks_per_sec");
    my $calls = $d->option("calls");
    my $file1 = $d->file(1);
    die "JsonlData missing attribute ticks_per_sec\n" unless defined $ticks && length $ticks;
    die "JsonlData missing option calls\n" unless defined $calls && length $calls;
    die "JsonlData missing file(1)\n" unless defined $file1 && length $file1;
    die "file(1) must contain workload.pl, got $file1\n" unless $file1 =~ /workload\.pl/;
    # Shell-safe single-quoted export (values are dump strings, no newlines expected).
    for my $pair (
      [ "EXPECT_TICKS", $ticks ],
      [ "EXPECT_CALLS", $calls ],
      [ "EXPECT_FILE1", $file1 ],
    ) {
      my ( $name, $val ) = @$pair;
      $val =~ s/'\''/'\''\\'\'''\''/g;
      print "$name='\''$val'\'';\n";
    }
  ' "$GOLDEN"
)" || fail "failed to load JsonlData expected meta from $GOLDEN"

[[ -n "$EXPECT_TICKS" ]] || fail "empty EXPECT_TICKS"
[[ -n "$EXPECT_CALLS" ]] || fail "empty EXPECT_CALLS"
[[ -n "$EXPECT_FILE1" ]] || fail "empty EXPECT_FILE1"
# Golden string expectations for this fixture (also greppable).
[[ "$EXPECT_TICKS" == "10000000" ]] || fail "default-calls1 ticks_per_sec golden expected 10000000, got $EXPECT_TICKS"
[[ "$EXPECT_CALLS" == "1" ]] || fail "default-calls1 option calls golden expected 1, got $EXPECT_CALLS"
ok "JsonlData expect: attribute_ticks_per_sec=$EXPECT_TICKS option_calls=$EXPECT_CALLS file_1=…/workload.pl"

# Optional independent recount from golden JSONL ATTRIBUTE/OPTION/NEW_FID lines.
if command -v python3 >/dev/null 2>&1; then
  python3 - "$GOLDEN" "$EXPECT_TICKS" "$EXPECT_CALLS" "$EXPECT_FILE1" <<'PY' \
    || fail "golden JSONL ATTRIBUTE/OPTION/NEW_FID recount mismatch"
import json, sys
path, want_ticks, want_calls, want_file1 = sys.argv[1:5]
ticks = calls = file1 = None
with open(path, encoding="utf-8") as fh:
    for line in fh:
        line = line.strip()
        if not line:
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError:
            continue
        tag = obj.get("tag") or obj.get("type") or ""
        args = obj.get("args")
        # Support both {tag,args} event shape and flat records if present.
        if tag == "ATTRIBUTE" and isinstance(args, list) and len(args) >= 2:
            if args[0] == "ticks_per_sec":
                ticks = str(args[1])
        elif tag == "OPTION" and isinstance(args, list) and len(args) >= 2:
            if args[0] == "calls":
                calls = str(args[1])
        elif tag == "NEW_FID" and isinstance(args, list) and len(args) >= 2:
            # NEW_FID: fid, eval_fid, eval_line, flags, size, mtime, name
            # Path is last arg (schema name); same as JsonlData / ProfileModel.
            try:
                fid = int(args[0])
            except (TypeError, ValueError):
                continue
            if fid == 1:
                file1 = str(args[-1])
if ticks is None:
    # Some golden lines may use name/value fields; fall back without hard-fail
    # if recount cannot find tags (JsonlData already verified).
    print("NOTE: golden scan did not find ATTRIBUTE ticks_per_sec; trusting JsonlData")
    sys.exit(0)
if ticks != want_ticks:
    sys.exit(f"ticks_per_sec recount={ticks!r} want={want_ticks!r}")
if calls is not None and calls != want_calls:
    sys.exit(f"option calls recount={calls!r} want={want_calls!r}")
if file1 is not None:
    if "workload.pl" not in file1:
        sys.exit(f"file_1 recount missing workload.pl: {file1!r}")
    if file1 != want_file1:
        sys.exit(f"file_1 recount={file1!r} want={want_file1!r}")
print("golden-recount-ok")
PY
  ok "golden JSONL ATTRIBUTE/OPTION/NEW_FID recount consistent with JsonlData"
fi

# ---------------------------------------------------------------------------
# Resolve native CLI (prefer cargo when present so smoke exercises current tree).
# ---------------------------------------------------------------------------
CLI_MODE="" # binary | cargo | none
CLI_BIN=""
CLI_CMD=()

if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
  CLI_BIN="$NYTPROF_NATIVE_CLI"
  CLI_MODE=binary
elif command -v cargo >/dev/null 2>&1 && [[ -f "$ROOT/Cargo.toml" ]]; then
  CLI_MODE=cargo
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  CLI_BIN="$ROOT/prefix/bin/nytprof-cli"
  CLI_MODE=binary
elif [[ -x "$ROOT/prefix/bin/nytprof-dump" ]]; then
  CLI_BIN="$ROOT/prefix/bin/nytprof-dump"
  CLI_MODE=binary
elif [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
  CLI_BIN="$ROOT/target/debug/nytprof-dump"
  CLI_MODE=binary
elif [[ -x "$ROOT/target/release/nytprof-dump" ]]; then
  CLI_BIN="$ROOT/target/release/nytprof-dump"
  CLI_MODE=binary
else
  CLI_MODE=none
fi

if [[ "$CLI_MODE" == "binary" ]]; then
  CLI_CMD=("$CLI_BIN")
  ok "using native binary: $CLI_BIN"
elif [[ "$CLI_MODE" == "cargo" ]]; then
  CLI_CMD=(cargo run -q -p nytprof-cli --)
  ok "using cargo run -p nytprof-cli"
else
  log "NOTE: no native CLI — native report --json path will be skipped"
  log "  (pure-Perl query --jsonl still required)"
fi

# ---------------------------------------------------------------------------
# Assert attribute_ticks_per_sec / option_calls / file_1
# ---------------------------------------------------------------------------
assert_meta_files() {
  local f="$1"
  local label="$2"
  if command -v python3 >/dev/null 2>&1; then
    if ! EXPECT_TICKS="$EXPECT_TICKS" EXPECT_CALLS="$EXPECT_CALLS" EXPECT_FILE1="$EXPECT_FILE1" \
      python3 - "$f" <<'PY'
import json, os, sys
path = sys.argv[1]
want_ticks = os.environ["EXPECT_TICKS"]
want_calls = os.environ["EXPECT_CALLS"]
want_file1 = os.environ["EXPECT_FILE1"]
with open(path, encoding="utf-8") as fh:
    obj = json.load(fh)
if not isinstance(obj, dict):
    sys.exit("not an object")
if obj.get("ok") is not True:
    sys.exit(f"ok must be true, got {obj.get('ok')!r}")
ticks = obj.get("attribute_ticks_per_sec")
calls = obj.get("option_calls")
file1 = obj.get("file_1")
if not isinstance(ticks, str) or not ticks:
    sys.exit(f"attribute_ticks_per_sec must be non-empty string, got {ticks!r}")
if ticks != want_ticks:
    sys.exit(f"attribute_ticks_per_sec={ticks!r} want={want_ticks!r}")
if not isinstance(calls, str) or not calls:
    sys.exit(f"option_calls must be non-empty string, got {calls!r}")
if calls != want_calls:
    sys.exit(f"option_calls={calls!r} want={want_calls!r}")
if not isinstance(file1, str) or not file1:
    sys.exit(f"file_1 must be non-empty string, got {file1!r}")
if "workload.pl" not in file1:
    sys.exit(f"file_1 must contain workload.pl, got {file1!r}")
if file1 != want_file1:
    sys.exit(f"file_1={file1!r} want={want_file1!r}")
print(f"{ticks}|{calls}|{file1}")
PY
    then
      fail "$label: meta/files JSON fields failed
$(cat "$f")"
    fi
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      use strict; use warnings;
      my ($path, $want_ticks, $want_calls, $want_file1) = @ARGV;
      open my $fh, "<", $path or die $!;
      local $/; my $raw = <$fh>;
      my $obj = JSON::PP->new->decode($raw);
      die "not object\n" unless ref($obj) eq "HASH";
      die "ok must be true\n" unless $obj->{ok};
      my $ticks = $obj->{attribute_ticks_per_sec};
      my $calls = $obj->{option_calls};
      my $file1 = $obj->{file_1};
      die "attribute_ticks_per_sec missing\n" unless defined $ticks && length $ticks;
      die "attribute_ticks_per_sec mismatch\n" unless $ticks eq $want_ticks;
      die "option_calls missing\n" unless defined $calls && length $calls;
      die "option_calls mismatch\n" unless $calls eq $want_calls;
      die "file_1 missing\n" unless defined $file1 && length $file1;
      die "file_1 no workload.pl\n" unless $file1 =~ /workload\.pl/;
      die "file_1 mismatch\n" unless $file1 eq $want_file1;
      print "$ticks|$calls|$file1\n";
    ' "$f" "$EXPECT_TICKS" "$EXPECT_CALLS" "$EXPECT_FILE1" \
      || fail "$label: invalid JSON or meta/files fields (perl JSON::PP)
$(cat "$f")"
  else
    # Last-resort greps for compact JSON.
    grep -qE "\"attribute_ticks_per_sec\"[[:space:]]*:[[:space:]]*\"${EXPECT_TICKS}\"" "$f" \
      || fail "$label: missing attribute_ticks_per_sec=$EXPECT_TICKS
$(cat "$f")"
    grep -qE "\"option_calls\"[[:space:]]*:[[:space:]]*\"${EXPECT_CALLS}\"" "$f" \
      || fail "$label: missing option_calls=$EXPECT_CALLS
$(cat "$f")"
    grep -qE 'workload\.pl' "$f" \
      || fail "$label: missing workload.pl in JSON
$(cat "$f")"
    grep -qE '"file_1"[[:space:]]*:' "$f" \
      || fail "$label: missing file_1 key
$(cat "$f")"
    log "NOTE: no python3/perl JSON::PP path fully exercised; used key greps for $label"
  fi
  ok "$label: attribute_ticks_per_sec=$EXPECT_TICKS option_calls=$EXPECT_CALLS file_1 contains workload.pl"
}

# ---------------------------------------------------------------------------
# 1. Perl query --json --jsonl (required) ×2
# ---------------------------------------------------------------------------
echo "=== default-calls1: query --json --jsonl ×2 ==="
JOUT1="$TMPDIR_SMOKE/perl_jsonl_1.out"
JOUT2="$TMPDIR_SMOKE/perl_jsonl_2.out"
JERR1="$TMPDIR_SMOKE/perl_jsonl_1.err"
JERR2="$TMPDIR_SMOKE/perl_jsonl_2.err"

if ! "${ENGINE[@]}" query --json --jsonl "$GOLDEN" >"$JOUT1" 2>"$JERR1"; then
  cat "$JERR1" >&2 || true
  cat "$JOUT1" >&2 || true
  fail "query --json --jsonl run #1 failed"
fi
if ! "${ENGINE[@]}" query --json --jsonl "$GOLDEN" >"$JOUT2" 2>"$JERR2"; then
  cat "$JERR2" >&2 || true
  cat "$JOUT2" >&2 || true
  fail "query --json --jsonl run #2 failed"
fi
cat "$JOUT1"
assert_meta_files "$JOUT1" "perl query --json --jsonl #1"
assert_meta_files "$JOUT2" "perl query --json --jsonl #2"

if ! cmp -s "$JOUT1" "$JOUT2"; then
  fail "query --json --jsonl not byte-identical across two runs
--- run1 ---
$(cat "$JOUT1")
--- run2 ---
$(cat "$JOUT2")"
fi
ok "perl query --json --jsonl consistent across two runs"

# ---------------------------------------------------------------------------
# 2. Optional native report --json
# ---------------------------------------------------------------------------
if [[ "$CLI_MODE" != "none" ]]; then
  echo "=== default-calls1: native report --json ==="
  NOUT="$TMPDIR_SMOKE/native.out"
  NERR="$TMPDIR_SMOKE/native.err"
  if ! "${CLI_CMD[@]}" report --json "$PROFILE" >"$NOUT" 2>"$NERR"; then
    cat "$NERR" >&2 || true
    cat "$NOUT" >&2 || true
    fail "native report --json failed"
  fi
  cat "$NOUT"
  assert_meta_files "$NOUT" "native report --json"

  # Cross-check meta samples equal between sides.
  if command -v python3 >/dev/null 2>&1; then
    python3 - "$JOUT1" "$NOUT" <<'PY' || fail "native↔perl meta cross-check failed"
import json, sys
p = json.load(open(sys.argv[1], encoding="utf-8"))
n = json.load(open(sys.argv[2], encoding="utf-8"))
for k in ("attribute_ticks_per_sec", "option_calls", "file_1"):
    if p.get(k) != n.get(k):
        sys.exit(f"{k}: perl={p.get(k)!r} native={n.get(k)!r}")
print("cross-ok")
PY
    ok "native↔perl attribute_ticks_per_sec / option_calls / file_1 equal"
  fi

  # Optional: Perl query --json via native dump of live profile
  DOUT="$TMPDIR_SMOKE/perl_profile.out"
  DERR="$TMPDIR_SMOKE/perl_profile.err"
  if "${ENGINE[@]}" query --json "$PROFILE" >"$DOUT" 2>"$DERR"; then
    assert_meta_files "$DOUT" "perl query --json <profile>"
  else
    log "NOTE: query --json <profile> skipped/failed (native dump path optional)
$(cat "$DERR" 2>/dev/null || true)"
  fi
else
  log "SKIP native report --json (no native CLI)"
fi

ok "json_meta_files_smoke completed successfully (default-calls1 ticks=$EXPECT_TICKS calls=$EXPECT_CALLS file_1 has workload.pl)"
exit 0
