#!/usr/bin/env bash
# JSON-SUB-ENTRY-MVP: expose SUB_ENTRY multiplicity on both JSON surfaces.
#
# Specs:
#   docs/schemas/native-aggregates-json-mvp-v0.md
#   docs/schemas/perl-engine-dispatch-mvp-v0.md
#   docs/schemas/perl-jsonl-data-mvp-v0.md
# Board: JSON-SUB-ENTRY-MVP
#
# Asserts sub_entry_events on real shipped CLIs (no re-aggregation):
#   - fixtures/v5/default-calls1 → 0  (calls=1; no SUB_ENTRY tags)
#   - fixtures/v5/calls2-default → 27 (matches stream recount of SUB_ENTRY)
#
# Surfaces:
#   1) native  nytprof-cli report --json  <profile.out>
#   2) Perl    nytprof-engine query --json --jsonl <readstream.jsonl>
#   3) optional Perl query --json <profile> when native dump path works
#
# Never puts crates/ on oracle PERL5LIB. No XS.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/json_sub_entry_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

ENGINE_BIN="perl/bin/nytprof-engine"
ENGINE_LIB="perl/lib"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$ENGINE_BIN" ]] || fail "missing $ENGINE_BIN"
[[ -d "$ROOT/$ENGINE_LIB" ]] || fail "missing $ENGINE_LIB"
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
# JSON helpers: extract / assert sub_entry_events
# ---------------------------------------------------------------------------
json_field_int() {
  # Usage: json_field_int FILE FIELD → prints integer or fails
  local f="$1"
  local field="$2"
  if command -v python3 >/dev/null 2>&1; then
    python3 -c '
import json,sys
o=json.load(open(sys.argv[1],encoding="utf-8"))
v=o.get(sys.argv[2])
if not isinstance(v, int) or isinstance(v, bool):
    raise SystemExit("missing or non-int %s: %r" % (sys.argv[2], v))
print(v)
' "$f" "$field"
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      my ($f,$field)=@ARGV;
      open my $fh, "<", $f or die $!;
      local $/; my $o = JSON::PP->new->decode(<$fh>);
      my $v = $o->{$field};
      die "missing or non-int $field\n" unless defined $v && $v =~ /^-?\d+$/;
      print 0+$v, "\n";
    ' "$f" "$field"
  else
    # Grep last resort for compact JSON `"field":N`
    local line
    line="$(grep -oE "\"${field}\"[[:space:]]*:[[:space:]]*-?[0-9]+" "$f" | head -1 || true)"
    [[ -n "$line" ]] || fail "cannot parse $field from $f (no python3/perl)"
    echo "$line" | sed -E 's/.*:[[:space:]]*//'
  fi
}

assert_sub_entry() {
  local f="$1"
  local expect="$2"
  local label="$3"
  local got
  got="$(json_field_int "$f" "sub_entry_events")"
  [[ "$got" == "$expect" ]] \
    || fail "$label: sub_entry_events want $expect got $got
$(cat "$f")"
  ok "$label: sub_entry_events=$got"
}

# Fixture table: label | profile | golden jsonl | expected sub_entry_events
run_fixture() {
  local label="$1"
  local profile_rel="$2"
  local golden_rel="$3"
  local expect="$4"

  echo "=== $label: expect sub_entry_events=$expect ==="

  [[ -f "$ROOT/$profile_rel" ]] || fail "missing profile $profile_rel"
  [[ -f "$ROOT/$golden_rel" ]] || fail "missing golden $golden_rel"

  # --- Perl query --json --jsonl (always required; pure-Perl) ---
  local pout="$TMPDIR_SMOKE/${label}_perl_jsonl.out"
  local perr="$TMPDIR_SMOKE/${label}_perl_jsonl.err"
  if ! "${ENGINE[@]}" query --json --jsonl "$golden_rel" >"$pout" 2>"$perr"; then
    cat "$perr" >&2 || true
    cat "$pout" >&2 || true
    fail "$label: query --json --jsonl failed"
  fi
  cat "$pout"
  assert_sub_entry "$pout" "$expect" "$label perl query --json --jsonl"

  # Second run consistency on Perl path
  local pout2="$TMPDIR_SMOKE/${label}_perl_jsonl_2.out"
  if ! "${ENGINE[@]}" query --json --jsonl "$golden_rel" >"$pout2" 2>"$TMPDIR_SMOKE/${label}_perl_jsonl_2.err"; then
    fail "$label: query --json --jsonl run #2 failed"
  fi
  local se1 se2
  se1="$(json_field_int "$pout" "sub_entry_events")"
  se2="$(json_field_int "$pout2" "sub_entry_events")"
  [[ "$se1" == "$se2" ]] || fail "$label: perl sub_entry_events inconsistent ($se1 vs $se2)"
  ok "$label perl query --json consistent ($se1)"

  # --- Native report --json (when CLI available) ---
  if [[ "$CLI_MODE" != "none" ]]; then
    local nout="$TMPDIR_SMOKE/${label}_native.out"
    local nerr="$TMPDIR_SMOKE/${label}_native.err"
    if ! "${CLI_CMD[@]}" report --json "$profile_rel" >"$nout" 2>"$nerr"; then
      cat "$nerr" >&2 || true
      cat "$nout" >&2 || true
      fail "$label: native report --json failed"
    fi
    cat "$nout"
    assert_sub_entry "$nout" "$expect" "$label native report --json"

    # Cross-check shared field equal between sides
    local nse pse
    nse="$(json_field_int "$nout" "sub_entry_events")"
    pse="$(json_field_int "$pout" "sub_entry_events")"
    [[ "$nse" == "$pse" ]] \
      || fail "$label: native sub_entry_events ($nse) != perl ($pse)"
    ok "$label native↔perl sub_entry_events equal ($nse)"

    # Optional: Perl query --json via native dump of live profile
    local dout="$TMPDIR_SMOKE/${label}_perl_profile.out"
    local derr="$TMPDIR_SMOKE/${label}_perl_profile.err"
    if "${ENGINE[@]}" query --json "$profile_rel" >"$dout" 2>"$derr"; then
      assert_sub_entry "$dout" "$expect" "$label perl query --json <profile>"
    else
      log "NOTE: $label query --json <profile> skipped/failed (native dump path optional)
$(cat "$derr" 2>/dev/null || true)"
    fi
  else
    log "SKIP native report --json for $label (no native CLI)"
  fi
}

run_fixture \
  "default-calls1" \
  "fixtures/v5/default-calls1/nytprof.out" \
  "fixtures/v5/default-calls1/readstream.jsonl" \
  "0"

run_fixture \
  "calls2-default" \
  "fixtures/v5/calls2-default/nytprof.out" \
  "fixtures/v5/calls2-default/readstream.jsonl" \
  "27"

ok "json_sub_entry_smoke completed successfully (default-calls1=0, calls2-default=27)"
exit 0
