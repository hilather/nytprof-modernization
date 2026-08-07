#!/usr/bin/env bash
# Native dump structural parity vs golden readstream.jsonl.
#
# Spec: docs/schemas/native-dump-parity-mvp-v0.md
# Board: NATIVE-DUMP-PARITY / DUMP-PARITY-EXPAND
#
# Uses the shipped CLI dump only (no dump reimplementation).
# Does NOT require oracle Perl env / PERL5LIB. Never puts crates/ on PERL5LIB.
#
# Checks (per fixture):
#   1) Dump profile twice → normalize both → structural match (stability)
#   2) Normalize golden readstream.jsonl
#   3) compare_jsonl.pl golden.norm vs native.norm → full match
#   4) Tag multiplicity TIME_LINE / TIME_BLOCK / SUB_RETURN equal on both
#      sides — counts come from the golden dump for that fixture (not
#      hard-coded default-calls1 values). blocks-calls1 uses TIME_BLOCK
#      for statement timing (TIME_LINE may be 0).
#
# Usage (from repo root or any cwd):
#   bash tools/oracle/selftest_native_dump_parity.sh
#       # default: default-calls1 (compat)
#   bash tools/oracle/selftest_native_dump_parity.sh calls2-default
#   bash tools/oracle/selftest_native_dump_parity.sh blocks-calls1
#   bash tools/oracle/selftest_native_dump_parity.sh default-calls1 calls2-default blocks-calls1
#   bash tools/oracle/selftest_native_dump_parity.sh fixtures/v5/calls2-default
#   ./tools/oracle/selftest_native_dump_parity.sh   # if executable
#
# All-fixture helper:
#   bash tools/oracle/selftest_native_dump_parity_all.sh
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIR/../.." && pwd)"
cd "$ROOT"

NORMALIZE=(python3 "$DIR/normalize_jsonl.py")
COMPARE=(perl "$DIR/compare_jsonl.pl")

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

need_file() {
  [[ -f "$1" ]] || fail "missing $1"
}

# Resolve a fixture argument to a short name under fixtures/v5/.
# Accepts: "default-calls1", "fixtures/v5/default-calls1", absolute path, trailing slash.
resolve_fixture_name() {
  local arg="$1"
  # Strip trailing slash
  arg="${arg%/}"
  # Absolute or relative path containing fixtures/v5/<name>
  if [[ "$arg" == *"/fixtures/v5/"* ]]; then
    arg="${arg##*/fixtures/v5/}"
  elif [[ "$arg" == fixtures/v5/* ]]; then
    arg="${arg#fixtures/v5/}"
  fi
  # Bare name remaining
  if [[ "$arg" == */* ]]; then
    # Absolute path to fixture dir: take last component
    if [[ -d "$arg" ]]; then
      arg="$(basename "$arg")"
    else
      fail "unrecognized fixture path: $1"
    fi
  fi
  [[ -n "$arg" ]] || fail "empty fixture name from: $1"
  printf '%s' "$arg"
}

need_file "$DIR/normalize_jsonl.py"
need_file "$DIR/compare_jsonl.pl"

# ---------------------------------------------------------------------------
# Resolve native dump CLI once (prefer cargo run of shipped path)
# ---------------------------------------------------------------------------
DUMP_MODE=""
if command -v cargo >/dev/null 2>&1; then
  DUMP_MODE="cargo"
elif [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
  DUMP_MODE="prefix-cli"
elif [[ -x "$ROOT/prefix/bin/nytprof-dump" ]]; then
  DUMP_MODE="prefix-dump"
elif [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
  DUMP_MODE="debug"
elif [[ -x "$ROOT/target/release/nytprof-dump" ]]; then
  DUMP_MODE="release"
else
  fail "no cargo and no prefix/target nytprof-cli/nytprof-dump binary found"
fi

run_native_dump() {
  # $1 = profile path; stdout = JSONL dump
  local profile="$1"
  case "$DUMP_MODE" in
    cargo)       cargo run -q -p nytprof-cli -- dump "$profile" ;;
    prefix-cli)  "$ROOT/prefix/bin/nytprof-cli" dump "$profile" ;;
    prefix-dump) "$ROOT/prefix/bin/nytprof-dump" dump "$profile" ;;
    debug)       "$ROOT/target/debug/nytprof-dump" dump "$profile" ;;
    release)     "$ROOT/target/release/nytprof-dump" dump "$profile" ;;
    *)           fail "internal: unknown DUMP_MODE=$DUMP_MODE" ;;
  esac
}

WORK="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-native-dump-parity.XXXXXX")"
cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

