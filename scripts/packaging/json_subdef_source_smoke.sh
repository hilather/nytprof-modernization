#!/usr/bin/env bash
# JSON-SUBDEF-SOURCE-MVP: expose dump-derived A9 sub_def samples + A8 source line
# on shipped JSON surfaces.
#
# Specs:
#   docs/schemas/perl-engine-dispatch-mvp-v0.md
#   docs/schemas/native-aggregates-json-mvp-v0.md
# Board: JSON-SUBDEF-SOURCE-MVP
#
# Asserts on fixtures/v5/default-calls1 (real CLI only; no re-aggregation):
#   sub_def_leaf: fid=1 first_line=3 last_line=7
#   sub_def_mid:  fid=1 first_line=8 last_line=12
#   source_line_1_5: text contains "$x++" and "1 .. 50"
#     (golden exact often "    $x++ for 1 .. 50;\n")
#
# Surfaces:
#   1) Perl   nytprof-engine query --json --jsonl <readstream.jsonl>  (required)
#   2) native nytprof-cli report --json <profile.out>               (optional)
#   3) optional Perl query --json <profile> when native dump path works
#
# Never puts crates/ on oracle PERL5LIB. No XS.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/json_subdef_source_smoke.sh
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
# Assert sub_def_leaf / sub_def_mid / source_line_1_5
# ---------------------------------------------------------------------------
assert_subdef_source() {
  local f="$1"
  local label="$2"
  if command -v python3 >/dev/null 2>&1; then
    if ! python3 - "$f" <<'PY'
import json, sys
path = sys.argv[1]
with open(path, encoding="utf-8") as fh:
    obj = json.load(fh)
if not isinstance(obj, dict):
    sys.exit("not an object")
if obj.get("ok") is not True:
    sys.exit(f"ok must be true, got {obj.get('ok')!r}")

def check_def(key, fid, first, last):
    d = obj.get(key)
    if not isinstance(d, dict):
        sys.exit(f"{key} must be object, got {type(d).__name__}: {d!r}")
    for k, want in (("fid", fid), ("first_line", first), ("last_line", last)):
        got = d.get(k)
        if got != want:
            sys.exit(f"{key}.{k} must be {want}, got {got!r}")

check_def("sub_def_leaf", 1, 3, 7)
check_def("sub_def_mid", 1, 8, 12)
src = obj.get("source_line_1_5")
if not isinstance(src, str):
    sys.exit(f"source_line_1_5 must be string, got {type(src).__name__}: {src!r}")
if "$x++" not in src:
    sys.exit(f"source_line_1_5 must contain $x++, got {src!r}")
if "1 .. 50" not in src:
    sys.exit(f"source_line_1_5 must contain 1 .. 50, got {src!r}")
print(src.replace("\n", "\\n"))
PY
    then
      fail "$label: sub_def/source JSON fields failed
$(cat "$f")"
    fi
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      use strict; use warnings;
      open my $fh, "<", $ARGV[0] or die $!;
      local $/; my $raw = <$fh>;
      my $obj = JSON::PP->new->decode($raw);
      die "not object\n" unless ref($obj) eq "HASH";
      die "ok must be true\n" unless $obj->{ok};
      for my $pair (
        [ "sub_def_leaf", 1, 3, 7 ],
        [ "sub_def_mid",  1, 8, 12 ],
      ) {
        my ( $key, $fid, $first, $last ) = @$pair;
        my $d = $obj->{$key};
        die "$key not object\n" unless ref($d) eq "HASH";
        die "$key.fid\n"        unless ($d->{fid} // -1) == $fid;
        die "$key.first_line\n" unless ($d->{first_line} // -1) == $first;
        die "$key.last_line\n"  unless ($d->{last_line}  // -1) == $last;
      }
      my $src = $obj->{source_line_1_5};
      die "source_line_1_5 missing\n" unless defined $src;
      die "source_line_1_5 no \$x++\n" unless $src =~ /\$x\+\+/;
      die "source_line_1_5 no 1 .. 50\n" unless $src =~ /1 \.\. 50/;
      (my $one = $src) =~ s/\n/\\n/g;
      print $one, "\n";
    ' "$f" || fail "$label: invalid JSON or sub_def/source fields (perl JSON::PP)
$(cat "$f")"
  else
    # Last-resort greps for compact JSON.
    grep -qE '"sub_def_leaf"[[:space:]]*:[[:space:]]*\{[^}]*"fid"[[:space:]]*:[[:space:]]*1' "$f" \
      || fail "$label: missing sub_def_leaf fid=1\n$(cat "$f")"
    grep -qE '"first_line"[[:space:]]*:[[:space:]]*3' "$f" \
      || fail "$label: missing first_line:3\n$(cat "$f")"
    grep -qE '"last_line"[[:space:]]*:[[:space:]]*7' "$f" \
      || fail "$label: missing last_line:7\n$(cat "$f")"
    grep -qE '"sub_def_mid"[[:space:]]*:[[:space:]]*\{[^}]*"fid"[[:space:]]*:[[:space:]]*1' "$f" \
      || fail "$label: missing sub_def_mid fid=1\n$(cat "$f")"
    grep -qE '"first_line"[[:space:]]*:[[:space:]]*8' "$f" \
      || fail "$label: missing first_line:8\n$(cat "$f")"
    grep -qE '"last_line"[[:space:]]*:[[:space:]]*12' "$f" \
      || fail "$label: missing last_line:12\n$(cat "$f")"
    grep -qE '\$x\+\+' "$f" \
      || fail "$label: missing \$x++ in JSON\n$(cat "$f")"
    grep -qE '1 \.\. 50' "$f" \
      || fail "$label: missing 1 .. 50 in JSON\n$(cat "$f")"
    log "NOTE: no python3/perl JSON::PP path fully exercised; used key greps for $label"
  fi
  ok "$label: sub_def_leaf 1/3–7, sub_def_mid 1/8–12, source has \$x++ and 1 .. 50"
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
assert_subdef_source "$JOUT1" "perl query --json --jsonl #1"
assert_subdef_source "$JOUT2" "perl query --json --jsonl #2"

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
  assert_subdef_source "$NOUT" "native report --json"

  # Cross-check leaf/mid ranges equal between sides (parse via python/perl when available).
  if command -v python3 >/dev/null 2>&1; then
    python3 - "$JOUT1" "$NOUT" <<'PY' || fail "native↔perl sub_def cross-check failed"
import json, sys
p = json.load(open(sys.argv[1], encoding="utf-8"))
n = json.load(open(sys.argv[2], encoding="utf-8"))
for k in ("sub_def_leaf", "sub_def_mid"):
    if p.get(k) != n.get(k):
        sys.exit(f"{k}: perl={p.get(k)!r} native={n.get(k)!r}")
ps = p.get("source_line_1_5") or ""
ns = n.get("source_line_1_5") or ""
if ("$x++" not in ns) or ("1 .. 50" not in ns):
    sys.exit(f"native source_line_1_5 missing markers: {ns!r}")
if ps != ns:
    # Accept equal content; fail only if both present but differ.
    sys.exit(f"source_line_1_5 differ: perl={ps!r} native={ns!r}")
print("cross-ok")
PY
    ok "native↔perl sub_def_leaf/mid + source_line_1_5 equal"
  fi

  # Optional: Perl query --json via native dump of live profile
  DOUT="$TMPDIR_SMOKE/perl_profile.out"
  DERR="$TMPDIR_SMOKE/perl_profile.err"
  if "${ENGINE[@]}" query --json "$PROFILE" >"$DOUT" 2>"$DERR"; then
    assert_subdef_source "$DOUT" "perl query --json <profile>"
  else
    log "NOTE: query --json <profile> skipped/failed (native dump path optional)
$(cat "$DERR" 2>/dev/null || true)"
  fi
else
  log "SKIP native report --json (no native CLI)"
fi

ok "json_subdef_source_smoke completed successfully (default-calls1 leaf 1/3–7 mid 1/8–12 source hot-loop)"
exit 0
