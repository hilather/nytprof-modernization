#!/usr/bin/env bash
# NATIVE-AGG-JSON: native CLI structured aggregates JSON smoke.
#
# Spec: docs/schemas/native-aggregates-json-mvp-v0.md
# Board: NATIVE-AGG-JSON
#
# Resolves the native CLI, runs `report --json` twice on default-calls1,
# asserts leaf_returns=15 / mid_returns=3 / mid_leaf_edge=15 (and maps),
# checks format aliases + aggregates subcommand + human default unchanged.
# Never puts crates/ on oracle PERL5LIB.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/native_agg_json_smoke.sh
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

if [[ ! -f "$ROOT/$FIXTURE_REL" ]]; then
  fail "missing fixture $FIXTURE_REL"
fi

# Sanity: never inject crates/ into oracle PERL5LIB from this smoke.
if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ (got: $PERL5LIB)"
fi

TMPDIR_SMOKE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT

# ---------------------------------------------------------------------------
# JSON assert helpers (python3 preferred; perl JSON::PP; greps last).
# ---------------------------------------------------------------------------
json_assert_mvp() {
  local f="$1"
  local label="$2"
  if command -v python3 >/dev/null 2>&1; then
    python3 -c '
import json,sys
path,label=sys.argv[1],sys.argv[2]
o=json.load(open(path,encoding="utf-8"))
def need(cond, msg):
    if not cond:
        raise SystemExit("%s: %s" % (label, msg))
need(o.get("ok") is True, "ok")
need(o.get("leaf_returns") == 15, "leaf_returns=%r" % (o.get("leaf_returns"),))
need(o.get("mid_returns") == 3, "mid_returns=%r" % (o.get("mid_returns"),))
need(o.get("mid_leaf_edge") == 15, "mid_leaf_edge=%r" % (o.get("mid_leaf_edge"),))
need(isinstance(o.get("discount_events"), int) and o["discount_events"] > 0, "discount_events")
subs=o.get("subs") or {}
need(subs.get("main::leaf") == 15, "subs leaf")
need(subs.get("main::mid") == 3, "subs mid")
edges=o.get("edges") or {}
ek="main::mid\tmain::leaf"
need(edges.get(ek) == 15, "edges mid->leaf")
need(o.get("profile"), "profile")
print("ok", label)
' "$f" "$label" || fail "$label: invalid JSON or MVP fields (python3)
$(cat "$f")"
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      my ($f,$label)=@ARGV;
      open my $fh, "<", $f or die "$label: $!";
      local $/; my $obj = JSON::PP->new->decode(<$fh>);
      die "$label: ok\n" unless $obj->{ok};
      die "$label: leaf_returns\n" unless ($obj->{leaf_returns} // -1) == 15;
      die "$label: mid_returns\n" unless ($obj->{mid_returns} // -1) == 3;
      die "$label: mid_leaf_edge\n" unless ($obj->{mid_leaf_edge} // -1) == 15;
      die "$label: discount_events\n" unless ($obj->{discount_events} // 0) > 0;
      my $subs = $obj->{subs};
      die "$label: subs\n" unless ref($subs) eq "HASH";
      die "$label: subs leaf\n" unless ($subs->{"main::leaf"} // -1) == 15;
      die "$label: subs mid\n"  unless ($subs->{"main::mid"}  // -1) == 3;
      my $edges = $obj->{edges};
      die "$label: edges\n" unless ref($edges) eq "HASH";
      my $ek = "main::mid\tmain::leaf";
      die "$label: edge mid->leaf\n" unless ($edges->{$ek} // -1) == 15;
      die "$label: profile\n" unless $obj->{profile};
      print "ok $label\n";
    ' "$f" "$label" || fail "$label: invalid JSON or MVP fields (perl JSON::PP)
$(cat "$f")"
  else
    grep -qE '"ok"[[:space:]]*:[[:space:]]*true' "$f" \
      || fail "$label: missing ok:true\n$(cat "$f")"
    grep -qE '"leaf_returns"[[:space:]]*:[[:space:]]*15' "$f" \
      || fail "$label: missing leaf_returns:15\n$(cat "$f")"
    grep -qE '"mid_returns"[[:space:]]*:[[:space:]]*3' "$f" \
      || fail "$label: missing mid_returns:3\n$(cat "$f")"
    grep -qE '"mid_leaf_edge"[[:space:]]*:[[:space:]]*15' "$f" \
      || fail "$label: missing mid_leaf_edge:15\n$(cat "$f")"
    grep -qE '"main::leaf"[[:space:]]*:[[:space:]]*15' "$f" \
      || fail "$label: missing subs main::leaf 15\n$(cat "$f")"
    log "NOTE: no python3/perl JSON::PP path fully exercised; used key greps for $label"
  fi
}

json_core_fingerprint() {
  local f="$1"
  if command -v python3 >/dev/null 2>&1; then
    python3 -c '
import json,sys
o=json.load(open(sys.argv[1],encoding="utf-8"))
print(o.get("leaf_returns"), o.get("mid_returns"), o.get("mid_leaf_edge"),
      o.get("discount_events"),
      o.get("subs",{}).get("main::leaf"), o.get("subs",{}).get("main::mid"),
      o.get("edges",{}).get("main::mid\tmain::leaf"), o.get("ok"))
' "$f"
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      open my $fh, "<", $ARGV[0] or die $!;
      local $/; my $o = JSON::PP->new->decode(<$fh>);
      my $ek = "main::mid\tmain::leaf";
      print join(" ",
        $o->{leaf_returns}//"", $o->{mid_returns}//"", $o->{mid_leaf_edge}//"",
        $o->{discount_events}//"",
        ($o->{subs}//{})->{"main::leaf"}//"", ($o->{subs}//{})->{"main::mid"}//"",
        ($o->{edges}//{})->{$ek}//"", $o->{ok} ? "1" : "0"), "\n";
    ' "$f"
  else
    cat "$f"
  fi
}

# ---------------------------------------------------------------------------
# 1. report --json ×2
# ---------------------------------------------------------------------------
echo "=== report --json default-calls1 ×2 ==="
JOUT1="$TMPDIR_SMOKE/agg_json_1.out"
JOUT2="$TMPDIR_SMOKE/agg_json_2.out"
JERR1="$TMPDIR_SMOKE/agg_json_1.err"
JERR2="$TMPDIR_SMOKE/agg_json_2.err"

if ! "${CLI_CMD[@]}" report --json "$FIXTURE_REL" >"$JOUT1" 2>"$JERR1"; then
  cat "$JERR1" >&2 || true
  cat "$JOUT1" >&2 || true
  fail "report --json run #1 failed"
fi
if ! "${CLI_CMD[@]}" report --json "$FIXTURE_REL" >"$JOUT2" 2>"$JERR2"; then
  cat "$JERR2" >&2 || true
  cat "$JOUT2" >&2 || true
  fail "report --json run #2 failed"
fi
cat "$JOUT1"
json_assert_mvp "$JOUT1" "json run #1"
json_assert_mvp "$JOUT2" "json run #2"
ok "report --json ×2: leaf=15 mid=3 edge=15"

FP1="$(json_core_fingerprint "$JOUT1")"
FP2="$(json_core_fingerprint "$JOUT2")"
if [[ "$FP1" != "$FP2" ]]; then
  fail "report --json not consistent across two runs
--- run1 fingerprint ---
$FP1
--- run2 fingerprint ---
$FP2
--- raw1 ---
$(cat "$JOUT1")
--- raw2 ---
$(cat "$JOUT2")"
fi
ok "report --json consistent across two runs ($FP1)"

# ---------------------------------------------------------------------------
# 2. --format=json / aggregates aliases
# ---------------------------------------------------------------------------
echo "=== report --format=json + aggregates ==="
FMT_OUT="$TMPDIR_SMOKE/agg_format.out"
FMT_ERR="$TMPDIR_SMOKE/agg_format.err"
if ! "${CLI_CMD[@]}" report --format=json "$FIXTURE_REL" \
  >"$FMT_OUT" 2>"$FMT_ERR"; then
  cat "$FMT_ERR" >&2 || true
  cat "$FMT_OUT" >&2 || true
  fail "report --format=json failed"
fi
json_assert_mvp "$FMT_OUT" "format=json"
ok "report --format=json: 15/3/15"

AGG_OUT="$TMPDIR_SMOKE/agg_sub.out"
AGG_ERR="$TMPDIR_SMOKE/agg_sub.err"
if ! "${CLI_CMD[@]}" aggregates "$FIXTURE_REL" >"$AGG_OUT" 2>"$AGG_ERR"; then
  cat "$AGG_ERR" >&2 || true
  cat "$AGG_OUT" >&2 || true
  fail "aggregates subcommand failed"
fi
json_assert_mvp "$AGG_OUT" "aggregates"
ok "aggregates: 15/3/15"

# ---------------------------------------------------------------------------
# 3. Human default unchanged when --json absent
# ---------------------------------------------------------------------------
echo "=== report human default (no --json) ==="
HUM_OUT="$TMPDIR_SMOKE/report_human.out"
HUM_ERR="$TMPDIR_SMOKE/report_human.err"
if ! "${CLI_CMD[@]}" report "$FIXTURE_REL" >"$HUM_OUT" 2>"$HUM_ERR"; then
  cat "$HUM_ERR" >&2 || true
  cat "$HUM_OUT" >&2 || true
  fail "report (human) failed"
fi
grep -q 'main::leaf' "$HUM_OUT" || fail "human report missing main::leaf:
$(cat "$HUM_OUT")"
grep -qE 'returns=15' "$HUM_OUT" || fail "human report missing returns=15:
$(cat "$HUM_OUT")"
if grep -qE '^\s*\{' "$HUM_OUT"; then
  # First non-empty line starting with { would look like JSON mode.
  if head -n1 "$HUM_OUT" | grep -qE '^\s*\{'; then
    fail "human report looks like JSON object:
$(cat "$HUM_OUT")"
  fi
fi
ok "human default: greppable leaf/returns=15 (no --json)"

ok "native_agg_json_smoke completed successfully"
