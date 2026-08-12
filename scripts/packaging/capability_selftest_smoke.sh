#!/usr/bin/env bash
# CAPABILITY-SELFTEST / CAPABILITY-JSON-MVP: native offline capability smoke.
#
# Spec: docs/schemas/capability-selftest-mvp-v0.md
# Board: CAPABILITY-SELFTEST, CAPABILITY-JSON-MVP
#
# Resolves the native CLI, runs `capability` twice (both exit 0), checks
# stable markers are present and consistent; then runs `capability --json`
# twice and asserts structured fields. Prefer fail when no native
# binary/cargo is available (packaging-native path). Never puts crates/ on
# oracle PERL5LIB.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/capability_selftest_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE_REL="fixtures/v5/default-calls1/nytprof.out"
ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

# ---------------------------------------------------------------------------
# Resolve native CLI (same spirit as tools/oracle/selftest_fail_closed.sh).
# Prefer cargo when present so the smoke exercises the current tree; pin via
# NYTPROF_NATIVE_CLI / prefix when testing a packaged install without cargo.
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
  fail "no native CLI found (packaging-native smoke fails closed)
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

# Sanity: never inject crates/ into oracle PERL5LIB from this smoke.
if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ (got: $PERL5LIB)"
fi

# ---------------------------------------------------------------------------
# Run capability twice; both must exit 0 with consistent markers.
# ---------------------------------------------------------------------------
OUT1="$(mktemp)"
OUT2="$(mktemp)"
ERR1="$(mktemp)"
ERR2="$(mktemp)"
trap 'rm -f "$OUT1" "$OUT2" "$ERR1" "$ERR2"' EXIT

set +e
"${CLI_CMD[@]}" capability >"$OUT1" 2>"$ERR1"
RC1=$?
"${CLI_CMD[@]}" capability >"$OUT2" 2>"$ERR2"
RC2=$?
set -e

if [[ "$RC1" -ne 0 ]]; then
  cat "$ERR1" >&2 || true
  cat "$OUT1" >&2 || true
  fail "capability run #1 exited $RC1 (expected 0)"
fi
if [[ "$RC2" -ne 0 ]]; then
  cat "$ERR2" >&2 || true
  cat "$OUT2" >&2 || true
  fail "capability run #2 exited $RC2 (expected 0)"
fi
ok "capability ×2 exited 0"

# Required stable markers (present on both runs).
# E5 honesty: v6_decode/v6_report yes; convert/merge no; collection_default v5.
for marker in \
  "OK: native capability self-test" \
  "decode: yes" \
  "report: yes" \
  "verify: yes" \
  "v6_decode: yes" \
  "v6_report: yes" \
  "convert: no" \
  "merge: no" \
  "collection_default: v5"
do
  grep -qxF "$marker" "$OUT1" || fail "run #1 missing marker: $marker\n$(cat "$OUT1")"
  grep -qxF "$marker" "$OUT2" || fail "run #2 missing marker: $marker\n$(cat "$OUT2")"
done
ok "stable markers present on both runs (incl. E5 honesty)"

# Consistency: extract contract lines (+ profile probes) and compare.
# profile_ok / v6_profile_ok may be path or skip; both runs must match each other.
extract_core() {
  # Keep only the contract lines (ignore any trailing noise).
  grep -E '^(OK: native capability self-test|decode: |report: |verify: |v6_decode: |v6_report: |convert: |merge: |collection_default: |profile_ok: |v6_profile_ok: )' "$1" | sort
}

CORE1="$(extract_core "$OUT1")"
CORE2="$(extract_core "$OUT2")"
if [[ "$CORE1" != "$CORE2" ]]; then
  fail "capability output not consistent across two runs
--- run1 ---
$CORE1
--- run2 ---
$CORE2"
fi
ok "capability output consistent across two runs"

# profile_ok line must exist
grep -qE '^profile_ok: ' "$OUT1" || fail "missing profile_ok: line\n$(cat "$OUT1")"

# In-repo golden fixture: probe must not be skip when fixture is present.
if [[ -f "$ROOT/$FIXTURE_REL" ]]; then
  if grep -qxF 'profile_ok: skip' "$OUT1"; then
    fail "fixture $FIXTURE_REL present but profile_ok: skip
$(cat "$OUT1")"
  fi
  grep -qE '^profile_ok: .+' "$OUT1" || fail "expected non-skip profile_ok with fixture present"
  # path should mention nytprof.out or default-calls1
  if ! grep -qE '^profile_ok: .*(default-calls1|nytprof\.out)' "$OUT1"; then
    log "NOTE: profile_ok path (for operators):"
    grep -E '^profile_ok: ' "$OUT1" || true
  fi
  ok "profile_ok probed golden fixture (not skip)"
