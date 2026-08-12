#!/usr/bin/env bash
# R3 field-window evidence collector (PR-D01).
#
# Builds a local evidence pack for opt-in engine=auto / native reporting.
# Does NOT flip product defaults. Does NOT upload telemetry.
#
# Spec:  docs/schemas/r3-field-window-mvp-v0.md
# Guide: docs/R3_FIELD_WINDOW.md
# Board: R3-FIELD-WINDOW-PACK
#
# Usage (from repo root or any cwd):
#   ./scripts/field/r3_field_window_collect.sh --out /tmp/r3-pack
#   ./scripts/field/r3_field_window_collect.sh --out /tmp/r3-pack \
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
SKIP_FORCE_NO_NATIVE=0

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

usage() {
  cat <<'EOF'
Usage: r3_field_window_collect.sh --out DIR [options]

Required:
  --out DIR                 Evidence pack output directory (created)

Options:
  --profile PATH            Additional profile to exercise (repeatable)
  --site LABEL              Site id recorded in summary.json
  --note TEXT               Free-text note recorded in summary.json
  --no-default-fixture      Do not auto-include fixtures/v5/default-calls1
  --skip-force-no-native    Skip NYTPROF_FORCE_NO_NATIVE fallback exercise
  -h, --help                Show this help

Never flips product defaults. Pack residual flags all false for R3/R4/COL-007.
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
    --skip-force-no-native)
      SKIP_FORCE_NO_NATIVE=1
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
[[ -f "$ROOT/perl/bin/nytprof-engine" ]] || fail "missing perl/bin/nytprof-engine"
[[ -f "$ROOT/perl/lib/Devel/NYTProf/EngineDispatch.pm" ]] || fail "missing EngineDispatch.pm"

if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ (got: $PERL5LIB)"
fi

DEFAULT_FIXTURE="fixtures/v5/default-calls1/nytprof.out"
if [[ "$INCLUDE_DEFAULT_FIXTURE" -eq 1 ]]; then
  [[ -f "$ROOT/$DEFAULT_FIXTURE" ]] || fail "missing $DEFAULT_FIXTURE"
  # Prepend default fixture if not already listed
  have_default=0
  for p in "${PROFILES[@]+"${PROFILES[@]}"}"; do
    if [[ "$p" == "$DEFAULT_FIXTURE" || "$p" == "$ROOT/$DEFAULT_FIXTURE" ]]; then
      have_default=1
      break
    fi
  done
  if [[ "$have_default" -eq 0 ]]; then
    PROFILES=("$DEFAULT_FIXTURE" "${PROFILES[@]+"${PROFILES[@]}"}")
  fi
fi

