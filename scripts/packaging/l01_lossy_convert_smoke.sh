#!/usr/bin/env bash
# L01 — optional --allow-lossy convert (strict remains default).
#
# Drives the shipped nytprof-cli convert entry on the known oracle refuse
# fixture fixtures/v5/default-calls1/nytprof.out (fractional PID_START NV).
# Strict must fail closed (no OK: convert). --allow-lossy must write NYTPROF6.
#
# Exit 0: pass or honest SKIP (no cargo/native). Exit 1: convert/honesty fail.
# Exit 2: misuse / crates/ on PERL5LIB.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE="$ROOT/fixtures/v5/default-calls1/nytprof.out"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }

usage() {
  cat <<'EOF'
Usage: l01_lossy_convert_smoke.sh

L01: real convert --allow-lossy on oracle default-calls1 refuse case.
Strict default still refuses. Not TEST-008 / not packing convert.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    *) printf 'ERROR: unknown flag: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

echo "l01_lossy_convert_smoke: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary (S2 not claimed)"
echo "never crates/ on PERL5LIB; strict convert remains default"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$FIXTURE" ]] || fail "missing refuse fixture $FIXTURE"
head5="$(head -c 12 "$FIXTURE" | tr -d '\0' || true)"
[[ "$head5" == "NYTProf 5"* ]] || fail "$FIXTURE: expected NYTProf 5 header"

# Flag must exist on the shipped CLI source (even when no binary).
grep -F -q -- '--allow-lossy' "$ROOT/crates/nytprof-cli/src/main.rs" \
  || fail "shipped CLI source missing --allow-lossy"
ok "shipped convert CLI source names --allow-lossy"

CLI=""
CLI_CMD=()
if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
  CLI="$NYTPROF_NATIVE_CLI"
  CLI_CMD=("$CLI")
elif command -v cargo >/dev/null 2>&1 && [[ -f "$ROOT/Cargo.toml" ]]; then
  cargo build -q -p nytprof-cli
  if [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
    CLI="$ROOT/target/debug/nytprof-dump"
    CLI_CMD=("$CLI")
  else
    CLI="cargo"
    CLI_CMD=(cargo run -q -p nytprof-cli --)
  fi
elif [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
  CLI="$ROOT/target/debug/nytprof-dump"
  CLI_CMD=("$CLI")
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  CLI="$ROOT/prefix/bin/nytprof-cli"
  CLI_CMD=("$CLI")
fi

if [[ -z "$CLI" ]]; then
  echo "SKIP: no nytprof-cli / cargo — flag + fixture asserts hold"
  echo "NOTE: not full TEST-008; packing/string-dict convert residual"
  ok "L01-LOSSY-CONVERT (docs/source only)"
  exit 0
fi

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-l01-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT
STRICT_OUT="$WORKDIR/strict.v6"
LOSSY_OUT="$WORKDIR/lossy.v6"

echo "running strict: ${CLI_CMD[*]} convert --to=v6 $FIXTURE -o $STRICT_OUT"
set +e
STRICT_OUT_TXT="$("${CLI_CMD[@]}" convert --to=v6 "$FIXTURE" -o "$STRICT_OUT" 2>"$WORKDIR/strict.err")"
STRICT_RC=$?
set -e
STRICT_ERR="$(cat "$WORKDIR/strict.err" 2>/dev/null || true)"
printf '%s\n' "$STRICT_OUT_TXT"
printf '%s\n' "$STRICT_ERR"
[[ "$STRICT_RC" -ne 0 ]] || fail "strict convert of oracle fixture must refuse (got 0)"
if printf '%s\n' "$STRICT_OUT_TXT" | grep -E -q '^OK: convert'; then
  fail "strict convert must not print OK: convert"
fi
grep -Eiq 'fractional|PID_|strict' <<<"$STRICT_ERR$STRICT_OUT_TXT" \
  || fail "strict refuse must mention fractional/PID_/strict"
ok "strict convert refuses oracle default-calls1 (fractional NV)"

echo "running lossy: ${CLI_CMD[*]} convert --to=v6 --allow-lossy $FIXTURE -o $LOSSY_OUT"
set +e
LOSSY_TXT="$("${CLI_CMD[@]}" convert --to=v6 --allow-lossy "$FIXTURE" -o "$LOSSY_OUT" 2>"$WORKDIR/lossy.err")"
LOSSY_RC=$?
set -e
LOSSY_ERR="$(cat "$WORKDIR/lossy.err" 2>/dev/null || true)"
printf '%s\n' "$LOSSY_TXT"
printf '%s\n' "$LOSSY_ERR"
[[ "$LOSSY_RC" -eq 0 ]] || fail "allow-lossy convert failed (rc=$LOSSY_RC)"
printf '%s\n' "$LOSSY_TXT" | grep -E -q '^OK: convert' \
  || fail "allow-lossy must print OK: convert"
[[ -f "$LOSSY_OUT" ]] || fail "allow-lossy did not write $LOSSY_OUT"
magic="$(head -c 8 "$LOSSY_OUT" || true)"
[[ "$magic" == "NYTPROF6" ]] || fail "allow-lossy want NYTPROF6 (got $(printf %q "$magic"))"
ok "allow-lossy convert wrote NYTPROF6 via shipped convert"

echo "NOT-YET: packing/string-dict v6 convert / full TEST-008"
echo "NOT-YET: BUILD-003-FULL / PRODUCT-V6-COLLECT-EL8 / S2 / R3-R4 flip"
ok "L01-LOSSY-CONVERT"
exit 0