else
  log "NOTE: $FIXTURE_REL absent; accepting profile_ok: skip or other probe"
fi

# Optional aliases smoke (selftest / capabilities) — cheap, same markers.
for alias in selftest capabilities; do
  set +e
  ALIAS_OUT="$("${CLI_CMD[@]}" "$alias" 2>/dev/null)"
  ALIAS_RC=$?
  set -e
  [[ "$ALIAS_RC" -eq 0 ]] || fail "alias '$alias' exited $ALIAS_RC"
  printf '%s\n' "$ALIAS_OUT" | grep -qxF 'OK: native capability self-test' \
    || fail "alias '$alias' missing OK marker"
done
ok "aliases selftest + capabilities exit 0 with OK marker"

# ---------------------------------------------------------------------------
# CAPABILITY-JSON-MVP: capability --json twice; parse + field checks.
# ---------------------------------------------------------------------------
JOUT1="$(mktemp)"
JOUT2="$(mktemp)"
JERR1="$(mktemp)"
JERR2="$(mktemp)"
trap 'rm -f "$OUT1" "$OUT2" "$ERR1" "$ERR2" "$JOUT1" "$JOUT2" "$JERR1" "$JERR2"' EXIT

set +e
"${CLI_CMD[@]}" capability --json >"$JOUT1" 2>"$JERR1"
JRC1=$?
"${CLI_CMD[@]}" capability --json >"$JOUT2" 2>"$JERR2"
JRC2=$?
set -e

if [[ "$JRC1" -ne 0 ]]; then
  cat "$JERR1" >&2 || true
  cat "$JOUT1" >&2 || true
  fail "capability --json run #1 exited $JRC1 (expected 0)"
fi
if [[ "$JRC2" -ne 0 ]]; then
  cat "$JERR2" >&2 || true
  cat "$JOUT2" >&2 || true
  fail "capability --json run #2 exited $JRC2 (expected 0)"
fi
ok "capability --json ×2 exited 0"

# Parse / validate JSON (prefer python3 json.tool; fall back to perl JSON::PP; else key grep).
json_validate_file() {
  local f="$1"
  local label="$2"
  if command -v python3 >/dev/null 2>&1; then
    python3 -m json.tool <"$f" >/dev/null \
      || fail "$label: invalid JSON (python3 -m json.tool)\n$(cat "$f")"
    # Field assertions via python
    if ! python3 - "$f" <<'PY'
import json, sys
path = sys.argv[1]
with open(path, encoding="utf-8") as fh:
    obj = json.load(fh)
if not isinstance(obj, dict):
    sys.exit("not an object")
for k in ("ok", "decode", "report", "verify", "v6_decode", "v6_report"):
    if obj.get(k) is not True:
        sys.exit(f"{k} must be true, got {obj.get(k)!r}")
for k in ("convert", "merge"):
    if obj.get(k) is not False:
        sys.exit(f"{k} must be false (residual honesty), got {obj.get(k)!r}")
if obj.get("collection_default") != "v5":
    sys.exit(f"collection_default must be 'v5', got {obj.get('collection_default')!r}")
if "profile_ok" not in obj:
    sys.exit("missing profile_ok")
if "v6_profile_ok" not in obj:
    sys.exit("missing v6_profile_ok")
# profile_ok / v6_profile_ok: str or null only
for key in ("profile_ok", "v6_profile_ok"):
    po = obj[key]
    if po is not None and not isinstance(po, str):
        sys.exit(f"{key} must be str or null, got {type(po).__name__}")
PY
    then
      fail "$label: required JSON fields missing or false
$(cat "$f")"
    fi
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      use strict; use warnings;
      open my $fh, "<", $ARGV[0] or die $!;
      local $/; my $raw = <$fh>;
      my $obj = JSON::PP->new->decode($raw);
      die "not object\n" unless ref($obj) eq "HASH";
      for my $k (qw(ok decode report verify v6_decode v6_report)) {
        die "$k must be true\n" unless $obj->{$k};
      }
      for my $k (qw(convert merge)) {
        die "$k must be false\n" if $obj->{$k};
      }
      die "collection_default\n" unless defined $obj->{collection_default}
        && $obj->{collection_default} eq "v5";
      die "missing profile_ok\n" unless exists $obj->{profile_ok};
      die "missing v6_profile_ok\n" unless exists $obj->{v6_profile_ok};
      for my $k (qw(profile_ok v6_profile_ok)) {
        my $po = $obj->{$k};
        die "$k type\n" if defined $po && ref($po);
      }
    ' "$f" || fail "$label: invalid JSON or fields (perl JSON::PP)\n$(cat "$f")"
  else
    # Last-resort structured greps (compact single-line object).
    grep -qE '"ok"[[:space:]]*:[[:space:]]*true' "$f" \
      || fail "$label: missing ok:true\n$(cat "$f")"
    grep -qE '"decode"[[:space:]]*:[[:space:]]*true' "$f" \
      || fail "$label: missing decode:true\n$(cat "$f")"
    grep -qE '"report"[[:space:]]*:[[:space:]]*true' "$f" \
      || fail "$label: missing report:true\n$(cat "$f")"
    grep -qE '"verify"[[:space:]]*:[[:space:]]*true' "$f" \
      || fail "$label: missing verify:true\n$(cat "$f")"
    grep -qE '"v6_decode"[[:space:]]*:[[:space:]]*true' "$f" \
      || fail "$label: missing v6_decode:true\n$(cat "$f")"
    grep -qE '"v6_report"[[:space:]]*:[[:space:]]*true' "$f" \
      || fail "$label: missing v6_report:true\n$(cat "$f")"
    grep -qE '"convert"[[:space:]]*:[[:space:]]*false' "$f" \
      || fail "$label: missing convert:false\n$(cat "$f")"
    grep -qE '"merge"[[:space:]]*:[[:space:]]*false' "$f" \
      || fail "$label: missing merge:false\n$(cat "$f")"
    grep -qE '"collection_default"[[:space:]]*:[[:space:]]*"v5"' "$f" \
      || fail "$label: missing collection_default:v5\n$(cat "$f")"
    grep -qE '"profile_ok"' "$f" \
      || fail "$label: missing profile_ok\n$(cat "$f")"
    grep -qE '"v6_profile_ok"' "$f" \
      || fail "$label: missing v6_profile_ok\n$(cat "$f")"
    log "NOTE: no python3/perl JSON::PP; used key greps for $label"
  fi
}

