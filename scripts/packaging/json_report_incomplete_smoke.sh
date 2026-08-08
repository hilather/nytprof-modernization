#!/usr/bin/env bash
# JSON-REPORT-INCOMPLETE-FAILCLOSED: shipped report --json (and aggregates
# aliases) must fail closed on record-aligned incomplete streams (COMPAT-010).
#
# Contract: docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md
# Schema:   docs/schemas/native-aggregates-json-mvp-v0.md (Fail-closed load)
# Board:    JSON-REPORT-INCOMPLETE-FAILCLOSED
#
# Cases:
#   1) Craft 500-byte record-aligned prefix of fixtures/v5/default-calls1
#   2) report --json on prefix → exit ≠ 0
#   3) stdout must NOT be a successful complete ok:true object with
#      is_stream_complete:true
#   4) aggregates / agg aliases also fail closed
#   5) golden default-calls1 report --json still exit 0 with ok:true complete
#   6) optional salvage: NYTPROF_ALLOW_INCOMPLETE=1 on prefix may emit JSON
#      with is_stream_complete false / non-empty reasons (not complete-ok)
#
# Aligns with tools/oracle/selftest_incomplete_stream.sh (text report/verify)
# and load_model_for_report / require_complete_stream.
# Never puts crates/ on oracle PERL5LIB. No XS.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/json_report_incomplete_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE_REL="fixtures/v5/default-calls1/nytprof.out"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

# ---------------------------------------------------------------------------
# Resolve native CLI (prefer cargo when present so smoke exercises current tree).
# ---------------------------------------------------------------------------
CLI_MODE="" # binary | cargo
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
  fail "no native CLI found (JSON-REPORT-INCOMPLETE-FAILCLOSED fails closed without native)
  looked for: \$NYTPROF_NATIVE_CLI, cargo + workspace Cargo.toml,
  prefix/bin/{nytprof-cli,nytprof-dump}, target/{debug,release}/nytprof-dump
  Install: ./scripts/packaging/install_native.sh
  Or build: cargo build -p nytprof-cli"
fi

if [[ "$CLI_MODE" == "binary" ]]; then
  CLI_CMD=("$CLI_BIN")
  ok "using native binary: $CLI_BIN"
else
  CLI_CMD=(cargo run -q -p nytprof-cli --)
  ok "using cargo run -p nytprof-cli"
fi

[[ -f "$ROOT/$FIXTURE_REL" ]] || fail "missing fixture $FIXTURE_REL"

# Sanity: never inject crates/ into oracle PERL5LIB from this smoke.
if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ (got: $PERL5LIB)"
fi

TMPDIR_SMOKE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT

PREFIX_OUT="$TMPDIR_SMOKE/prefix-500.out"
if command -v python3 >/dev/null 2>&1; then
  python3 - "$ROOT/$FIXTURE_REL" "$PREFIX_OUT" <<'PY'
import sys
src, dest = sys.argv[1], sys.argv[2]
data = open(src, "rb").read()
assert len(data) > 500, len(data)
open(dest, "wb").write(data[:500])
PY
else
  # dd fallback when python3 absent
  head -c 500 "$ROOT/$FIXTURE_REL" >"$PREFIX_OUT" \
    || fail "could not craft 500-byte prefix"
  sz=$(wc -c <"$PREFIX_OUT" | tr -d ' ')
  [[ "$sz" == "500" ]] || fail "prefix size $sz != 500"
fi
ok "crafted incomplete 500-byte prefix of default-calls1"

