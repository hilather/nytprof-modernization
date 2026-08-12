#!/usr/bin/env bash
# Light measurement harness for first-slice offline paths (P3/P4 engineering proxies).
#
# NOT a performance certification. Local exploratory timings only.
# See docs/BENCH_NOTES.md — no public performance claims.
#
# Usage:
#   bash tools/bench/light_bench.sh
#   OUT=/tmp/light_bench.txt bash tools/bench/light_bench.sh
#   RELEASE=1 RUNS=3 bash tools/bench/light_bench.sh
#
# Env:
#   OUT=path       Optional file to also write the same report (stdout always printed)
#   FIXTURES=…     Space-separated fixture dirs relative to repo root
#                  (default: fixtures/v5/default-calls1 fixtures/v5/default-calls2 if present)
#   STEPS=…        Comma-separated steps (default: dump,verify,report,csv,html)
#   RUNS=N         Timed repetitions per step (default: 1)
#   RELEASE=0|1    When 1, cargo build/run --release (default: 0 / debug)
#
# Exit 0 on success (missing optional fixtures/steps are skipped, not failures).

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

# --- config ---
RUNS="${RUNS:-1}"
if ! [[ "$RUNS" =~ ^[1-9][0-9]*$ ]]; then
  emit "ERROR: RUNS must be a positive integer (got: $RUNS)"
  exit 1
fi

RELEASE_FLAG=()
PROFILE_LABEL=debug
if [[ "${RELEASE:-0}" == "1" ]]; then
  RELEASE_FLAG=(--release)
  PROFILE_LABEL=release
fi

DEFAULT_STEPS="dump,verify,report,csv,html"
STEPS_CSV="${STEPS:-$DEFAULT_STEPS}"
# shellcheck disable=SC2206
IFS=',' read -r -a STEP_LIST <<<"$STEPS_CSV"
# trim whitespace on each step name
for i in "${!STEP_LIST[@]}"; do
  STEP_LIST[$i]="$(printf '%s' "${STEP_LIST[$i]}" | tr -d '[:space:]')"
done

step_wanted() {
  local want="$1"
  local s
  for s in "${STEP_LIST[@]}"; do
    if [[ "$s" == "$want" ]]; then
      return 0
    fi
  done
  return 1
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

cli_help() {
  cargo run -q -p nytprof-cli "${RELEASE_FLAG[@]}" -- --help 2>&1 || true
}

cli_has_subcommand() {
  # Probe help text after build; do not fail the suite if missing.
  local name="$1"
  local help
  help=$(cli_help)
  if printf '%s' "$help" | grep -Eqi "(^|[[:space:]])${name}([[:space:]]|$)"; then
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
emit "profile: $PROFILE_LABEL"
emit "runs: $RUNS"
emit "steps: $STEPS_CSV"
emit "commit: $(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
emit "rustc: $(rustc --version 2>/dev/null || echo unknown)"
emit "note: exploratory local timings only — NOT certification; no public claims"
emit "note: P3 proxies=dump,verify; P4 proxies=report,csv,html (see docs/BENCH_NOTES.md)"
emit ""

emit "build: cargo build -q -p nytprof-cli ${RELEASE_FLAG[*]:-}"
cargo build -q -p nytprof-cli "${RELEASE_FLAG[@]}"
emit "build: ok"
emit ""

# Capability probes (after build)
HAS_CSV=0
HAS_HTML=0
HAS_VERIFY=0
if step_wanted csv && cli_has_subcommand csv; then
  HAS_CSV=1
  emit "csv: available"
elif step_wanted csv; then
  emit "csv: skipped (subcommand not present)"
else
  emit "csv: not requested"
fi
if step_wanted html && cli_has_subcommand html; then
  HAS_HTML=1
  emit "html: available"
elif step_wanted html; then
  emit "html: skipped (subcommand not present)"
else
  emit "html: not requested"
fi
if step_wanted verify && cli_has_subcommand verify; then
  HAS_VERIFY=1
  emit "verify: available"
elif step_wanted verify; then
  emit "verify: skipped (subcommand not present)"
else
  emit "verify: not requested"
fi
emit ""

run_step_reps() {
  local step_name="$1"
  shift
  local r
  for ((r = 1; r <= RUNS; r++)); do
    if [[ "$RUNS" -gt 1 ]]; then
      run_timed "${step_name}#${r}" -- "$@"
    else
      run_timed "$step_name" -- "$@"
    fi
  done
}

for fixdir in "${FIXTURE_LIST[@]}"; do
  out="$fixdir/nytprof.out"
  if [[ ! -f "$out" ]]; then
    emit "fixture $fixdir: SKIP (missing $out)"
    emit ""
    continue
  fi
  emit "fixture: $out"

  if step_wanted dump; then
    run_step_reps "dump" cargo run -q -p nytprof-cli "${RELEASE_FLAG[@]}" -- dump "$out"
  fi

  if step_wanted verify; then
    if [[ "$HAS_VERIFY" -eq 1 ]]; then
      run_step_reps "verify" cargo run -q -p nytprof-cli "${RELEASE_FLAG[@]}" -- verify "$out"
    else
      emit "  verify: skipped"
    fi
  fi

  if step_wanted report; then
    run_step_reps "report" cargo run -q -p nytprof-cli "${RELEASE_FLAG[@]}" -- report "$out"
  fi

  if step_wanted csv; then
    if [[ "$HAS_CSV" -eq 1 ]]; then
      run_step_reps "csv" cargo run -q -p nytprof-cli "${RELEASE_FLAG[@]}" -- csv "$out"
    else
      emit "  csv: skipped"
    fi
  fi

  if step_wanted html; then
    if [[ "$HAS_HTML" -eq 1 ]]; then
      html_tmp=$(mktemp "${TMPDIR:-/tmp}/nytprof-light-bench-html.XXXXXX.html")
      # shellcheck disable=SC2064
      trap "rm -f '$html_tmp'" RETURN
      run_step_reps "html" cargo run -q -p nytprof-cli "${RELEASE_FLAG[@]}" -- html "$out" -o "$html_tmp"
      rm -f "$html_tmp"
      trap - RETURN
    else
      emit "  html: skipped"
    fi
  fi

  emit ""
done

emit "done (exit 0)"
emit "claim: none — not certification (docs/BENCH_NOTES.md)"

if [[ -n "${OUT:-}" ]]; then
  {
    printf '%s\n' "${REPORT_LINES[@]}"
  } >"$OUT"
  # Also note path on stderr so stdout report stays clean for capture
  printf 'wrote %s\n' "$OUT" >&2
fi

exit 0
