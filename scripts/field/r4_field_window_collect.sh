#!/usr/bin/env bash
# R4 field-window evidence collector (PR-E01).
#
# Builds a local evidence pack for opt-in format=v6 / convert / report tooling.
# Does NOT flip product defaults. Does NOT upload telemetry.
# collection_default remains v5 (asserted from capability when native present).
#
# Spec:  docs/schemas/r4-field-window-mvp-v0.md
# Guide: docs/R4_FIELD_WINDOW.md
# Board: R4-FIELD-WINDOW-PACK
#
# Usage (from repo root or any cwd):
#   ./scripts/field/r4_field_window_collect.sh --out /tmp/r4-pack
#   ./scripts/field/r4_field_window_collect.sh --out /tmp/r4-pack \
#       --profile /path/to/nytprof.out --site site-a --note "staging"
#
# Isolation: never puts crates/ on oracle PERL5LIB.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

OUT=""
SITE=""
NOTE=""
PROFILES=()
INCLUDE_DEFAULT_FIXTURE=1

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

usage() {
  cat <<'EOF'
Usage: r4_field_window_collect.sh --out DIR [options]

Required:
  --out DIR                 Evidence pack output directory (created)

Options:
  --profile PATH            Additional profile to exercise (repeatable)
  --site LABEL              Site id recorded in summary.json
  --note TEXT               Free-text note recorded in summary.json
  --no-default-fixture      Do not auto-include dual-sink default_calls1 pair
  -h, --help                Show this help

Never flips product defaults. Pack residual flags all false for R4/R3/COL-008.
Default lab fixtures: fixtures/e4/dual-sink/default_calls1_{v5,v6}.nytprof
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out)
      [[ $# -ge 2 ]] || fail "--out requires a path"
      OUT="$2"
      shift 2
      ;;
    --profile)
      [[ $# -ge 2 ]] || fail "--profile requires a path"
      PROFILES+=("$2")
      shift 2
      ;;
    --site)
      [[ $# -ge 2 ]] || fail "--site requires a label"
      SITE="$2"
      shift 2
      ;;
    --note)
      [[ $# -ge 2 ]] || fail "--note requires text"
      NOTE="$2"
      shift 2
      ;;
    --no-default-fixture)
      INCLUDE_DEFAULT_FIXTURE=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "unknown argument: $1 (try --help)"
      ;;
  esac
done

[[ -n "$OUT" ]] || fail "--out DIR is required"
[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml (not repo root?)"

if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ (got: $PERL5LIB)"
fi

DEFAULT_V5="fixtures/e4/dual-sink/default_calls1_v5.nytprof"
DEFAULT_V6="fixtures/e4/dual-sink/default_calls1_v6.nytprof"

if [[ "$INCLUDE_DEFAULT_FIXTURE" -eq 1 ]]; then
  [[ -f "$ROOT/$DEFAULT_V5" ]] || fail "missing $DEFAULT_V5"
  [[ -f "$ROOT/$DEFAULT_V6" ]] || fail "missing $DEFAULT_V6"
  for def in "$DEFAULT_V5" "$DEFAULT_V6"; do
    have=0
    for p in "${PROFILES[@]+"${PROFILES[@]}"}"; do
      if [[ "$p" == "$def" || "$p" == "$ROOT/$def" ]]; then
        have=1
        break
      fi
    done
    if [[ "$have" -eq 0 ]]; then
      PROFILES+=("$def")
    fi
  done
fi

[[ ${#PROFILES[@]} -gt 0 ]] || fail "no profiles to exercise (use --profile or default fixture)"

mkdir -p "$OUT"/{env,capability,runs,profiles,artifacts}
OUT="$(cd "$OUT" && pwd)"

# ---------------------------------------------------------------------------
# Native discovery — prefer a CLI that supports convert (R2-stable / R4 pack).
# Stale prefix/ installs may lack convert; probe before accepting a candidate.
# ---------------------------------------------------------------------------
cli_supports_convert() {
  local bin="$1"
  # R2-stable capability JSON has convert:true; older CLIs lack the key / subcommand.
  if "$bin" capability --json 2>/dev/null | python3 -c 'import json,sys
try:
  d=json.load(sys.stdin)
  sys.exit(0 if d.get("convert") is True else 1)
except Exception:
  sys.exit(1)' 2>/dev/null; then
    return 0
  fi
  # Fallback: convert subcommand recognized (not "unknown subcommand")
  local help
  help="$("$bin" convert --help 2>&1 || true)"
  if printf '%s' "$help" | grep -Eqi 'Usage:.*convert|--to=v5'; then
    return 0
  fi
  return 1
}

find_cli() {
  if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
    if cli_supports_convert "${NYTPROF_NATIVE_CLI}"; then
      echo "path:${NYTPROF_NATIVE_CLI}"
      return 0
    fi
    # Explicit override still wins even without convert (honest pack; smoke will fail closed)
    echo "path:${NYTPROF_NATIVE_CLI}"
    return 0
  fi
  # Prefer workspace target (current tree) over prefix (may be stale install)
  for p in \
    target/release/nytprof-dump \
    target/debug/nytprof-dump \
    prefix/bin/nytprof-cli \
    prefix/bin/nytprof-dump
  do
    if [[ -x "$ROOT/$p" ]] && cli_supports_convert "$ROOT/$p"; then
      echo "path:$ROOT/$p"
      return 0
    fi
  done
  # Any executable as last resort (may lack convert — pack records failures honestly)
  for p in \
    target/release/nytprof-dump \
    target/debug/nytprof-dump \
    prefix/bin/nytprof-cli \
    prefix/bin/nytprof-dump
  do
    if [[ -x "$ROOT/$p" ]]; then
      echo "path:$ROOT/$p"
      return 0
    fi
  done
  if command -v cargo >/dev/null 2>&1; then
    echo "cargo"
    return 0
  fi
  return 1
}

NATIVE_DISCOVERABLE=0
NATIVE_CLI_SPEC=""
CLI_ARR=()
if CLI_SPEC="$(find_cli)"; then
  NATIVE_DISCOVERABLE=1
  NATIVE_CLI_SPEC="$CLI_SPEC"
  ok "native discoverable ($CLI_SPEC)"
  if [[ "$CLI_SPEC" == "cargo" ]]; then
    cargo build -q -p nytprof-cli
    ok "cargo build -p nytprof-cli"
    if [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
      NATIVE_CLI_SPEC="path:$ROOT/target/debug/nytprof-dump"
    elif [[ -x "$ROOT/target/release/nytprof-dump" ]]; then
      NATIVE_CLI_SPEC="path:$ROOT/target/release/nytprof-dump"
    fi
  fi
  if [[ "$NATIVE_CLI_SPEC" == path:* ]]; then
    CLI_ARR=("${NATIVE_CLI_SPEC#path:}")
  else
    CLI_ARR=(cargo run -q -p nytprof-cli --)
  fi
else
  ok "native not discoverable (format pack requires native for full evidence)"
fi

# ---------------------------------------------------------------------------
# Provenance
# ---------------------------------------------------------------------------
{
  echo "schema: r4-field-window-mvp-v0"
  echo "generated_at_utc: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "repo_root: $ROOT"
  echo "out: $OUT"
  echo "site: ${SITE:-}"
  echo "note: ${NOTE:-}"
  echo "no_default_flip: true"
  echo "collection_default: v5  # product default; pack never flips"
  echo "---"
  date -u
  uname -a || true
  echo "---"
  if command -v git >/dev/null 2>&1 && [[ -d "$ROOT/.git" || -f "$ROOT/.git" ]]; then
    git -C "$ROOT" rev-parse HEAD 2>/dev/null || true
    git -C "$ROOT" status -sb 2>/dev/null || true
  else
    echo "git: unavailable"
  fi
  echo "---"
  perl -v 2>&1 | head -n 8 || true
  echo "---"
  echo "native_discoverable: $NATIVE_DISCOVERABLE"
  echo "native_cli_spec: ${NATIVE_CLI_SPEC:-}"
  echo "NYTPROF_ENGINE=${NYTPROF_ENGINE:-<unset>}"
  echo "NYTPROF_NATIVE_CLI=${NYTPROF_NATIVE_CLI:-<unset>}"
  echo "PERL5LIB=${PERL5LIB:-<unset>}"
  echo "---"
  echo "profiles:"
  for p in "${PROFILES[@]}"; do
    echo "  - $p"
  done
} >"$OUT/env/provenance.txt"

cat >"$OUT/profiles/README.md" <<EOF
# Profiles exercised

This pack records **paths** only by default (does not copy input profile blobs).
Convert outputs land under \`artifacts/\` (pack-local).

| Path | Notes |
|------|-------|
$(for p in "${PROFILES[@]}"; do echo "| \`$p\` | |"; done)

Redact proprietary paths before sharing packs outside the trust boundary.
See [docs/schemas/r4-field-window-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/r4-field-window-mvp-v0.md).
EOF

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
safe_label() {
  local p="$1"
  local b
  b="$(basename "$p")"
  # Prefer parent dir for nytprof.out; else file basename (dual-sink names)
  if [[ "$b" == "nytprof.out" ]]; then
    b="$(basename "$(dirname "$p")")"
  fi
  # strip extension for dual-sink *.nytprof
  b="${b%.nytprof}"
  b="$(printf '%s' "$b" | tr -c 'A-Za-z0-9._-' '_')"
  printf '%s' "$b"
}

rel_profile() {
  local p="$1"
  if [[ "$p" == /* ]]; then
    case "$p" in
      "$ROOT"/*) printf '%s' "${p#"$ROOT"/}" ;;
      *) printf '%s' "$p" ;;
    esac
  else
    printf '%s' "$p"
  fi
}

resolve_profile() {
  local p="$1"
  if [[ -f "$p" ]]; then
    printf '%s' "$p"
    return 0
  fi
  if [[ -f "$ROOT/$p" ]]; then
    printf '%s' "$ROOT/$p"
    return 0
  fi
  return 1
}

file_bytes() {
  local f="$1"
  if [[ -f "$f" ]]; then
    wc -c <"$f" | tr -d ' '
  else
    echo ""
  fi
}

detect_family() {
  # Best-effort magic via od (avoid bash null-byte warnings from binary head)
  local f="$1"
  local magic
  magic="$(od -An -N8 -tx1 "$f" 2>/dev/null | tr -d ' \n' || true)"
  # NYTPROF6 = 4e 59 54 50 52 4f 46 36
  if [[ "$magic" == "4e595450524f4636" ]]; then
    echo v6
    return 0
  fi
  # NYTProf  = 4e 59 54 50 72 6f 66 20  (v5 text header)
  if [[ "$magic" == 4e595450726f66* ]]; then
    echo v5
    return 0
  fi
  # dual-sink naming fallback
  case "$f" in
    *_v6.nytprof|*/default_calls1_v6.nytprof) echo v6; return 0 ;;
    *_v5.nytprof|*/default_calls1_v5.nytprof) echo v5; return 0 ;;
  esac
  echo unknown
}

extract_returns() {
  local outf="$1" sub="$2"
  grep -E "main::${sub}" "$outf" 2>/dev/null \
    | grep -Eo 'returns[= ]+[0-9]+' \
    | head -n1 \
    | grep -Eo '[0-9]+' \
    || true
}

RUN_ROWS=()

record_run() {
  local id="$1"
  local action="$2"
  local family="$3"
  local profile_rel="$4"
  local output_rel="$5"
  local rc="$6"
  local outf="$7"
  local bytes_in="${8:-}"
  local bytes_out="${9:-}"

  local leaf mid
  leaf="$(extract_returns "$outf" leaf)"
  mid="$(extract_returns "$outf" mid)"

  cat >"$OUT/runs/${id}.meta.json" <<EOF
{
  "id": $(python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$id"),
  "action": $(python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$action"),
  "format_family": $(python3 -c 'import json,sys; print(json.dumps(sys.argv[1]) if sys.argv[1] else "null")' "$family"),
  "profile": $(python3 -c 'import json,sys; print(json.dumps(sys.argv[1]) if sys.argv[1] else "null")' "$profile_rel"),
  "output": $(python3 -c 'import json,sys; print(json.dumps(sys.argv[1]) if sys.argv[1] else "null")' "$output_rel"),
  "rc": $rc,
  "leaf_returns": $([[ -n "$leaf" ]] && echo "$leaf" || echo null),
  "mid_returns": $([[ -n "$mid" ]] && echo "$mid" || echo null),
  "bytes_in": $([[ -n "$bytes_in" ]] && echo "$bytes_in" || echo null),
  "bytes_out": $([[ -n "$bytes_out" ]] && echo "$bytes_out" || echo null)
}
EOF

  RUN_ROWS+=("$id|$action|$family|$profile_rel|$output_rel|$rc|${leaf:-}|${mid:-}|${bytes_in:-}|${bytes_out:-}")
  ok "run $id rc=$rc leaf=${leaf:-?} mid=${mid:-?} family=${family:-?}"
}

run_cli() {
  # run_cli id action family profile_abs profile_rel [extra args as global RUN_ARGS]
  local id="$1"
  local action="$2"
  local family="$3"
  local profile_abs="$4"
  local profile_rel="$5"
  shift 5
  local -a extra=("$@")

  local outf="$OUT/runs/${id}.stdout.txt"
  local errf="$OUT/runs/${id}.stderr.txt"
  local rcf="$OUT/runs/${id}.rc"
  local rc=0
  local bin=""

  bin="$(file_bytes "$profile_abs")"

  set +e
  "${CLI_ARR[@]}" "$action" "${extra[@]}" "$profile_abs" \
    >"$outf" 2>"$errf"
  rc=$?
  set -e
  printf '%s\n' "$rc" >"$rcf"
  record_run "$id" "$action" "$family" "$profile_rel" "" "$rc" "$outf" "$bin" ""
}

run_convert() {
  local id="$1"
  local to="$2"
  local profile_abs="$3"
  local profile_rel="$4"
  local out_name="$5"

  local out_abs="$OUT/artifacts/$out_name"
  local out_rel="artifacts/$out_name"
  local outf="$OUT/runs/${id}.stdout.txt"
  local errf="$OUT/runs/${id}.stderr.txt"
  local rcf="$OUT/runs/${id}.rc"
  local rc=0
  local bin_in bin_out

  bin_in="$(file_bytes "$profile_abs")"

  set +e
  "${CLI_ARR[@]}" convert --to="$to" "$profile_abs" -o "$out_abs" \
    >"$outf" 2>"$errf"
  rc=$?
  set -e
  printf '%s\n' "$rc" >"$rcf"
  bin_out=""
  if [[ -f "$out_abs" ]]; then
    bin_out="$(file_bytes "$out_abs")"
  fi
  record_run "$id" "convert" "convert" "$profile_rel" "$out_rel" "$rc" "$outf" "$bin_in" "$bin_out"
  # return path for chaining via global
  CONVERT_OUT_ABS="$out_abs"
  CONVERT_OUT_REL="$out_rel"
}

# ---------------------------------------------------------------------------
# Capability (when native present)
# ---------------------------------------------------------------------------
COLLECTION_DEFAULT="v5"
if [[ "$NATIVE_DISCOVERABLE" -eq 1 ]]; then
  set +e
  "${CLI_ARR[@]}" capability --json \
    >"$OUT/capability/capability.json" \
    2>"$OUT/capability/capability.stderr.txt"
  cap_rc=$?
  set -e
  printf '%s\n' "$cap_rc" >"$OUT/capability/capability.rc"
  if [[ -f "$OUT/capability/capability.json" ]]; then
    cp "$OUT/capability/capability.json" "$OUT/capability/capability.stdout.txt" 2>/dev/null || true
    # parse collection_default if present
    cd_val="$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(d.get("collection_default") or "")' \
      "$OUT/capability/capability.json" 2>/dev/null || true)"
    if [[ -n "$cd_val" ]]; then
      COLLECTION_DEFAULT="$cd_val"
    fi
  fi
  ok "capability --json rc=$cap_rc collection_default=$COLLECTION_DEFAULT"
else
  echo "skip: native not discoverable" >"$OUT/capability/capability.stdout.txt"
  : >"$OUT/capability/capability.stderr.txt"
  echo "1" >"$OUT/capability/capability.rc"
  echo '{"ok":false,"skipped":true,"reason":"native_not_discoverable","collection_default":"v5"}' \
    >"$OUT/capability/capability.json"
  ok "capability skipped (no native)"
fi

# ---------------------------------------------------------------------------
# Per-profile runs
# ---------------------------------------------------------------------------
if [[ "$NATIVE_DISCOVERABLE" -eq 1 ]]; then
  for pref in "${PROFILES[@]}"; do
    if ! pabs="$(resolve_profile "$pref")"; then
      fail "profile not found: $pref"
    fi
    prel="$(rel_profile "$pref")"
    if [[ "$prel" == "$pref" && "$pabs" == "$ROOT/"* ]]; then
      prel="$(rel_profile "$pabs")"
    fi
    label="$(safe_label "$prel")"
    family="$(detect_family "$pabs")"

    if [[ "$family" == "v5" || "$family" == "unknown" ]]; then
      run_cli "v5_report_${label}" report v5 "$pabs" "$prel"
      # convert v5 → v6 when family is v5 (or unknown try)
      if [[ "$family" == "v5" ]]; then
        run_convert "convert_to_v6_${label}" v6 "$pabs" "$prel" "convert_to_v6_${label}.nytprof"
        if [[ -f "${CONVERT_OUT_ABS:-}" ]]; then
          run_cli "report_after_convert_to_v6_${label}" report v6 \
            "$CONVERT_OUT_ABS" "$CONVERT_OUT_REL"
        fi
      fi
    fi

    if [[ "$family" == "v6" ]]; then
      run_cli "v6_report_${label}" report v6 "$pabs" "$prel"
      run_cli "v6_verify_${label}" verify v6 "$pabs" "$prel"
      run_convert "convert_to_v5_${label}" v5 "$pabs" "$prel" "convert_to_v5_${label}.nytprof"
      if [[ -f "${CONVERT_OUT_ABS:-}" ]]; then
        run_cli "report_after_convert_to_v5_${label}" report v5 \
          "$CONVERT_OUT_ABS" "$CONVERT_OUT_REL"
      fi
    fi
  done
else
  ok "skipping format runs (native not discoverable)"
fi

# ---------------------------------------------------------------------------
# summary.json via Python
# ---------------------------------------------------------------------------
export R4_OUT="$OUT"
export R4_SITE="$SITE"
export R4_NOTE="$NOTE"
export R4_NATIVE_DISCOVERABLE="$NATIVE_DISCOVERABLE"
export R4_NATIVE_CLI_SPEC="$NATIVE_CLI_SPEC"
export R4_ROOT="$ROOT"
export R4_COLLECTION_DEFAULT="$COLLECTION_DEFAULT"

printf '%s\n' "${PROFILES[@]}" >"$OUT/.profiles.txt"
printf '%s\n' "${RUN_ROWS[@]+"${RUN_ROWS[@]}"}" >"$OUT/.runs.txt"

# size samples for dual-sink defaults when present
V5_BYTES=""
V6_BYTES=""
if [[ -f "$ROOT/$DEFAULT_V5" ]]; then
  V5_BYTES="$(file_bytes "$ROOT/$DEFAULT_V5")"
fi
if [[ -f "$ROOT/$DEFAULT_V6" ]]; then
  V6_BYTES="$(file_bytes "$ROOT/$DEFAULT_V6")"
fi
export R4_V5_BYTES="$V5_BYTES"
export R4_V6_BYTES="$V6_BYTES"

python3 - <<'PY'
import json, os, pathlib, datetime, subprocess

out = pathlib.Path(os.environ["R4_OUT"])
root = pathlib.Path(os.environ["R4_ROOT"])
site = os.environ.get("R4_SITE") or None
note = os.environ.get("R4_NOTE") or None
native_disc = os.environ.get("R4_NATIVE_DISCOVERABLE") == "1"
native_spec = os.environ.get("R4_NATIVE_CLI_SPEC") or None
collection_default = os.environ.get("R4_COLLECTION_DEFAULT") or "v5"

profiles = []
for line in (out / ".profiles.txt").read_text().splitlines():
    line = line.strip()
    if not line:
        continue
    p = line
    if line.startswith(str(root) + "/"):
        p = line[len(str(root)) + 1 :]
    profiles.append(p)

runs = []
for line in (out / ".runs.txt").read_text().splitlines():
    if not line.strip():
        continue
    parts = line.split("|")
    # id|action|family|profile|output|rc|leaf|mid|bytes_in|bytes_out
    while len(parts) < 10:
        parts.append("")
    rid, action, family, profile, output, rc, leaf, mid, bin_in, bin_out = parts[:10]

    def as_int(x):
        x = (x or "").strip()
        if x.isdigit() or (x.startswith("-") and x[1:].isdigit()):
            return int(x)
        return None

    runs.append({
        "id": rid,
        "action": action,
        "format_family": family or None,
        "profile": profile or None,
        "output": output or None,
        "rc": as_int(rc),
        "leaf_returns": as_int(leaf),
        "mid_returns": as_int(mid),
        "bytes_in": as_int(bin_in),
        "bytes_out": as_int(bin_out),
    })

git_commit = None
try:
    git_commit = subprocess.check_output(
        ["git", "-C", str(root), "rev-parse", "HEAD"],
        stderr=subprocess.DEVNULL,
        text=True,
    ).strip()
except Exception:
    git_commit = None

def find_run(suffix_or_id):
    for r in runs:
        if r["id"] == suffix_or_id or r["id"].endswith(suffix_or_id):
            return r
    return None

fixture = None
v5r = find_run("v5_report_default_calls1_v5")
v6r = find_run("v6_report_default_calls1_v6")
c2v6 = find_run("convert_to_v6_default_calls1_v5")
c2v5 = find_run("convert_to_v5_default_calls1_v6")
if v5r or v6r or c2v6 or c2v5:
    fixture = {
        "leaf_returns": None,
        "mid_returns": None,
        "v5_report_rc": v5r.get("rc") if v5r else None,
        "v6_report_rc": v6r.get("rc") if v6r else None,
        "convert_to_v6_rc": c2v6.get("rc") if c2v6 else None,
        "convert_to_v5_rc": c2v5.get("rc") if c2v5 else None,
    }
    for src in (v6r, v5r):
        if src and src.get("leaf_returns") is not None:
            fixture["leaf_returns"] = src.get("leaf_returns")
            fixture["mid_returns"] = src.get("mid_returns")
            break

sizes = {}
v5b = (os.environ.get("R4_V5_BYTES") or "").strip()
v6b = (os.environ.get("R4_V6_BYTES") or "").strip()
if v5b.isdigit():
    sizes["default_calls1_v5_bytes"] = int(v5b)
if v6b.isdigit():
    sizes["default_calls1_v6_bytes"] = int(v6b)
if c2v6 and c2v6.get("bytes_out") is not None:
    sizes["convert_to_v6_bytes"] = c2v6["bytes_out"]
if c2v5 and c2v5.get("bytes_out") is not None:
    sizes["convert_to_v5_bytes"] = c2v5["bytes_out"]
if not sizes:
    sizes = None

summary = {
    "schema": "r4-field-window-mvp-v0",
    "generated_at_utc": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "git_commit": git_commit,
    "site": site,
    "note": note,
    "no_default_flip": True,
    "collection_default": collection_default,
    "native_discoverable": native_disc,
    "native_cli_spec": native_spec,
    "profiles": profiles,
    "runs": runs,
    "sizes": sizes,
    "fixture_default_calls1": fixture,
    "residuals": {
        "r4_format_default_flip": False,
        "r3_product_default_flip": False,
        "col008_batched_rust_writer": False,
        "lossy_convert": False,
        "public_perf_certification": False,
    },
}

(out / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
(out / ".profiles.txt").unlink(missing_ok=True)
(out / ".runs.txt").unlink(missing_ok=True)
print("wrote", out / "summary.json")
PY

# ---------------------------------------------------------------------------
# MANIFEST.md
# ---------------------------------------------------------------------------
cat >"$OUT/MANIFEST.md" <<EOF
# R4 field-window evidence pack

**Schema:** \`r4-field-window-mvp-v0\`  
**Generated:** see \`summary.json\` / \`env/provenance.txt\`  
**Site:** ${SITE:-*(none)*}  
**Binding:** \`no_default_flip: true\` — this pack does **not** authorize product default changes.  
**Collection default:** \`${COLLECTION_DEFAULT}\` (must remain **v5** under R2-stable / this window).

## Contents

| Path | Role |
|------|------|
| \`summary.json\` | Machine-readable roll-up |
| \`env/provenance.txt\` | Host / tool provenance |
| \`capability/\` | Native \`capability --json\` (or skip) |
| \`runs/\` | Per-tool stdout/stderr/rc + meta |
| \`artifacts/\` | Convert outputs (pack-local) |
| \`profiles/README.md\` | Profile path list (no input blobs) |

## How produced

\`\`\`sh
./scripts/field/r4_field_window_collect.sh --out <this-dir> ...
\`\`\`

Guide: https://github.com/hilather/nytprof-modernization/blob/main/docs/R4_FIELD_WINDOW.md  
Report template: https://github.com/hilather/nytprof-modernization/blob/main/docs/templates/R4_FIELD_WINDOW_REPORT.md  
Schema: https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/r4-field-window-mvp-v0.md

## Explicit non-claims

- Not charter **R4** product format default flip (ADR-Q025 / REL-008 still required).
- Not **R3** engine default flip, **COL-008** baseline, lossy convert, or public perf certification.
- Product \`collection_default\` remains **v5** for this pack.
EOF

# Optional checksums
if command -v sha256sum >/dev/null 2>&1; then
  (cd "$OUT" && find . -type f ! -name SHA256SUMS ! -name '.profiles.txt' ! -name '.runs.txt' -print0 \
    | sort -z | xargs -0 sha256sum >SHA256SUMS) || true
elif command -v shasum >/dev/null 2>&1; then
  (cd "$OUT" && find . -type f ! -name SHA256SUMS -print0 \
    | sort -z | xargs -0 shasum -a 256 >SHA256SUMS) || true
fi

ok "R4 field-window pack written to $OUT"
ok "no_default_flip=true collection_default=$COLLECTION_DEFAULT (product defaults unchanged)"
log "Next: fill docs/templates/R4_FIELD_WINDOW_REPORT.md from this pack"
