#!/usr/bin/env bash
# Light measurement harness for first-slice offline paths (decode / report / optional csv).
#
# NOT a performance certification. Local exploratory timings only.
# See docs/BENCH_NOTES.md — no public performance claims.
#
# Usage:
#   bash tools/bench/light_bench.sh
#   OUT=/tmp/light_bench.txt bash tools/bench/light_bench.sh
#
# Env:
#   OUT=path     Optional file to also write the same report (stdout always printed)
#   FIXTURES=…   Space-separated fixture dirs relative to repo root
#                (default: fixtures/v5/default-calls1 fixtures/v5/default-calls2 if present)
#
# Exit 0 on success (missing optional fixtures/csv are skipped, not failures).

set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIR/../.." && pwd)"
cd "$ROOT"

# --- output capture (stdout always; optional OUT= file) ---
REPORT_LINES=()
emit() {
  REPORT_LINES+=("$*")
  printf '%s\n' "$*"
}

# --- timing: prefer /usr/bin/time -f, else bash TIMEFORMAT, else python3 ---
TIME_MODE=
if [[ -x /usr/bin/time ]] && /usr/bin/time -f '%e' true >/dev/null 2>&1; then
  TIME_MODE=gnu
elif [[ -n "${BASH_VERSION:-}" ]]; then
  TIME_MODE=bash
else
  TIME_MODE=python
fi

# Run command; print wall seconds to stdout (one number). Command stderr preserved on failure.
# Usage: wall_seconds <label> -- <cmd...>
# Sets global LAST_WALL_S
LAST_WALL_S=0
run_timed() {
  local label="$1"
  shift
  if [[ "${1:-}" == "--" ]]; then
    shift
  fi
  local sec
  case "$TIME_MODE" in
    gnu)
      # GNU time writes to stderr by default; capture only the format line.
      # Run command with its stdout/stderr discarded for clean bench (caller redirects).
      local tmp
      tmp=$(mktemp)
      set +e
      /usr/bin/time -f '%e' -o "$tmp" -- "$@" >/dev/null 2>&1
      local rc=$?
      set -e
      sec=$(tr -d ' \n' <"$tmp" || true)
      rm -f "$tmp"
      if [[ $rc -ne 0 ]]; then
        emit "ERROR: step '$label' failed (exit $rc)"
        exit "$rc"
      fi
      ;;
    bash)
      local tfile
      tfile=$(mktemp)
      # shellcheck disable=SC2034
      TIMEFORMAT='%R'
      set +e
      {
        time "$@" >/dev/null 2>&1
      } 2>"$tfile"
      local rc=$?
      set -e
      sec=$(tr -d ' \n' <"$tfile" || true)
      rm -f "$tfile"
      if [[ $rc -ne 0 ]]; then
        emit "ERROR: step '$label' failed (exit $rc)"
        exit "$rc"
      fi
      ;;
    python)
      sec=$(python3 - "$label" "$@" <<'PY'
import subprocess, sys, time
label = sys.argv[1]
cmd = sys.argv[2:]
t0 = time.perf_counter()
r = subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
dt = time.perf_counter() - t0
if r.returncode != 0:
    sys.stderr.write(f"ERROR: step {label!r} failed (exit {r.returncode})\n")
    sys.exit(r.returncode)
print(f"{dt:.3f}")
PY
)
      ;;
  esac
  # Normalize to at least 3 decimal places when possible
  if [[ -z "${sec:-}" ]]; then
    sec="0"
  fi
  LAST_WALL_S="$sec"
  emit "  ${label}: ${sec}s wall"
}

cli_has_csv() {
  # Probe help / usage text after build; do not fail the suite if missing.
  local help
  help=$(cargo run -q -p nytprof-cli -- --help 2>&1 || true)
  # Also accept bare unknown-subcommand message that lists csv.
  if printf '%s' "$help" | grep -Eqi '(^|[[:space:]])csv([[:space:]]|$)'; then
    return 0
  fi
  # Fallback: try running csv with no args and look for usage that mentions csv
  help=$(cargo run -q -p nytprof-cli -- csv 2>&1 || true)
  if printf '%s' "$help" | grep -Eqi 'Usage:.*csv|nytprof-cli csv'; then
    return 0
  fi
  return 1
}

# --- fixtures ---
DEFAULT_FIXTURES=()
if [[ -f fixtures/v5/default-calls1/nytprof.out ]]; then
  DEFAULT_FIXTURES+=(fixtures/v5/default-calls1)
fi
if [[ -f fixtures/v5/default-calls2/nytprof.out ]]; then
  DEFAULT_FIXTURES+=(fixtures/v5/default-calls2)
fi

if [[ -n "${FIXTURES:-}" ]]; then
  # shellcheck disable=SC2206
  FIXTURE_LIST=($FIXTURES)
else
  FIXTURE_LIST=("${DEFAULT_FIXTURES[@]}")
fi

if [[ ${#FIXTURE_LIST[@]} -eq 0 ]]; then
  emit "ERROR: no fixtures found (expected fixtures/v5/default-calls1/nytprof.out)"
  exit 1
fi

emit "nytprof light_bench"
emit "root: $ROOT"
emit "timing: $TIME_MODE"
emit "commit: $(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
emit "rustc: $(rustc --version 2>/dev/null || echo unknown)"
emit "note: exploratory local timings only — NOT certification; no public claims"
emit ""

emit "build: cargo build -q -p nytprof-cli"
cargo build -q -p nytprof-cli
emit "build: ok"
emit ""

HAS_CSV=0
if cli_has_csv; then
  HAS_CSV=1
  emit "csv: available"
else
  emit "csv: skipped (subcommand not present)"
fi
emit ""

for fixdir in "${FIXTURE_LIST[@]}"; do
  out="$fixdir/nytprof.out"
  if [[ ! -f "$out" ]]; then
    emit "fixture $fixdir: SKIP (missing $out)"
    emit ""
    continue
  fi
  emit "fixture: $out"

  run_timed "dump" -- cargo run -q -p nytprof-cli -- dump "$out"
  run_timed "report" -- cargo run -q -p nytprof-cli -- report "$out"
  if [[ "$HAS_CSV" -eq 1 ]]; then
    run_timed "csv" -- cargo run -q -p nytprof-cli -- csv "$out"
  else
    emit "  csv: skipped"
  fi
  emit ""
done

emit "done (exit 0)"

if [[ -n "${OUT:-}" ]]; then
  {
    printf '%s\n' "${REPORT_LINES[@]}"
  } >"$OUT"
  # Also note path on stderr so stdout report stays clean for capture
  printf 'wrote %s\n' "$OUT" >&2
fi

exit 0