# Assert stdout is not a successful complete JSON stream object.
# Rejects: {"ok":true,... "is_stream_complete":true ...} as a full success claim.
assert_not_successful_complete_ok_json() {
  local label="$1"
  local stdout_file="$2"

  # Empty stdout is fine on fail-closed (errors typically go to stderr).
  if [[ ! -s "$stdout_file" ]]; then
    return 0
  fi

  if command -v python3 >/dev/null 2>&1; then
    python3 -c '
import json,sys
label, path = sys.argv[1], sys.argv[2]
raw = open(path, encoding="utf-8", errors="replace").read().strip()
if not raw:
    sys.exit(0)
# Multi-line text reports are not the success JSON object.
# Only reject a parseable JSON object that claims complete success.
try:
    o = json.loads(raw)
except Exception:
    # Non-JSON stdout on failure is acceptable (must not be complete-ok JSON).
    sys.exit(0)
if not isinstance(o, dict):
    sys.exit(0)
ok = o.get("ok")
isc = o.get("is_stream_complete")
ok_true = ok is True or ok == 1 or ok == "true"
isc_true = isc is True or isc == 1 or isc == "true"
if ok_true and isc_true:
    raise SystemExit(
        "%s: stdout is successful complete JSON (ok:true is_stream_complete:true); "
        "must fail closed on incomplete stream\n%s" % (label, raw[:500])
    )
if ok_true and "is_stream_complete" not in o:
    # Pre-stream-field landings: still forbid bare ok:true on incomplete.
    raise SystemExit(
        "%s: stdout is ok:true JSON without incompleteness signal on incomplete input\n%s"
        % (label, raw[:500])
    )
' "$label" "$stdout_file" || fail "$label: complete-ok JSON on incomplete input"
  else
    # Grep fallback: reject compact one-line success with both markers.
    if grep -qE '"ok"[[:space:]]*:[[:space:]]*true' "$stdout_file" \
      && grep -qE '"is_stream_complete"[[:space:]]*:[[:space:]]*true' "$stdout_file"; then
      fail "$label: stdout looks like ok:true + is_stream_complete:true (must fail closed)
$(head -c 500 "$stdout_file")"
    fi
    if grep -qE '"ok"[[:space:]]*:[[:space:]]*true' "$stdout_file" \
      && ! grep -qE 'is_stream_complete' "$stdout_file"; then
      fail "$label: stdout has ok:true without stream-completeness field on incomplete input
$(head -c 500 "$stdout_file")"
    fi
  fi
}

# Default policy: salvage env must be unset.
unset NYTPROF_ALLOW_INCOMPLETE || true

# ---------------------------------------------------------------------------
# Incomplete prefix → report --json / aggregates fail closed
# ---------------------------------------------------------------------------
expect_json_cmd_fails() {
  local label="$1"
  shift
  local out_f="$TMPDIR_SMOKE/fail_stdout.txt"
  local err_f="$TMPDIR_SMOKE/fail_stderr.txt"
  set +e
  "${CLI_CMD[@]}" "$@" >"$out_f" 2>"$err_f"
  local rc=$?
  set -e
  if [[ "$rc" -eq 0 ]]; then
    fail "$label: expected non-zero exit, got 0
stdout:
$(cat "$out_f")
stderr:
$(cat "$err_f")"
  fi
  assert_not_successful_complete_ok_json "$label" "$out_f"
  # Must not look like a normal text-report OK either.
  if grep -qE '^OK:' "$out_f" 2>/dev/null; then
    fail "$label: must not print OK: on incomplete input
$(cat "$out_f")"
  fi
  ok "$label (exit $rc)"
}

echo "=== incomplete prefix: report --json fail-closed ==="
expect_json_cmd_fails "report --json incomplete prefix" report --json "$PREFIX_OUT"
expect_json_cmd_fails "report --format=json incomplete prefix" report --format=json "$PREFIX_OUT"
expect_json_cmd_fails "aggregates incomplete prefix" aggregates "$PREFIX_OUT"
expect_json_cmd_fails "agg incomplete prefix" agg "$PREFIX_OUT"

# ---------------------------------------------------------------------------
# Golden still succeeds with complete ok:true
# ---------------------------------------------------------------------------
echo "=== golden default-calls1: report --json still complete ok ==="
GOLD_OUT="$TMPDIR_SMOKE/golden_json.out"
GOLD_ERR="$TMPDIR_SMOKE/golden_json.err"
set +e
"${CLI_CMD[@]}" report --json "$ROOT/$FIXTURE_REL" >"$GOLD_OUT" 2>"$GOLD_ERR"
GOLD_RC=$?
set -e
if [[ "$GOLD_RC" -ne 0 ]]; then
  fail "golden report --json expected exit 0, got $GOLD_RC