[[ ${#PROFILES[@]} -gt 0 ]] || fail "no profiles to exercise (use --profile or default fixture)"

mkdir -p "$OUT"/{env,capability,runs,profiles}
OUT="$(cd "$OUT" && pwd)"

# ---------------------------------------------------------------------------
# Native discovery (same spirit as packaging smokes)
# ---------------------------------------------------------------------------
find_cli() {
  if [[ -n "${NYTPROF_NATIVE_CLI:-}" && -x "${NYTPROF_NATIVE_CLI}" ]]; then
    echo "path:${NYTPROF_NATIVE_CLI}"
    return 0
  fi
  for p in \
    prefix/bin/nytprof-cli \
    prefix/bin/nytprof-dump \
    target/release/nytprof-dump \
    target/debug/nytprof-dump
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
else
  ok "native not discoverable (auto will fall back to legacy when exercised)"
fi

ENGINE=(perl -I"$ROOT/perl/lib" "$ROOT/perl/bin/nytprof-engine")

# ---------------------------------------------------------------------------
# Provenance
# ---------------------------------------------------------------------------
{
  echo "schema: r3-field-window-mvp-v0"
  echo "generated_at_utc: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "repo_root: $ROOT"
  echo "out: $OUT"
  echo "site: ${SITE:-}"
  echo "note: ${NOTE:-}"
  echo "no_default_flip: true"
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

This pack records **paths** only by default (does not copy profile blobs).

| Path | Notes |
|------|-------|
$(for p in "${PROFILES[@]}"; do echo "| \`$p\` | |"; done)

Redact proprietary paths before sharing packs outside the trust boundary.
See [docs/schemas/r3-field-window-mvp-v0.md](https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/r3-field-window-mvp-v0.md).
EOF

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
safe_label() {
  local p="$1"
  local b
  b="$(basename "$(dirname "$p")")"
  if [[ "$b" == "." || "$b" == "/" ]]; then
    b="$(basename "$p")"
  fi
  # Prefer fixture dir name (default-calls1) over nytprof.out
  if [[ "$(basename "$p")" == "nytprof.out" ]]; then
    :
  else
    b="$(basename "$p")"
  fi
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

extract_returns() {
  # $1=stdout file $2=sub name → integer or empty
  local outf="$1" sub="$2"
  grep -E "main::${sub}" "$outf" 2>/dev/null \
    | grep -Eo 'returns[= ]+[0-9]+' \
    | head -n1 \
    | grep -Eo '[0-9]+' \
    || true
}

stderr_has_fallback() {
  local errf="$1"
  if grep -Eqi 'auto:.*native CLI not found|using legacy' "$errf" 2>/dev/null; then
    return 0
  fi
  return 1
}

# Collect run rows as lines: id|engine|force|action|profile|rc|fallback|leaf|mid
RUN_ROWS=()

record_run() {
  local id="$1"
  local engine="$2"
  local force="$3"
  local action="$4"
  local profile_rel="$5"
  local rc="$6"
  local outf="$7"
  local errf="$8"

  local leaf mid fb=0
  leaf="$(extract_returns "$outf" leaf)"
  mid="$(extract_returns "$outf" mid)"

  if stderr_has_fallback "$errf"; then
    fb=1
  fi

  cat >"$OUT/runs/${id}.meta.json" <<EOF
{
  "id": $(python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$id"),
  "engine_requested": $(python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$engine"),
  "force_no_native": $([[ "$force" == "1" ]] && echo true || echo false),
  "action": $(python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$action"),
  "profile": $(python3 -c 'import json,sys; print(json.dumps(sys.argv[1]) if sys.argv[1] else "null")' "$profile_rel"),
  "rc": $rc,
  "stderr_fallback_note": $([[ "$fb" -eq 1 ]] && echo true || echo false),
  "leaf_returns": $([[ -n "$leaf" ]] && echo "$leaf" || echo null),
  "mid_returns": $([[ -n "$mid" ]] && echo "$mid" || echo null)
}
EOF

  RUN_ROWS+=("$id|$engine|$force|$action|$profile_rel|$rc|$fb|${leaf:-}|${mid:-}")
  ok "run $id rc=$rc leaf=${leaf:-?} mid=${mid:-?} fallback=$fb"
}

run_engine() {
  local id="$1"
  local engine="$2"
  local force="$3"
  local action="$4"
  local profile_abs="$5"
  local profile_rel="$6"

  local outf="$OUT/runs/${id}.stdout.txt"
  local errf="$OUT/runs/${id}.stderr.txt"
  local rcf="$OUT/runs/${id}.rc"
  local rc=0
  local -a env_prefix=()

  if [[ "$force" == "1" ]]; then
    env_prefix=(env NYTPROF_FORCE_NO_NATIVE=1)
  fi

  set +e
  "${env_prefix[@]}" "${ENGINE[@]}" --engine="$engine" "$action" "$profile_abs" \
    >"$outf" 2>"$errf"
  rc=$?
  set -e
  printf '%s\n' "$rc" >"$rcf"
  record_run "$id" "$engine" "$force" "$action" "$profile_rel" "$rc" "$outf" "$errf"
}

# ---------------------------------------------------------------------------
# Capability (when native present)
# ---------------------------------------------------------------------------
if [[ "$NATIVE_DISCOVERABLE" -eq 1 ]]; then
  CAP_CMD=()
  if [[ "$NATIVE_CLI_SPEC" == path:* ]]; then
    CAP_CMD=("${NATIVE_CLI_SPEC#path:}")
  else
    CAP_CMD=(cargo run -q -p nytprof-cli --)
  fi
  set +e
  "${CAP_CMD[@]}" capability --json \
    >"$OUT/capability/capability.json" \
    2>"$OUT/capability/capability.stderr.txt"
  cap_rc=$?
  set -e
  printf '%s\n' "$cap_rc" >"$OUT/capability/capability.rc"
  # human copy of stdout path naming
  if [[ -f "$OUT/capability/capability.json" ]]; then
    cp "$OUT/capability/capability.json" "$OUT/capability/capability.stdout.txt" 2>/dev/null || true
  fi
  ok "capability --json rc=$cap_rc"
else
  echo "skip: native not discoverable" >"$OUT/capability/capability.stdout.txt"
  : >"$OUT/capability/capability.stderr.txt"
  echo "1" >"$OUT/capability/capability.rc"
  echo '{"ok":false,"skipped":true,"reason":"native_not_discoverable"}' >"$OUT/capability/capability.json"
  ok "capability skipped (no native)"
fi

# ---------------------------------------------------------------------------
# Per-profile engine runs
# ---------------------------------------------------------------------------
for pref in "${PROFILES[@]}"; do
  if ! pabs="$(resolve_profile "$pref")"; then
    fail "profile not found: $pref"
  fi
  prel="$(rel_profile "$pref")"
  if [[ "$prel" == "$pref" && "$pabs" == "$ROOT/"* ]]; then
    prel="$(rel_profile "$pabs")"
  fi
  label="$(safe_label "$prel")"

  run_engine "engine_auto_report_${label}" auto 0 report "$pabs" "$prel"
  if [[ "$NATIVE_DISCOVERABLE" -eq 1 ]]; then
    run_engine "engine_native_report_${label}" native 0 report "$pabs" "$prel"
  fi
  # legacy is always attempted (oracle path); may fail if oracle pin absent — record rc
  run_engine "engine_legacy_report_${label}" legacy 0 report "$pabs" "$prel"

  if [[ "$SKIP_FORCE_NO_NATIVE" -eq 0 ]]; then
    run_engine "engine_auto_force_no_native_report_${label}" auto 1 report "$pabs" "$prel"
  fi

  # query on auto when useful (binary profiles via facade dump path)
  run_engine "engine_auto_query_${label}" auto 0 query "$pabs" "$prel"
done

# ---------------------------------------------------------------------------
# summary.json via Python for reliable JSON
# ---------------------------------------------------------------------------
export R3_OUT="$OUT"
export R3_SITE="$SITE"
export R3_NOTE="$NOTE"
export R3_NATIVE_DISCOVERABLE="$NATIVE_DISCOVERABLE"
export R3_NATIVE_CLI_SPEC="$NATIVE_CLI_SPEC"
export R3_ROOT="$ROOT"

# profiles list + run rows as env is awkward; write temp lists
printf '%s\n' "${PROFILES[@]}" >"$OUT/.profiles.txt"
printf '%s\n' "${RUN_ROWS[@]}" >"$OUT/.runs.txt"

python3 - <<'PY'
import json, os, pathlib, datetime, subprocess

out = pathlib.Path(os.environ["R3_OUT"])
root = pathlib.Path(os.environ["R3_ROOT"])
site = os.environ.get("R3_SITE") or None
note = os.environ.get("R3_NOTE") or None
native_disc = os.environ.get("R3_NATIVE_DISCOVERABLE") == "1"
native_spec = os.environ.get("R3_NATIVE_CLI_SPEC") or None

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
    # id|engine|force|action|profile|rc|fb|leaf|mid
    while len(parts) < 9:
        parts.append("")
    rid, engine, force, action, profile, rc, fb, leaf, mid = parts[:9]
    def as_int(x):
        x = (x or "").strip()
        return int(x) if x.isdigit() else None
    runs.append({
        "id": rid,
        "engine_requested": engine,
        "force_no_native": force == "1",
        "action": action,
        "profile": profile or None,
        "rc": int(rc) if str(rc).lstrip("-").isdigit() else None,
        "stderr_fallback_note": fb == "1",
        "leaf_returns": as_int(leaf),
        "mid_returns": as_int(mid),
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

fixture = None
for r in runs:
    if r["id"] == "engine_auto_report_default-calls1" and native_disc:
        fixture = {
            "leaf_returns": r.get("leaf_returns"),
            "mid_returns": r.get("mid_returns"),
            "auto_rc": r.get("rc"),
        }
        break
for r in runs:
    if r["id"] == "engine_native_report_default-calls1":
        if fixture is None:
            fixture = {}
        fixture["native_rc"] = r.get("rc")
        if fixture.get("leaf_returns") is None:
            fixture["leaf_returns"] = r.get("leaf_returns")
            fixture["mid_returns"] = r.get("mid_returns")
        break

summary = {
    "schema": "r3-field-window-mvp-v0",
    "generated_at_utc": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "git_commit": git_commit,
    "site": site,
    "note": note,
    "no_default_flip": True,
    "native_discoverable": native_disc,
    "native_cli_spec": native_spec,
    "profiles": profiles,
    "runs": runs,
    "fixture_default_calls1": fixture,
    "residuals": {
        "r3_product_default_flip": False,
        "r4_format_default_flip": False,
        "col007_product_writer": False,
        "v6_wire_freeze": False,
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
# R3 field-window evidence pack

**Schema:** \`r3-field-window-mvp-v0\`  
**Generated:** see \`summary.json\` / \`env/provenance.txt\`  
**Site:** ${SITE:-*(none)*}  
**Binding:** \`no_default_flip: true\` — this pack does **not** authorize product default changes.

## Contents

| Path | Role |
|------|------|
| \`summary.json\` | Machine-readable roll-up |
| \`env/provenance.txt\` | Host / tool provenance |
| \`capability/\` | Native \`capability --json\` (or skip) |
| \`runs/\` | Per-engine stdout/stderr/rc + meta |
| \`profiles/README.md\` | Profile path list (no blobs) |

## How produced

\`\`\`sh
./scripts/field/r3_field_window_collect.sh --out <this-dir> ...
\`\`\`

Guide: https://github.com/hilather/nytprof-modernization/blob/main/docs/R3_FIELD_WINDOW.md  
Report template: https://github.com/hilather/nytprof-modernization/blob/main/docs/templates/R3_FIELD_WINDOW_REPORT.md  
Schema: https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/r3-field-window-mvp-v0.md

## Explicit non-claims

- Not charter **R3** product default flip (PR-D02 / ADR-Q024 still required).
- Not **R4** format default, **COL-007**, v6 wire freeze, or public perf certification.
- Pure-Rust \`nytprof-cli\` \`auto\`→\`native\` residual remains; dual-path auto evidence is the Perl facade.
EOF

# Optional checksums
if command -v sha256sum >/dev/null 2>&1; then
  (cd "$OUT" && find . -type f ! -name SHA256SUMS ! -name '.profiles.txt' ! -name '.runs.txt' -print0 \
    | sort -z | xargs -0 sha256sum >SHA256SUMS) || true
elif command -v shasum >/dev/null 2>&1; then
  (cd "$OUT" && find . -type f ! -name SHA256SUMS -print0 \
    | sort -z | xargs -0 shasum -a 256 >SHA256SUMS) || true
fi

ok "R3 field-window pack written to $OUT"
ok "no_default_flip=true (product defaults unchanged)"
log "Next: fill docs/templates/R3_FIELD_WINDOW_REPORT.md from this pack"