# ---------------------------------------------------------------------------
# Per-fixture parity
# ---------------------------------------------------------------------------
run_fixture_parity() {
  local name="$1"
  local fixture_dir="fixtures/v5/$name"
  local profile="$ROOT/$fixture_dir/nytprof.out"
  local golden="$ROOT/$fixture_dir/readstream.jsonl"

  need_file "$profile"
  need_file "$golden"

  local tmp="$WORK/$name"
  mkdir -p "$tmp"

  local native1="$tmp/native1.jsonl"
  local native2="$tmp/native2.jsonl"
  local native1_norm="$tmp/native1.norm.jsonl"
  local native2_norm="$tmp/native2.norm.jsonl"
  local golden_norm="$tmp/golden.norm.jsonl"

  log "=== fixture: $name ==="

  # 1) Dump twice (stability)
  log "=== $name: native dump (1/2) ==="
  if ! run_native_dump "$profile" >"$native1" 2>"$tmp/dump1.err"; then
    cat "$tmp/dump1.err" >&2 || true
    fail "native dump #1 failed for $name"
  fi
  [[ -s "$native1" ]] || fail "native dump #1 empty for $name"

  log "=== $name: native dump (2/2) ==="
  if ! run_native_dump "$profile" >"$native2" 2>"$tmp/dump2.err"; then
    cat "$tmp/dump2.err" >&2 || true
    fail "native dump #2 failed for $name"
  fi
  [[ -s "$native2" ]] || fail "native dump #2 empty for $name"

  log "=== $name: normalize native dumps ==="
  "${NORMALIZE[@]}" --mode structural "$native1" >"$native1_norm"
  "${NORMALIZE[@]}" --mode structural "$native2" >"$native2_norm"

  if ! "${COMPARE[@]}" "$native1_norm" "$native2_norm" >"$tmp/compare_stability.out"; then
    cat "$tmp/compare_stability.out" >&2 || true
    fail "native dump not stable for $name: two dumps differ after normalize"
  fi
  ok "$name native dump stability (dump×2 normalize match)"

  # 2–3) Golden vs native full structural compare
  log "=== $name: normalize golden readstream.jsonl ==="
  "${NORMALIZE[@]}" --mode structural "$golden" >"$golden_norm"

  log "=== $name: compare_jsonl golden.norm vs native.norm ==="
  if ! "${COMPARE[@]}" "$golden_norm" "$native1_norm" >"$tmp/compare_golden.out"; then
    cat "$tmp/compare_golden.out" >&2 || true
    fail "native dump structural mismatch vs golden $fixture_dir/readstream.jsonl"
  fi
  # Surface the comparator OK line for operators / evidence.
  cat "$tmp/compare_golden.out"
  ok "$name full structural equality (golden vs native after normalize)"

  # 4) Tag multiplicity: counts from this fixture's golden, not hard-coded.
  log "=== $name: tag multiplicity TIME_LINE / TIME_BLOCK / SUB_RETURN ==="
  python3 - "$name" "$golden_norm" "$native1_norm" <<'PY'
import json
import sys
from collections import Counter

fixture_name = sys.argv[1]
tags_of_interest = ("TIME_LINE", "TIME_BLOCK", "SUB_RETURN")

def count_tags(path: str) -> Counter:
    c = Counter()
    with open(path, encoding="utf-8") as f:
        for lineno, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            try:
                o = json.loads(line)
            except json.JSONDecodeError as e:
                print(f"ERROR: {path}:{lineno}: {e}", file=sys.stderr)
                sys.exit(1)
            tag = o.get("tag")
            if tag in tags_of_interest:
                c[tag] += 1
    return c

golden_path, native_path = sys.argv[2], sys.argv[3]
g = count_tags(golden_path)
n = count_tags(native_path)

# Ensure keys present (0 is valid — e.g. TIME_BLOCK on default-calls1,
# TIME_LINE on blocks-calls1).
for t in tags_of_interest:
    g.setdefault(t, 0)
    n.setdefault(t, 0)

print(f"  fixture: {fixture_name}")
print("  golden:", {t: g[t] for t in tags_of_interest})
print("  native:", {t: n[t] for t in tags_of_interest})

failed = False
for t in tags_of_interest:
    if g[t] != n[t]:
        print(
            f"ERROR: {fixture_name}: {t} count mismatch golden={g[t]} native={n[t]}",
            file=sys.stderr,
        )
        failed = True

if failed:
    sys.exit(1)

# Fixture-class sanity derived from *this* golden, not default-calls1 numbers:
# - Some statement timing must exist (TIME_LINE and/or TIME_BLOCK).
# - SUB_RETURN must be present on these workloads.
# Multiplicity values themselves are never hard-coded.
if g["TIME_LINE"] + g["TIME_BLOCK"] == 0:
    print(
        f"ERROR: {fixture_name}: expected TIME_LINE+TIME_BLOCK > 0 on golden",
        file=sys.stderr,
    )
    sys.exit(1)
if g["SUB_RETURN"] == 0:
    print(
        f"ERROR: {fixture_name}: expected SUB_RETURN > 0 on golden",
        file=sys.stderr,
    )
    sys.exit(1)

print(
    f"  multiplicity match: TIME_LINE={g['TIME_LINE']} "
    f"TIME_BLOCK={g['TIME_BLOCK']} SUB_RETURN={g['SUB_RETURN']}"
)
sys.exit(0)
PY
  ok "$name tag multiplicity TIME_LINE/TIME_BLOCK/SUB_RETURN match"

  # Optional durable capture (nest by fixture so multi-fixture runs don't clobber)
  if [[ -n "${NATIVE_DUMP_PARITY_KEEP_DIR:-}" ]]; then
    local keep="$NATIVE_DUMP_PARITY_KEEP_DIR/$name"
    mkdir -p "$keep"
    cp -a "$native1" "$keep/native.jsonl"
    cp -a "$native1_norm" "$keep/native.norm.jsonl"
    cp -a "$golden_norm" "$keep/golden.norm.jsonl"
    cp -a "$tmp/compare_golden.out" "$keep/compare_golden.out"
    ok "$name kept dump parity evidence under $keep"
  fi

  ok "$name native dump parity smoke passed (full structural equality)"
}

# ---------------------------------------------------------------------------
# Fixture list: default default-calls1; else args as names or paths
# ---------------------------------------------------------------------------
FIXTURES=()
if [[ $# -eq 0 ]]; then
  FIXTURES=(default-calls1)
else
  for arg in "$@"; do
    FIXTURES+=("$(resolve_fixture_name "$arg")")
  done
fi

log "native dump parity: dump mode=$DUMP_MODE fixtures=${FIXTURES[*]}"

for name in "${FIXTURES[@]}"; do
  run_fixture_parity "$name"
done

log ""
ok "native dump parity smoke passed (${FIXTURES[*]})"
exit 0