stdout:
$(cat "$GOLD_OUT")
stderr:
$(cat "$GOLD_ERR")"
fi

if command -v python3 >/dev/null 2>&1; then
  python3 -c '
import json,sys
path=sys.argv[1]
o=json.load(open(path,encoding="utf-8"))
assert o.get("ok") is True, o.get("ok")
isc=o.get("is_stream_complete")
assert isc is True or isc == 1, isc
reasons=o.get("incompleteness_reasons")
assert isinstance(reasons, list) and len(reasons)==0, reasons
print("golden ok is_stream_complete=true reasons=[]")
' "$GOLD_OUT" || fail "golden JSON completeness asserts failed
$(cat "$GOLD_OUT")"
else
  grep -qE '"ok"[[:space:]]*:[[:space:]]*true' "$GOLD_OUT" \
    || fail "golden missing ok:true"
  grep -qE '"is_stream_complete"[[:space:]]*:[[:space:]]*true' "$GOLD_OUT" \
    || fail "golden missing is_stream_complete:true"
fi
ok "golden report --json exit 0 with complete ok:true"

# ---------------------------------------------------------------------------
# Salvage path (optional): ALLOW_INCOMPLETE=1 may succeed but must not claim
# a complete stream. Aligns with text verify INCOMPLETE salvage policy.
# ---------------------------------------------------------------------------
echo "=== salvage NYTPROF_ALLOW_INCOMPLETE=1 on incomplete prefix ==="
export NYTPROF_ALLOW_INCOMPLETE=1
SAL_OUT="$TMPDIR_SMOKE/salvage_json.out"
SAL_ERR="$TMPDIR_SMOKE/salvage_json.err"
set +e
"${CLI_CMD[@]}" report --json "$PREFIX_OUT" >"$SAL_OUT" 2>"$SAL_ERR"
SAL_RC=$?
set -e
unset NYTPROF_ALLOW_INCOMPLETE

if [[ "$SAL_RC" -ne 0 ]]; then
  # Salvage is best-effort; if env is accepted at load but render fails, still
  # must not have printed complete-ok. Document as NOTE only if non-zero.
  assert_not_successful_complete_ok_json "salvage non-zero" "$SAL_OUT"
  log "NOTE: salvage report --json exit $SAL_RC (accepted as long as not complete-ok)
stderr:
$(cat "$SAL_ERR")"
else
  # Success under salvage: must not claim is_stream_complete true.
  if command -v python3 >/dev/null 2>&1; then
    python3 -c '
import json,sys
path=sys.argv[1]
raw=open(path,encoding="utf-8",errors="replace").read().strip()
if not raw:
    raise SystemExit("salvage exit 0 but empty stdout")
o=json.loads(raw)
if not isinstance(o, dict):
    raise SystemExit("salvage stdout not a JSON object")
isc=o.get("is_stream_complete")
if isc is True or isc == 1 or isc == "true":
    raise SystemExit(
        "salvage must not claim is_stream_complete:true on incomplete prefix; got %r"
        % (isc,)
    )
# Prefer explicit incompleteness signal when field present.
reasons=o.get("incompleteness_reasons")
if isinstance(reasons, list) and len(reasons)==0 and "is_stream_complete" in o:
    raise SystemExit("salvage incompleteness_reasons empty while incomplete")
print("salvage JSON: ok=%r is_stream_complete=%r reasons=%r"
      % (o.get("ok"), isc, reasons))
' "$SAL_OUT" || fail "salvage complete-stream claim
stdout:
$(cat "$SAL_OUT")
stderr:
$(cat "$SAL_ERR")"
  else
    if grep -qE '"is_stream_complete"[[:space:]]*:[[:space:]]*true' "$SAL_OUT"; then
      fail "salvage must not claim is_stream_complete:true
$(cat "$SAL_OUT")"
    fi
  fi
  ok "salvage report --json exit 0 without complete-stream claim"
fi

# Final isolation guard
if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ after run (got: $PERL5LIB)"
fi

ok "json_report_incomplete_smoke (JSON-REPORT-INCOMPLETE-FAILCLOSED) completed successfully"
exit 0
