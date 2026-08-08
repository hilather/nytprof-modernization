#!/usr/bin/env bash
# JSON-NATIVE-STREAM-MVP: stream completeness + PID/timing fields on native
# report --json (and optional cross-check vs Perl query --json --jsonl).
#
# Spec: docs/schemas/native-aggregates-json-mvp-v0.md
# Board: JSON-NATIVE-STREAM-MVP
#
# Default-calls1 contract (native ProfileModel):
#   is_stream_complete     == true
#   incompleteness_reasons == []
#   time_line_events       >= 1  (golden observes 916)
#   pid_start_events       >= 1
#   pid_end_events         >= 1
#
# When Perl engine + golden readstream are present, also compares those shared
# stream/PID keys native ↔ perl (equal when both sides expose them).
# Never puts crates/ on oracle PERL5LIB. No XS.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/json_native_stream_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE_REL="fixtures/v5/default-calls1/nytprof.out"
GOLDEN_REL="fixtures/v5/default-calls1/readstream.jsonl"
ENGINE_BIN="perl/bin/nytprof-engine"
ENGINE_LIB="perl/lib"

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
  fail "no native CLI found (JSON-NATIVE-STREAM-MVP fails closed without native)
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
# Assert stream/PID fields on a native report --json object file.
# ---------------------------------------------------------------------------
assert_stream_fields() {
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
isc = o.get("is_stream_complete")
need(isc is True or isc == 1, "is_stream_complete=%r (want true)" % (isc,))
reasons = o.get("incompleteness_reasons")
need(isinstance(reasons, list) and len(reasons) == 0,
     "incompleteness_reasons=%r (want [])" % (reasons,))
for k in ("time_line_events", "pid_start_events", "pid_end_events"):
    v = o.get(k)
    need(isinstance(v, int) and v >= 1, "%s=%r (want >= 1)" % (k, v))
print("stream_ok", label,
      "tl=%s ps=%s pe=%s" % (
        o.get("time_line_events"), o.get("pid_start_events"), o.get("pid_end_events")))
' "$f" "$label" || fail "$label: stream/PID fields invalid (python3)
$(cat "$f")"
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      my ($f,$label)=@ARGV;
      open my $fh, "<", $f or die "$label: $!";
      local $/; my $obj = JSON::PP->new->decode(<$fh>);
      die "$label: ok\n" unless $obj->{ok};
      die "$label: is_stream_complete\n" unless $obj->{is_stream_complete};
      my $reasons = $obj->{incompleteness_reasons};
      die "$label: incompleteness_reasons\n"
        unless defined $reasons && ref($reasons) eq "ARRAY" && !@$reasons;
      for my $k (qw(time_line_events pid_start_events pid_end_events)) {
        die "$label: $k\n" unless ($obj->{$k} // 0) >= 1;
      }
      printf "stream_ok %s tl=%s ps=%s pe=%s\n", $label,
        $obj->{time_line_events}//"", $obj->{pid_start_events}//"",
        $obj->{pid_end_events}//"";
    ' "$f" "$label" || fail "$label: stream/PID fields invalid (perl JSON::PP)
$(cat "$f")"
  else
    grep -qE '"is_stream_complete"[[:space:]]*:[[:space:]]*true' "$f" \
      || fail "$label: missing is_stream_complete:true\n$(cat "$f")"
    grep -qE '"incompleteness_reasons"[[:space:]]*:[[:space:]]*\[\s*\]' "$f" \
      || fail "$label: missing incompleteness_reasons:[]\n$(cat "$f")"
    grep -qE '"time_line_events"[[:space:]]*:[[:space:]]*[1-9]' "$f" \
      || fail "$label: missing time_line_events >= 1\n$(cat "$f")"
    grep -qE '"pid_start_events"[[:space:]]*:[[:space:]]*[1-9]' "$f" \
      || fail "$label: missing pid_start_events >= 1\n$(cat "$f")"
    grep -qE '"pid_end_events"[[:space:]]*:[[:space:]]*[1-9]' "$f" \
      || fail "$label: missing pid_end_events >= 1\n$(cat "$f")"
    log "NOTE: no python3/perl JSON::PP; used greps for $label"
  fi
}

stream_fingerprint() {
  local f="$1"
  if command -v python3 >/dev/null 2>&1; then
    python3 -c '
import json,sys
o=json.load(open(sys.argv[1],encoding="utf-8"))
print(o.get("is_stream_complete"), len(o.get("incompleteness_reasons") or []),
      o.get("time_line_events"), o.get("pid_start_events"), o.get("pid_end_events"))
' "$f"
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      open my $fh, "<", $ARGV[0] or die $!;
      local $/; my $o = JSON::PP->new->decode(<$fh>);
      my $r = $o->{incompleteness_reasons};
      my $nr = (defined $r && ref($r) eq "ARRAY") ? scalar(@$r) : -1;
      print join(" ",
        $o->{is_stream_complete} ? "1" : "0", $nr,
        $o->{time_line_events}//"", $o->{pid_start_events}//"",
        $o->{pid_end_events}//""), "\n";
    ' "$f"
  else
    cat "$f"
  fi
}

# ---------------------------------------------------------------------------
# 1. native report --json ×2
# ---------------------------------------------------------------------------
echo "=== native report --json stream/PID (default-calls1) ×2 ==="
JOUT1="$TMPDIR_SMOKE/native_stream_1.out"
JOUT2="$TMPDIR_SMOKE/native_stream_2.out"
JERR1="$TMPDIR_SMOKE/native_stream_1.err"
JERR2="$TMPDIR_SMOKE/native_stream_2.err"

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
assert_stream_fields "$JOUT1" "native run #1"
assert_stream_fields "$JOUT2" "native run #2"
ok "native report --json ×2: is_stream_complete=true reasons=[] time/pid ≥ 1"

FP1="$(stream_fingerprint "$JOUT1")"
FP2="$(stream_fingerprint "$JOUT2")"
if [[ "$FP1" != "$FP2" ]]; then
  fail "stream fingerprint not consistent across two runs
--- run1 ---
$FP1
--- run2 ---
$FP2"
fi
ok "stream fingerprint consistent ($FP1)"

# ---------------------------------------------------------------------------
# 2. Optional: Perl query --json --jsonl shared stream/PID field compare
# ---------------------------------------------------------------------------
echo "=== optional Perl query --json --jsonl shared stream/PID compare ==="
if [[ -x "$ROOT/$ENGINE_BIN" || -f "$ROOT/$ENGINE_BIN" ]] \
  && [[ -d "$ROOT/$ENGINE_LIB" ]] \
  && [[ -f "$ROOT/$GOLDEN_REL" ]] \
  && command -v perl >/dev/null 2>&1; then
  ENGINE=(perl -I"$ENGINE_LIB" "$ENGINE_BIN")
  POUT="$TMPDIR_SMOKE/perl_stream.out"
  PERR="$TMPDIR_SMOKE/perl_stream.err"
  if "${ENGINE[@]}" query --json --jsonl "$GOLDEN_REL" >"$POUT" 2>"$PERR"; then
    if command -v python3 >/dev/null 2>&1; then
      python3 -c '
import json,sys
native=json.load(open(sys.argv[1],encoding="utf-8"))
perl=json.load(open(sys.argv[2],encoding="utf-8"))
keys=("is_stream_complete","incompleteness_reasons",
      "time_line_events","pid_start_events","pid_end_events")
missing_n=[k for k in keys if k not in native]
missing_p=[k for k in keys if k not in perl]
if missing_n:
    raise SystemExit("native missing fields: %s" % (missing_n,))
if missing_p:
    print("NOTE: perl missing stream fields %s — skip equal compare" % (missing_p,))
    raise SystemExit(0)
for k in keys:
    nv, pv = native.get(k), perl.get(k)
    # Normalize bool-ish is_stream_complete
    if k == "is_stream_complete":
        nv = bool(nv)
        pv = bool(pv)
    if nv != pv:
        raise SystemExit("native vs perl mismatch on %s: native=%r perl=%r" % (k, nv, pv))
print("cross_ok",
      "tl=%s ps=%s pe=%s" % (
        native.get("time_line_events"),
        native.get("pid_start_events"),
        native.get("pid_end_events")))
' "$JOUT1" "$POUT" || fail "native↔perl stream field compare failed
--- native ---
$(cat "$JOUT1")
--- perl ---
$(cat "$POUT")
--- perl stderr ---
$(cat "$PERR")"
      ok "native↔perl shared stream/PID fields equal on default-calls1"
    elif command -v perl >/dev/null 2>&1; then
      perl -MJSON::PP -e '
        my ($nf,$pf)=@ARGV;
        open my $nh,"<",$nf or die $!;
        open my $ph,"<",$pf or die $!;
        local $/;
        my $n=JSON::PP->new->decode(<$nh>);
        my $p=JSON::PP->new->decode(<$ph>);
        my @keys=qw(is_stream_complete incompleteness_reasons
                    time_line_events pid_start_events pid_end_events);
        for my $k (@keys) {
          die "native missing $k\n" unless exists $n->{$k};
        }
        for my $k (@keys) {
          if (!exists $p->{$k}) {
            print "NOTE: perl missing $k — skip equal compare\n";
            exit 0;
          }
        }
        for my $k (@keys) {
          my ($nv,$pv)=($n->{$k},$p->{$k});
          if ($k eq "is_stream_complete") {
            $nv = $nv ? 1 : 0;
            $pv = $pv ? 1 : 0;
          }
          if ($k eq "incompleteness_reasons") {
            my $na = (ref($nv) eq "ARRAY") ? join("|", @$nv) : "";
            my $pa = (ref($pv) eq "ARRAY") ? join("|", @$pv) : "";
            die "mismatch $k: native=$na perl=$pa\n" if $na ne $pa;
            next;
          }
          die "mismatch $k: native=$nv perl=$pv\n" if "$nv" ne "$pv";
        }
        printf "cross_ok tl=%s ps=%s pe=%s\n",
          $n->{time_line_events}//"", $n->{pid_start_events}//"",
          $n->{pid_end_events}//"";
      ' "$JOUT1" "$POUT" || fail "native↔perl stream field compare failed (perl)
--- native ---
$(cat "$JOUT1")
--- perl ---
$(cat "$POUT")"
      ok "native↔perl shared stream/PID fields equal on default-calls1"
    else
      log "NOTE: no python3/JSON::PP for cross compare; native asserts only"
    fi
  else
    log "NOTE: perl query --json --jsonl failed; native asserts only
$(cat "$PERR" 2>/dev/null || true)"
  fi
else
  log "NOTE: perl engine/golden not fully available — native stream asserts only"
fi

ok "json_native_stream_smoke completed successfully"