json_validate_file "$JOUT1" "json run #1"
json_validate_file "$JOUT2" "json run #2"
ok "capability --json fields ok (incl. E5 honesty) on both runs"

# Consistency of profile_ok across two JSON runs (and vs fixture presence).
json_profile_ok() {
  local f="$1"
  if command -v python3 >/dev/null 2>&1; then
    python3 -c 'import json,sys; o=json.load(open(sys.argv[1],encoding="utf-8")); v=o.get("profile_ok"); print("null" if v is None else v)' "$f"
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      open my $fh, "<", $ARGV[0] or die $!;
      local $/; my $obj = JSON::PP->new->decode(<$fh>);
      print defined $obj->{profile_ok} ? $obj->{profile_ok} : "null";
    ' "$f"
  else
    # Extract profile_ok value roughly for consistency compare.
    if grep -qE '"profile_ok"[[:space:]]*:[[:space:]]*null' "$f"; then
      printf 'null'
    else
      sed -n 's/.*"profile_ok"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$f" | head -n1
    fi
  fi
}

JPO1="$(json_profile_ok "$JOUT1")"
JPO2="$(json_profile_ok "$JOUT2")"
if [[ "$JPO1" != "$JPO2" ]]; then
  fail "capability --json profile_ok not consistent
--- run1 ---
$JPO1
--- run2 ---
$JPO2
--- raw1 ---
$(cat "$JOUT1")
--- raw2 ---
$(cat "$JOUT2")"
fi
ok "capability --json profile_ok consistent across two runs ($JPO1)"

if [[ -f "$ROOT/$FIXTURE_REL" ]]; then
  if [[ "$JPO1" == "null" || -z "$JPO1" ]]; then
    fail "fixture $FIXTURE_REL present but JSON profile_ok is null/empty
$(cat "$JOUT1")"
  fi
  if [[ "$JPO1" != *"nytprof.out"* && "$JPO1" != *"default-calls1"* ]]; then
    log "NOTE: JSON profile_ok path (for operators): $JPO1"
  fi
  ok "JSON profile_ok probed golden fixture (not null)"
else
  log "NOTE: $FIXTURE_REL absent; accepting JSON profile_ok null or other probe ($JPO1)"
fi

# --format=json accepted and equivalent core fields to --json
set +e
FMT_OUT="$("${CLI_CMD[@]}" capability --format=json 2>/dev/null)"
FMT_RC=$?
set -e
[[ "$FMT_RC" -eq 0 ]] || fail "capability --format=json exited $FMT_RC"
printf '%s\n' "$FMT_OUT" >"$JOUT1"
json_validate_file "$JOUT1" "--format=json"
ok "capability --format=json accepted with required fields"

ok "capability self-test packaging smoke passed (human + JSON)"
exit 0
