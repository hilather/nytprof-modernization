#!/usr/bin/env bash
# Light measurement harness for offline paths + R2 P1/P2 engineering proxies.
#
# NOT a performance certification. Local exploratory timings only.
# See docs/BENCH_NOTES.md — no public performance claims until R2-stable gates green.
#
# Usage:
#   bash tools/bench/light_bench.sh
#   OUT=/tmp/light_bench.txt bash tools/bench/light_bench.sh
#   RELEASE=1 RUNS=3 bash tools/bench/light_bench.sh
#   STEPS=size,collector_micro,dump,report bash tools/bench/light_bench.sh
#
# Env:
#   OUT=path         Optional file to also write the same report (stdout always printed)
#   FIXTURES=…       Space-separated fixture dirs relative to repo root
#                    (default: fixtures/v5/default-calls1 fixtures/v5/default-calls2 if present)
#   STEPS=…          Comma-separated steps
#                    (default: size,dump,verify,report,csv,html,collector_micro)
#   RUNS=N           Timed repetitions per timed step (default: 1)
#   RELEASE=0|1      When 1, cargo build/run --release (default: 0 / unset)
#   SKIP_COLLECTOR=1 Skip collector_micro even if listed
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

DEFAULT_STEPS="size,dump,verify,report,csv,html,collector_micro"
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

need_cli=0
for s in dump verify report csv html; do
  if step_wanted "$s"; then
    need_cli=1
    break
  fi
done

# --- timing: prefer /usr/bin/time -f, else bash TIMEFORMAT, else python3 ---
TIME_MODE=
if [[ -x /usr/bin/time ]] && /usr/bin/time -f '%e' true >/dev/null 2>&1; then
  TIME_MODE=gnu
elif [[ -n "${BASH_VERSION:-}" ]]; then
  TIME_MODE=bash
else
  TIME_MODE=python
fi

# Run command; print wall seconds. Command stdout/stderr discarded on success path.
# Usage: run_timed <label> -- <cmd...>
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

if [[ ${#FIXTURE_LIST[@]} -eq 0 ]] && ! step_wanted size && ! step_wanted collector_micro; then
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
emit "note: P1 proxies=collector_micro; P2 proxies=size; P3=dump,verify; P4=report,csv,html"
emit "note: public-claim-ready only if R2-stable BENCH gates green (docs/BENCH_NOTES.md)"
emit ""

if [[ "$need_cli" -eq 1 ]]; then
  emit "build: cargo build -q -p nytprof-cli ${RELEASE_FLAG[*]:-}"
  cargo build -q -p nytprof-cli "${RELEASE_FLAG[@]}"
  emit "build: ok"
  emit ""
fi

# Capability probes (after build when needed)
HAS_CSV=0
HAS_HTML=0
HAS_VERIFY=0
if [[ "$need_cli" -eq 1 ]]; then
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
fi

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

# --- P2: size inventory ---
emit_file_size() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    emit "  size $path: SKIP (missing)"
    return 0
  fi
  local bytes
  if bytes=$(stat -c '%s' "$path" 2>/dev/null); then
    :
  elif bytes=$(stat -f '%z' "$path" 2>/dev/null); then
    :
  else
    bytes=$(wc -c <"$path" | tr -d ' ')
  fi
  emit "  size $path: ${bytes} bytes"
}

if step_wanted size; then
  emit "step: size (P2 storage proxy — inventory only; not a size SLO)"
  if [[ ${#FIXTURE_LIST[@]} -gt 0 ]]; then
    for fixdir in "${FIXTURE_LIST[@]}"; do
      emit_file_size "$fixdir/nytprof.out"
    done
  else
    emit "  (no default v5 fixtures in FIXTURES list)"
  fi
  # Committed product E3-EVENT C profiles (codec/packing matrix)
  shopt -s nullglob
  v6_files=(fixtures/v6/from-c/*.nytprof)
  shopt -u nullglob
  if [[ ${#v6_files[@]} -gt 0 ]]; then
    for f in "${v6_files[@]}"; do
      emit_file_size "$f"
    done
  else
    emit "  fixtures/v6/from-c/*.nytprof: none present"
  fi
  emit ""
fi

# --- P1: collector microbench via unit test suite ---
if step_wanted collector_micro; then
  emit "step: collector_micro (P1 engineering proxy — not end-to-end collection cert)"
  if [[ "${SKIP_COLLECTOR:-0}" == "1" ]]; then
    emit "  collector_micro: skipped (SKIP_COLLECTOR=1)"
  elif [[ ! -f collector/Makefile ]]; then
    emit "  collector_micro: skipped (no collector/Makefile)"
  elif ! command -v cc >/dev/null 2>&1 && ! command -v gcc >/dev/null 2>&1 && ! command -v clang >/dev/null 2>&1; then
    emit "  collector_micro: skipped (no C compiler)"
  else
    # Time the collector unit suite; surface microbench NOTE lines if present.
    local_log=$(mktemp)
    set +e
    if [[ "$TIME_MODE" == "gnu" ]]; then
      tmp=$(mktemp)
      /usr/bin/time -f '%e' -o "$tmp" -- make -C collector test >"$local_log" 2>&1
      rc=$?
      sec=$(tr -d ' \n' <"$tmp" || true)
      rm -f "$tmp"
    else
      t0=$(date +%s.%N 2>/dev/null || python3 -c 'import time; print(time.perf_counter())')
      make -C collector test >"$local_log" 2>&1
      rc=$?
      t1=$(date +%s.%N 2>/dev/null || python3 -c 'import time; print(time.perf_counter())')
      sec=$(python3 -c "print(f'{float('$t1')-float('$t0'):.3f}')" 2>/dev/null || echo "0")
    fi
    set -e
    if [[ $rc -ne 0 ]]; then
      emit "ERROR: collector_micro failed (make -C collector test exit $rc)"
      # show last lines for diagnosis
      tail -n 40 "$local_log" | while IFS= read -r line; do emit "  | $line"; done
      rm -f "$local_log"
      exit "$rc"
    fi
    emit "  collector_micro suite: ${sec:-0}s wall"
    # Extract engineering NOTE lines (microbench)
    if grep -E 'NOTE: light microbench|engineering only; not BENCH' "$local_log" >/dev/null 2>&1; then
      grep -E 'NOTE: light microbench|engineering only; not BENCH' "$local_log" | while IFS= read -r line; do
        emit "  $line"
      done
    else
      emit "  (no microbench NOTE line captured; suite still green)"
    fi
    rm -f "$local_log"
  fi
  emit ""
fi

# --- offline CLI steps (P3/P4 proxies) ---
for fixdir in "${FIXTURE_LIST[@]}"; do
  out="$fixdir/nytprof.out"
  if [[ ! -f "$out" ]]; then
    emit "fixture $fixdir: SKIP (missing $out)"
    emit ""
    continue
  fi

  any_cli=0
  for s in dump verify report csv html; do
    if step_wanted "$s"; then
      any_cli=1
      break
    fi
  done
  if [[ "$any_cli" -eq 0 ]]; then
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
emit "claim: none — not certification; public claims only after R2-stable BENCH gates (docs/BENCH_NOTES.md)"

if [[ -n "${OUT:-}" ]]; then
  {
    printf '%s\n' "${REPORT_LINES[@]}"
  } >"$OUT"
  printf 'wrote %s\n' "$OUT" >&2
fi

exit 0
