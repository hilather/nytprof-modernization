#!/usr/bin/env bash
# JSON-FILE-BASENAME-MVP: expose dump/model-derived stable basename for fid 1
# on shipped JSON surfaces.
#
# Specs:
#   docs/schemas/perl-engine-dispatch-mvp-v0.md
#   docs/schemas/native-aggregates-json-mvp-v0.md
# Board: JSON-FILE-BASENAME-MVP
#
# Asserts on fixtures/v5/default-calls1 (real CLI only; no re-aggregation):
#   file_1_basename: string equals or contains "workload.pl"
#                    (typically exact "workload.pl")
#
# Values must match JsonlData->file_basename(1) / ProfileModel::fid_basename(1).
# Absolute file_1 under /tmp is volatile — basename is the stable contract.
# Do not invent strings; do not freeze absolute paths as identity.
#
# Surfaces:
#   1) Perl   nytprof-engine query --json --jsonl <readstream.jsonl>  (required)
#   2) native nytprof-cli report --json <profile.out>               (optional)
#   3) optional Perl query --json <profile> when native dump path works
#
# Never puts crates/ on oracle PERL5LIB. No XS.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/json_file_basename_smoke.sh
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
# JsonlData expected basename (source of truth for pure-Perl path).
# ---------------------------------------------------------------------------
EXPECT_BASENAME=""

eval "$(
  perl -I"$ENGINE_LIB" -MDevel::NYTProf::JsonlData -e '
    use strict; use warnings;
    my $d = Devel::NYTProf::JsonlData->from_jsonl($ARGV[0]);
    my $bn = $d->file_basename(1);
    die "JsonlData missing file_basename(1)\n" unless defined $bn && length $bn;
    die "file_basename(1) must contain workload.pl, got $bn\n"
      unless $bn =~ /workload\.pl/;
    # Prefer exact golden when the API returns bare basename.
    $bn =~ s/'\''/'\''\\'\'''\''/g;
    print "EXPECT_BASENAME='\''$bn'\'';\n";
  ' "$GOLDEN"
)" || fail "failed to load JsonlData file_basename(1) from $GOLDEN"

[[ -n "$EXPECT_BASENAME" ]] || fail "empty EXPECT_BASENAME"
# Contract: equals or contains workload.pl (typically exact).
[[ "$EXPECT_BASENAME" == *workload.pl* ]] \
  || fail "EXPECT_BASENAME must contain workload.pl, got $EXPECT_BASENAME"
ok "JsonlData expect: file_1_basename=$EXPECT_BASENAME"

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
# Assert file_1_basename equals/contains workload.pl and matches JsonlData.
# ---------------------------------------------------------------------------
assert_file_basename() {
  local f="$1"
  local label="$2"
  if command -v python3 >/dev/null 2>&1; then
    if ! EXPECT_BASENAME="$EXPECT_BASENAME" python3 - "$f" <<'PY'
import json, os, sys
path = sys.argv[1]
want = os.environ["EXPECT_BASENAME"]
with open(path, encoding="utf-8") as fh:
    obj = json.load(fh)
if not isinstance(obj, dict):
    sys.exit("not an object")
if obj.get("ok") is not True:
    sys.exit(f"ok must be true, got {obj.get('ok')!r}")
bn = obj.get("file_1_basename")
if not isinstance(bn, str) or not bn:
    sys.exit(f"file_1_basename must be non-empty string, got {bn!r}")
if "workload.pl" not in bn:
    sys.exit(f"file_1_basename must contain workload.pl, got {bn!r}")
if bn != want and want not in bn and bn not in want:
    # Prefer exact match to JsonlData; also accept equal-or-contains contract.
    sys.exit(f"file_1_basename={bn!r} want={want!r}")
# Strict: default-calls1 should match JsonlData exactly.
if bn != want:
    sys.exit(f"file_1_basename={bn!r} JsonlData={want!r}")
print(bn)
PY
    then
      fail "$label: file_1_basename JSON field failed
$(cat "$f")"
    fi
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      use strict; use warnings;
      my ($path, $want) = @ARGV;
      open my $fh, "<", $path or die $!;
      local $/; my $raw = <$fh>;
      my $obj = JSON::PP->new->decode($raw);
      die "not object\n" unless ref($obj) eq "HASH";
      die "ok must be true\n" unless $obj->{ok};
      my $bn = $obj->{file_1_basename};
      die "file_1_basename missing\n" unless defined $bn && length $bn;
      die "file_1_basename no workload.pl\n" unless $bn =~ /workload\.pl/;
      die "file_1_basename mismatch: $bn vs $want\n" unless $bn eq $want;
      print "$bn\n";
    ' "$f" "$EXPECT_BASENAME" \
      || fail "$label: invalid JSON or file_1_basename (perl JSON::PP)
$(cat "$f")"
  else
    grep -qE '"file_1_basename"[[:space:]]*:[[:space:]]*"[^"]*workload\.pl[^"]*"' "$f" \
      || fail "$label: missing file_1_basename containing workload.pl
$(cat "$f")"
    log "NOTE: no python3/perl JSON::PP path fully exercised; used key greps for $label"
  fi
  ok "$label: file_1_basename=$EXPECT_BASENAME (contains workload.pl)"
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
assert_file_basename "$JOUT1" "perl query --json --jsonl #1"
assert_file_basename "$JOUT2" "perl query --json --jsonl #2"

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
  assert_file_basename "$NOUT" "native report --json"

  # Cross-check basename equal between sides.
  if command -v python3 >/dev/null 2>&1; then
    python3 - "$JOUT1" "$NOUT" <<'PY' || fail "native↔perl file_1_basename cross-check failed"
import json, sys
p = json.load(open(sys.argv[1], encoding="utf-8"))
n = json.load(open(sys.argv[2], encoding="utf-8"))
if p.get("file_1_basename") != n.get("file_1_basename"):
    sys.exit(
        f"file_1_basename: perl={p.get('file_1_basename')!r} "
        f"native={n.get('file_1_basename')!r}"
    )
print("cross-ok")
PY
    ok "native↔perl file_1_basename equal"
  fi

  # Optional: Perl query --json via native dump of live profile
  DOUT="$TMPDIR_SMOKE/perl_profile.out"
  DERR="$TMPDIR_SMOKE/perl_profile.err"
  if "${ENGINE[@]}" query --json "$PROFILE" >"$DOUT" 2>"$DERR"; then
    assert_file_basename "$DOUT" "perl query --json <profile>"
  else
    log "NOTE: query --json <profile> skipped/failed (native dump path optional)
$(cat "$DERR" 2>/dev/null || true)"
  fi
else
  log "SKIP native report --json (no native CLI)"
fi

ok "json_file_basename_smoke completed successfully (default-calls1 file_1_basename=$EXPECT_BASENAME)"
exit 0
