#!/usr/bin/env bash
# NATIVE-QUERY-JSON-CROSS / NATIVE-QUERY-JSON-CROSS-EXPAND /
# NATIVE-QUERY-JSON-CROSS-BLOCKS / NATIVE-QUERY-JSON-CROSS-META /
# NATIVE-QUERY-JSON-CROSS-TIMEBLOCK / NATIVE-QUERY-JSON-CROSS-COUNTS /
# NATIVE-QUERY-JSON-CROSS-TOTAL:
# cross-check shared JSON fields between
#   1) native  nytprof-cli report --json  <profile.out>
#   2) Perl    nytprof-engine query --json --jsonl <readstream.jsonl>
#
# Shared fields (default-calls1 contract):
#   leaf_returns == 15
#   mid_returns  == 3
#   mid_leaf_edge == 15
#   discount_events == 818  (or equal between sides if dump-derived drift)
#   sub_entry_events == 0   (when BOTH sides expose the field; JSON-SUB-ENTRY-MVP)
#   time_block_events == 0  (when BOTH sides expose; CROSS-TIMEBLOCK)
#
# Total + basetime (NATIVE-QUERY-JSON-CROSS-TOTAL; when both expose):
#   total_events       == 2474
#   attribute_basetime equal dump-derived string (golden often "1786111723")
#
# Event counts + basename (NATIVE-QUERY-JSON-CROSS-COUNTS; when both expose):
#   sub_return_events  == 27
#   new_fid_events     == 3
#   sub_callers_events == 13
#   src_line_events    == 632
#   sub_info_events    == 31
#   file_1_basename exact equal OR both contain workload.pl
#   (absolute file_1 remains volatile; basename is the greppable stable sample)
#
# Meta / stream + A9/A8 (NATIVE-QUERY-JSON-CROSS-META; when both sides expose):
#   is_stream_complete == true
#   time_line_events / pid_start_events / pid_end_events equal
#   incompleteness_reasons equal (prefer empty []) when both expose
#   sub_def_leaf / sub_def_mid (fid/first_line/last_line) equal
#   source_line_1_5 equal
#   greppable meta REQUIRED equal when both expose (CROSS-TIMEBLOCK upgrade):
#     attribute_ticks_per_sec, option_calls,
#     file_1 (exact string equal OR both paths contain workload.pl)
#   other optional scalars (e.g. ticks_per_sec alias) equal when both expose
#
# Expand (NATIVE-QUERY-JSON-CROSS-EXPAND):
#   On fixtures/v5/calls2-default, when both sides expose sub_entry_events,
#   require sub_entry_events == 27 (side-by-side native report --json + Perl
#   query --json --jsonl; calls2 is fixture-scoped for the SUB_ENTRY count).
#
# Blocks (NATIVE-QUERY-JSON-CROSS-BLOCKS + CROSS-TIMEBLOCK):
#   On fixtures/v5/blocks-calls1, pair ×2 native report --json vs Perl
#   query --json --jsonl; require line_calls_1_5 == 780 and
#   block_line_calls_1_4 == 810 and equal native↔perl (JSON-BLOCKS-MVP ints);
#   when both sides expose time_block_events, require == 916.
#
# Runs the default-calls1 pair twice for consistency. Optional third path:
# query --json via native dump of the live profile when the CLI is available.
#
# Specs:
#   docs/schemas/native-aggregates-json-mvp-v0.md
#   docs/schemas/perl-engine-dispatch-mvp-v0.md
# Board: NATIVE-QUERY-JSON-CROSS / NATIVE-QUERY-JSON-CROSS-EXPAND /
#        NATIVE-QUERY-JSON-CROSS-BLOCKS / NATIVE-QUERY-JSON-CROSS-META /
#        NATIVE-QUERY-JSON-CROSS-TIMEBLOCK / NATIVE-QUERY-JSON-CROSS-COUNTS /
#        NATIVE-QUERY-JSON-CROSS-TOTAL
#
# Does NOT reimplement aggregation — invokes real CLIs and parses JSON.
# Never puts crates/ on oracle PERL5LIB. Fail-fast; fails closed without native.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/native_query_json_cross_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

PROFILE_REL="fixtures/v5/default-calls1/nytprof.out"
GOLDEN_REL="fixtures/v5/default-calls1/readstream.jsonl"
CALLS2_PROFILE_REL="fixtures/v5/calls2-default/nytprof.out"
CALLS2_GOLDEN_REL="fixtures/v5/calls2-default/readstream.jsonl"
BLOCKS_PROFILE_REL="fixtures/v5/blocks-calls1/nytprof.out"
BLOCKS_GOLDEN_REL="fixtures/v5/blocks-calls1/readstream.jsonl"
ENGINE_BIN="perl/bin/nytprof-engine"
ENGINE_LIB="perl/lib"

# Shared-field contract (default-calls1)
WANT_LEAF=15
WANT_MID=3
WANT_EDGE=15
WANT_DISCOUNT=818
WANT_SUB_ENTRY_DEFAULT=0
# calls2-default SUB_ENTRY multiplicity (fixture-scoped)
WANT_SUB_ENTRY_CALLS2=27
# blocks-calls1 A4/A4b greppable ints (JSON-BLOCKS-MVP / CROSS-BLOCKS)
WANT_LINE_CALLS_1_5=780
WANT_BLOCK_LINE_CALLS_1_4=810
# TIME_BLOCK multiplicity (CROSS-TIMEBLOCK; A2)
WANT_TIME_BLOCK_DEFAULT=0
WANT_TIME_BLOCK_BLOCKS=916
# Event multiplicity + basename (CROSS-COUNTS / JSON-EVENT-COUNTS-MVP /
# JSON-FILE-BASENAME-MVP) on default-calls1
WANT_SUB_RETURN=27
WANT_NEW_FID=3
WANT_SUB_CALLERS=13
WANT_SRC_LINE=632
WANT_SUB_INFO=31
WANT_FILE_1_BASENAME_NEEDLE=workload.pl
# Total record multiplicity + basetime sample (CROSS-TOTAL /
# JSON-TOTAL-EVENTS-MVP / JSON-ATTR-BASETIME-MVP) on default-calls1
WANT_TOTAL_EVENTS=2474

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
  fail "no native CLI found (NATIVE-QUERY-JSON-CROSS fails closed without native)
  looked for: \$NYTPROF_NATIVE_CLI, cargo + workspace Cargo.toml,
  prefix/bin/{nytprof-cli,nytprof-dump}, target/{debug,release}/nytprof-dump
  Install: ./scripts/packaging/install_native.sh
  Or build: cargo build -p nytprof-cli
  (pure-Perl query --json alone: ./scripts/packaging/perl_query_json_smoke.sh)"
fi

if [[ "$CLI_MODE" == "binary" ]]; then
  CLI_CMD=("$CLI_BIN")
  ok "using native binary: $CLI_BIN"
else
  CLI_CMD=(cargo run -q -p nytprof-cli --)
  ok "using cargo run -p nytprof-cli"
fi

# ---------------------------------------------------------------------------
# Preconditions
# ---------------------------------------------------------------------------
[[ -f "$ROOT/$PROFILE_REL" ]] || fail "missing fixture $PROFILE_REL"
[[ -f "$ROOT/$GOLDEN_REL" ]] || fail "missing golden dump $GOLDEN_REL"
[[ -f "$ROOT/$ENGINE_BIN" ]] || fail "missing $ENGINE_BIN"
[[ -d "$ROOT/$ENGINE_LIB" ]] || fail "missing $ENGINE_LIB"
command -v perl >/dev/null 2>&1 || fail "perl not on PATH"

ENGINE=(perl -I"$ENGINE_LIB" "$ENGINE_BIN")

# Sanity: never inject crates/ into oracle PERL5LIB from this smoke.
if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ (got: $PERL5LIB)"
fi

TMPDIR_SMOKE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT

# ---------------------------------------------------------------------------
# Extract shared fields as "leaf mid edge discount" (space-separated).
# ---------------------------------------------------------------------------
extract_shared() {
  local f="$1"
  local label="$2"
  if command -v python3 >/dev/null 2>&1; then
    python3 -c '
import json,sys
path,label=sys.argv[1],sys.argv[2]
try:
    o=json.load(open(path,encoding="utf-8"))
except Exception as e:
    sys.stderr.write("%s: JSON parse failed: %s\n" % (label, e))
    sys.exit(2)
if not isinstance(o, dict):
    sys.stderr.write("%s: not a JSON object\n" % label)
    sys.exit(2)
def geti(k):
    v=o.get(k)
    if isinstance(v, bool):
        return int(v)
    if isinstance(v, int):
        return v
    if isinstance(v, float) and v == int(v):
        return int(v)
    sys.stderr.write("%s: %s missing or not int (%r)\n" % (label, k, v))
    sys.exit(2)
print(geti("leaf_returns"), geti("mid_returns"), geti("mid_leaf_edge"),
      geti("discount_events"))
' "$f" "$label" || fail "$label: extract_shared failed
$(cat "$f")"
  elif command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -e '
      use strict; use warnings;
      my ($f,$label)=@ARGV;
      open my $fh, "<", $f or die "$label: $!\n";
      local $/; my $raw = <$fh>;
      my $o = eval { JSON::PP->new->decode($raw) };
      die "$label: JSON parse: $@\n" if $@ || ref($o) ne "HASH";
      for my $k (qw(leaf_returns mid_returns mid_leaf_edge discount_events)) {
        my $v = $o->{$k};
        die "$label: $k missing or not int\n"
          unless defined $v && $v =~ /^-?\d+$/;
      }
      print join(" ",
        0+$o->{leaf_returns}, 0+$o->{mid_returns},
        0+$o->{mid_leaf_edge}, 0+$o->{discount_events}), "\n";
    ' "$f" "$label" || fail "$label: extract_shared failed (perl)
$(cat "$f")"
  else
    fail "$label: need python3 or perl JSON::PP to parse cross-check JSON"
  fi
}

# Optional field: prints integer value, or empty line if key absent.
# Used for SUB_ENTRY expand — only assert when BOTH sides expose the field.
extract_optional_int() {
  local f="$1"
  local key="$2"
  local label="$3"
  if command -v python3 >/dev/null 2>&1; then
    python3 -c '
import json,sys
path,key,label=sys.argv[1],sys.argv[2],sys.argv[3]
try:
    o=json.load(open(path,encoding="utf-8"))
except Exception as e:
    sys.stderr.write("%s: JSON parse failed: %s\n" % (label, e))
    sys.exit(2)
if not isinstance(o, dict):
    sys.stderr.write("%s: not a JSON object\n" % label)
    sys.exit(2)
if key not in o:
    print("")
    sys.exit(0)
v=o[key]
if isinstance(v, bool):
    print(int(v))
elif isinstance(v, int):
    print(v)
elif isinstance(v, float) and v == int(v):
    print(int(v))
else:
    sys.stderr.write("%s: %s present but not int (%r)\n" % (label, key, v))
    sys.exit(2)
' "$f" "$key" "$label" || fail "$label: extract_optional_int($key) failed
$(cat "$f")"
  else
    perl -MJSON::PP -e '
      use strict; use warnings;
      my ($f,$key,$label)=@ARGV;
      open my $fh, "<", $f or die "$label: $!\n";
      local $/; my $raw = <$fh>;
      my $o = eval { JSON::PP->new->decode($raw) };
      die "$label: JSON parse: $@\n" if $@ || ref($o) ne "HASH";
      if (!exists $o->{$key}) { print "\n"; exit 0; }
      my $v = $o->{$key};
      die "$label: $key present but not int\n"
        unless defined $v && !ref($v) && $v =~ /^-?\d+$/;
      print 0+$v, "\n";
    ' "$f" "$key" "$label" || fail "$label: extract_optional_int($key) failed (perl)
$(cat "$f")"
  fi
}

assert_shared_contract() {
  local tuple="$1"  # "leaf mid edge discount"
  local label="$2"
  local leaf mid edge disc
  read -r leaf mid edge disc <<<"$tuple"
  [[ "$leaf" == "$WANT_LEAF" ]] \
    || fail "$label: leaf_returns=$leaf want $WANT_LEAF"
  [[ "$mid" == "$WANT_MID" ]] \
    || fail "$label: mid_returns=$mid want $WANT_MID"
  [[ "$edge" == "$WANT_EDGE" ]] \
    || fail "$label: mid_leaf_edge=$edge want $WANT_EDGE"
  # discount: prefer golden 818; allow equal-sides-only via caller when needed
  if [[ "$disc" != "$WANT_DISCOUNT" ]]; then
    log "NOTE: $label discount_events=$disc (golden contract is $WANT_DISCOUNT)"
  fi
}

assert_tuples_equal() {
  local a="$1" b="$2" label="$3"
  if [[ "$a" != "$b" ]]; then
    fail "$label: shared fields diverge
  side A: $a  (leaf mid edge discount)
  side B: $b  (leaf mid edge discount)"
  fi
}

# When both sides expose sub_entry_events, require equal + optional want value.
# If either side omits the field (pre-JSON-SUB-ENTRY-MVP), log and skip assert.
assert_sub_entry_when_both() {
  local native_out="$1"
  local perl_out="$2"
  local label="$3"
  local want="${4:-}"  # empty = equal only; non-empty = also require this value

  local n_se p_se
  n_se="$(extract_optional_int "$native_out" "sub_entry_events" "$label native")"
  p_se="$(extract_optional_int "$perl_out" "sub_entry_events" "$label perl")"

  if [[ -z "$n_se" && -z "$p_se" ]]; then
    log "NOTE: $label: neither side exposes sub_entry_events (skip SUB_ENTRY cross)"
    return 0
  fi
  if [[ -z "$n_se" ]]; then
    log "NOTE: $label: native omits sub_entry_events (perl=$p_se); skip equal assert"
    return 0
  fi
  if [[ -z "$p_se" ]]; then
    log "NOTE: $label: perl omits sub_entry_events (native=$n_se); skip equal assert"
    return 0
  fi

  [[ "$n_se" == "$p_se" ]] \
    || fail "$label: sub_entry_events diverge native=$n_se perl=$p_se"

  if [[ -n "$want" ]]; then
    [[ "$n_se" == "$want" ]] \
      || fail "$label: sub_entry_events=$n_se want $want (both sides)"
  fi

  ok "$label: sub_entry_events equal ($n_se)${want:+ want=$want}"
}

# When both sides expose time_block_events, require equal + optional want value.
# If either side omits the field, log and skip assert (pre-JSON-TIME-BLOCK-MVP).
assert_time_block_when_both() {
  local native_out="$1"
  local perl_out="$2"
  local label="$3"
  local want="${4:-}"  # empty = equal only; non-empty = also require this value

  local n_tb p_tb
  n_tb="$(extract_optional_int "$native_out" "time_block_events" "$label native")"
  p_tb="$(extract_optional_int "$perl_out" "time_block_events" "$label perl")"

  if [[ -z "$n_tb" && -z "$p_tb" ]]; then
    log "NOTE: $label: neither side exposes time_block_events (skip TIME_BLOCK cross)"
    return 0
  fi
  if [[ -z "$n_tb" ]]; then
    log "NOTE: $label: native omits time_block_events (perl=$p_tb); skip equal assert"
    return 0
  fi
  if [[ -z "$p_tb" ]]; then
    log "NOTE: $label: perl omits time_block_events (native=$n_tb); skip equal assert"
    return 0
  fi

  [[ "$n_tb" == "$p_tb" ]] \
    || fail "$label: time_block_events diverge native=$n_tb perl=$p_tb"

  if [[ -n "$want" ]]; then
    [[ "$n_tb" == "$want" ]] \
      || fail "$label: time_block_events=$n_tb want $want (both sides)"
  fi

  ok "$label: time_block_events equal ($n_tb)${want:+ want=$want}"
}

# When both sides expose JSON-EVENT-COUNTS-MVP fields, require equal + golden
# default-calls1 contract 27/3/13/632/31. Skip-with-NOTE if only one side
# exposes a field (partial landing).
assert_event_counts_when_both() {
  local native_out="$1"
  local perl_out="$2"
  local label="$3"

  local keys=(
    "sub_return_events:$WANT_SUB_RETURN"
    "new_fid_events:$WANT_NEW_FID"
    "sub_callers_events:$WANT_SUB_CALLERS"
    "src_line_events:$WANT_SRC_LINE"
    "sub_info_events:$WANT_SUB_INFO"
  )
  local any=0
  local parts=()
  local pair key want n_v p_v
  for pair in "${keys[@]}"; do
    key="${pair%%:*}"
    want="${pair##*:}"
    n_v="$(extract_optional_int "$native_out" "$key" "$label native $key")"
    p_v="$(extract_optional_int "$perl_out" "$key" "$label perl $key")"
    if [[ -z "$n_v" && -z "$p_v" ]]; then
      log "NOTE: $label: neither side exposes $key (skip)"
      continue
    fi
    if [[ -z "$n_v" ]]; then
      log "NOTE: $label: native omits $key (perl=$p_v); skip equal assert"
      continue
    fi
    if [[ -z "$p_v" ]]; then
      log "NOTE: $label: perl omits $key (native=$n_v); skip equal assert"
      continue
    fi
    [[ "$n_v" == "$p_v" ]] \
      || fail "$label: $key diverge native=$n_v perl=$p_v"
    [[ "$n_v" == "$want" ]] \
      || fail "$label: $key=$n_v want $want (both sides)"
    parts+=("$key=$n_v")
    any=1
  done
  if [[ "$any" -eq 1 ]]; then
    ok "$label: event counts equal (${parts[*]})"
  else
    log "NOTE: $label: no shared event-count fields on both sides; skip CROSS-COUNTS"
  fi
}

# Optional string field: prints value, or empty line if key absent.
# Used for file_1_basename CROSS-COUNTS — only assert when BOTH sides expose.
extract_optional_str() {
  local f="$1"
  local key="$2"
  local label="$3"
  if command -v python3 >/dev/null 2>&1; then
    python3 -c '
import json,sys
path,key,label=sys.argv[1],sys.argv[2],sys.argv[3]
try:
    o=json.load(open(path,encoding="utf-8"))
except Exception as e:
    sys.stderr.write("%s: JSON parse failed: %s\n" % (label, e))
    sys.exit(2)
if not isinstance(o, dict):
    sys.stderr.write("%s: not a JSON object\n" % label)
    sys.exit(2)
if key not in o:
    print("")
    sys.exit(0)
v=o[key]
if v is None:
    print("")
    sys.exit(0)
if not isinstance(v, str):
    sys.stderr.write("%s: %s present but not string (%r)\n" % (label, key, v))
    sys.exit(2)
print(v)
' "$f" "$key" "$label" || fail "$label: extract_optional_str($key) failed
$(cat "$f")"
  else
    perl -MJSON::PP -e '
      use strict; use warnings;
      my ($f,$key,$label)=@ARGV;
      open my $fh, "<", $f or die "$label: $!\n";
      local $/; my $raw = <$fh>;
      my $o = eval { JSON::PP->new->decode($raw) };
      die "$label: JSON parse: $@\n" if $@ || ref($o) ne "HASH";
      if (!exists $o->{$key}) { print "\n"; exit 0; }
      my $v = $o->{$key};
      if (!defined $v) { print "\n"; exit 0; }
      die "$label: $key present but not string\n" if ref($v);
      print $v, "\n";
    ' "$f" "$key" "$label" || fail "$label: extract_optional_str($key) failed (perl)
$(cat "$f")"
  fi
}

# When both sides expose file_1_basename, require exact equal OR both contain
# workload.pl (JSON-FILE-BASENAME-MVP / CROSS-COUNTS). Absolute file_1 is
# volatile under /tmp; basename is the greppable stable sample.
assert_file_basename_when_both() {
  local native_out="$1"
  local perl_out="$2"
  local label="$3"
  local needle="${4:-$WANT_FILE_1_BASENAME_NEEDLE}"

  local n_b p_b
  n_b="$(extract_optional_str "$native_out" "file_1_basename" "$label native")"
  p_b="$(extract_optional_str "$perl_out" "file_1_basename" "$label perl")"

  if [[ -z "$n_b" && -z "$p_b" ]]; then
    log "NOTE: $label: neither side exposes file_1_basename (skip basename cross)"
    return 0
  fi
  if [[ -z "$n_b" ]]; then
    log "NOTE: $label: native omits file_1_basename (perl=$p_b); skip equal assert"
    return 0
  fi
  if [[ -z "$p_b" ]]; then
    log "NOTE: $label: perl omits file_1_basename (native=$n_b); skip equal assert"
    return 0
  fi

  if [[ "$n_b" == "$p_b" ]]; then
    :
  elif [[ "$n_b" == *"$needle"* && "$p_b" == *"$needle"* ]]; then
    :
  else
    fail "$label: file_1_basename diverge native=$n_b perl=$p_b (want exact equal or both contain $needle)"
  fi
  # Prefer golden needle on default-calls1 when both expose.
  if [[ "$n_b" != *"$needle"* ]]; then
    fail "$label: file_1_basename=$n_b does not contain $needle"
  fi
  ok "$label: file_1_basename equal ($n_b)"
}

# When both sides expose total_events, require equal + golden 2474 (CROSS-TOTAL).
assert_total_events_when_both() {
  local native_out="$1"
  local perl_out="$2"
  local label="$3"
  local want="${4:-$WANT_TOTAL_EVENTS}"

  local n_v p_v
  n_v="$(extract_optional_int "$native_out" "total_events" "$label native")"
  p_v="$(extract_optional_int "$perl_out" "total_events" "$label perl")"

  if [[ -z "$n_v" && -z "$p_v" ]]; then
    log "NOTE: $label: neither side exposes total_events (skip CROSS-TOTAL total)"
    return 0
  fi
  if [[ -z "$n_v" ]]; then
    log "NOTE: $label: native omits total_events (perl=$p_v); skip equal assert"
    return 0
  fi
  if [[ -z "$p_v" ]]; then
    log "NOTE: $label: perl omits total_events (native=$n_v); skip equal assert"
    return 0
  fi
  [[ "$n_v" == "$p_v" ]] \
    || fail "$label: total_events diverge native=$n_v perl=$p_v"
  [[ "$n_v" == "$want" ]] \
    || fail "$label: total_events=$n_v want $want (both sides)"
  ok "$label: total_events equal ($n_v)"
}

# When both sides expose attribute_basetime, require exact equal dump string
# (CROSS-TOTAL / JSON-ATTR-BASETIME-MVP).
assert_attribute_basetime_when_both() {
  local native_out="$1"
  local perl_out="$2"
  local label="$3"

  local n_b p_b
  n_b="$(extract_optional_str "$native_out" "attribute_basetime" "$label native")"
  p_b="$(extract_optional_str "$perl_out" "attribute_basetime" "$label perl")"

  if [[ -z "$n_b" && -z "$p_b" ]]; then
    log "NOTE: $label: neither side exposes attribute_basetime (skip CROSS-TOTAL basetime)"
    return 0
  fi
  if [[ -z "$n_b" ]]; then
    log "NOTE: $label: native omits attribute_basetime (perl=$p_b); skip equal assert"
    return 0
  fi
  if [[ -z "$p_b" ]]; then
    log "NOTE: $label: perl omits attribute_basetime (native=$n_b); skip equal assert"
    return 0
  fi
  [[ "$n_b" == "$p_b" ]] \
    || fail "$label: attribute_basetime diverge native=$n_b perl=$p_b"
  ok "$label: attribute_basetime equal ($n_b)"
}

# ---------------------------------------------------------------------------
# META: stream/PID + A9/A8 samples (when both sides expose the fields).
# Prints a compact fingerprint on success for round-consistency checks:
#   isc|n_reasons|tl|ps|pe|leaf_fid:first:last|mid_fid:first:last|src_len
# Empty line if required keys absent on either side (skip).
# ---------------------------------------------------------------------------
assert_meta_when_both() {
  local native_out="$1"
  local perl_out="$2"
  local label="$3"
  local require_complete="${4:-1}"  # 1 → also require is_stream_complete true

  if command -v python3 >/dev/null 2>&1; then
    python3 -c '
import json,sys
nf, pf, label, req = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
try:
    n = json.load(open(nf, encoding="utf-8"))
    p = json.load(open(pf, encoding="utf-8"))
except Exception as e:
    sys.stderr.write("%s: JSON parse failed: %s\n" % (label, e))
    sys.exit(2)
if not isinstance(n, dict) or not isinstance(p, dict):
    sys.stderr.write("%s: not JSON objects\n" % label)
    sys.exit(2)

# Stream/PID keys (JSON-NATIVE-STREAM-MVP)
stream_keys = ("is_stream_complete", "time_line_events",
               "pid_start_events", "pid_end_events")
# incompleteness_reasons is preferred when both expose (cheap equal)
reason_key = "incompleteness_reasons"
# A9/A8 sample keys (JSON-SUBDEF-SOURCE-MVP)
subdef_keys = ("sub_def_leaf", "sub_def_mid", "source_line_1_5")

def both_have(keys):
    n_ok = all(k in n for k in keys)
    p_ok = all(k in p for k in keys)
    return n_ok, p_ok

def norm_bool(v):
    if isinstance(v, bool):
        return v
    if v in (1, "1", "true", "True"):
        return True
    if v in (0, "0", "false", "False", None):
        return False
    return bool(v)

def norm_int(v, k):
    if isinstance(v, bool):
        return int(v)
    if isinstance(v, int):
        return v
    if isinstance(v, float) and v == int(v):
        return int(v)
    raise SystemExit("%s: %s not int (%r)" % (label, k, v))

def subdef_tuple(v, k):
    if v is None:
        return None
    if not isinstance(v, dict):
        raise SystemExit("%s: %s not object/null (%r)" % (label, k, v))
    for fk in ("fid", "first_line", "last_line"):
        if fk not in v:
            raise SystemExit("%s: %s missing %s" % (label, k, fk))
    return (norm_int(v["fid"], k+".fid"),
            norm_int(v["first_line"], k+".first_line"),
            norm_int(v["last_line"], k+".last_line"))

n_stream, p_stream = both_have(stream_keys)
n_sub, p_sub = both_have(subdef_keys)

if not n_stream and not p_stream and not n_sub and not p_sub:
    print("")  # nothing to assert
    sys.exit(0)

parts = []

if n_stream and p_stream:
    n_isc = norm_bool(n["is_stream_complete"])
    p_isc = norm_bool(p["is_stream_complete"])
    if n_isc != p_isc:
        raise SystemExit("%s: is_stream_complete diverge native=%r perl=%r"
                         % (label, n["is_stream_complete"], p["is_stream_complete"]))
    if req == "1" and not n_isc:
        raise SystemExit("%s: is_stream_complete want true (both sides)" % label)
    for k in ("time_line_events", "pid_start_events", "pid_end_events"):
        ni, pi = norm_int(n[k], "native."+k), norm_int(p[k], "perl."+k)
        if ni != pi:
            raise SystemExit("%s: %s diverge native=%s perl=%s" % (label, k, ni, pi))
    # incompleteness_reasons when both expose
    if reason_key in n and reason_key in p:
        nr, pr = n[reason_key], p[reason_key]
        if not isinstance(nr, list) or not isinstance(pr, list):
            raise SystemExit("%s: incompleteness_reasons must be arrays" % label)
        if nr != pr:
            raise SystemExit("%s: incompleteness_reasons diverge native=%r perl=%r"
                             % (label, nr, pr))
        n_reasons = len(nr)
        if req == "1" and n_isc and n_reasons != 0:
            raise SystemExit("%s: incompleteness_reasons non-empty when complete: %r"
                             % (label, nr))
    else:
        n_reasons = -1  # not both exposed
    tl = norm_int(n["time_line_events"], "time_line_events")
    ps = norm_int(n["pid_start_events"], "pid_start_events")
    pe = norm_int(n["pid_end_events"], "pid_end_events")
    parts.append("stream=isc:%s reasons:%s tl:%s ps:%s pe:%s"
                 % (int(n_isc), n_reasons, tl, ps, pe))
elif n_stream or p_stream:
    # only one side has full stream set — note and skip stream assert
    sys.stderr.write("NOTE: %s: stream/PID fields only on one side "
                     "(native=%s perl=%s); skip stream equal\n"
                     % (label, n_stream, p_stream))
else:
    pass

if n_sub and p_sub:
    for k in ("sub_def_leaf", "sub_def_mid"):
        nt, pt = subdef_tuple(n[k], "native."+k), subdef_tuple(p[k], "perl."+k)
        if nt != pt:
            raise SystemExit("%s: %s diverge native=%r perl=%r" % (label, k, n[k], p[k]))
    ns, ps = n["source_line_1_5"], p["source_line_1_5"]
    if ns is None and ps is None:
        src_len = 0
    elif isinstance(ns, str) and isinstance(ps, str):
        if ns != ps:
            raise SystemExit("%s: source_line_1_5 diverge native=%r perl=%r"
                             % (label, ns, ps))
        src_len = len(ns)
    else:
        raise SystemExit("%s: source_line_1_5 type mismatch native=%r perl=%r"
                         % (label, ns, ps))
    lt = subdef_tuple(n["sub_def_leaf"], "sub_def_leaf")
    mt = subdef_tuple(n["sub_def_mid"], "sub_def_mid")
    parts.append("subdef=leaf:%s mid:%s src_len:%s"
                 % ("%s:%s:%s" % lt if lt else "null",
                    "%s:%s:%s" % mt if mt else "null",
                    src_len))
elif n_sub or p_sub:
    sys.stderr.write("NOTE: %s: A9/A8 sample fields only on one side "
                     "(native=%s perl=%s); skip subdef equal\n"
                     % (label, n_sub, p_sub))

# Greppable meta REQUIRED equal when both expose (CROSS-TIMEBLOCK / JSON-META-FILES-MVP):
#   attribute_ticks_per_sec, option_calls, file_1
# file_1: exact string equal OR both paths contain "workload.pl"
# Missing on one side only → skip-with-NOTE for that field.
required_meta = (
    "attribute_ticks_per_sec",
    "option_calls",
    "file_1",
)
req_parts = []
for k in required_meta:
    n_has, p_has = (k in n), (k in p)
    if n_has and p_has:
        nv, pv = n[k], p[k]
        if k == "file_1":
            # exact equal, or both non-null strings containing workload.pl
            if nv == pv:
                pass
            elif (isinstance(nv, str) and isinstance(pv, str)
                  and "workload.pl" in nv and "workload.pl" in pv):
                pass
            else:
                raise SystemExit(
                    "%s: greppable meta file_1 diverge native=%r perl=%r "
                    "(want exact equal or both contain workload.pl)"
                    % (label, nv, pv))
        else:
            if nv != pv:
                raise SystemExit(
                    "%s: greppable meta %s diverge native=%r perl=%r"
                    % (label, k, nv, pv))
        req_parts.append("%s=%r" % (k, nv))
    elif n_has or p_has:
        sys.stderr.write(
            "NOTE: %s: greppable meta %s only on one side "
            "(native=%s perl=%s); skip equal for this field\n"
            % (label, k, n_has, p_has))
if req_parts:
    parts.append("grep_meta=" + ",".join(req_parts))

# Nice-to-have: other optional scalar meta when both expose
optional_meta = (
    "ticks_per_sec",
    "files_count",
    "application",
)
opt_parts = []
for k in optional_meta:
    if k in n and k in p:
        nv, pv = n[k], p[k]
        if nv != pv:
            raise SystemExit("%s: optional meta %s diverge native=%r perl=%r"
                             % (label, k, nv, pv))
        opt_parts.append("%s=%r" % (k, nv))
if opt_parts:
    parts.append("opt_meta=" + ",".join(opt_parts))

if not parts:
    print("")
    sys.exit(0)
print("|".join(parts))
' "$native_out" "$perl_out" "$label" "$require_complete" \
      || fail "$label: assert_meta_when_both failed
--- native ---
$(cat "$native_out")
--- perl ---
$(cat "$perl_out")"
  else
    # perl JSON::PP fallback (stream ints + sub_def ranges + source string)
    perl -MJSON::PP -e '
      use strict; use warnings;
      my ($nf,$pf,$label,$req)=@ARGV;
      open my $nh,"<",$nf or die "$label: $nf: $!\n";
      open my $ph,"<",$pf or die "$label: $pf: $!\n";
      local $/;
      my $n = eval { JSON::PP->new->decode(<$nh>) };
      die "$label: native JSON: $@\n" if $@ || ref($n) ne "HASH";
      my $p = eval { JSON::PP->new->decode(<$ph>) };
      die "$label: perl JSON: $@\n" if $@ || ref($p) ne "HASH";

      my @stream = qw(is_stream_complete time_line_events pid_start_events pid_end_events);
      my @subdef = qw(sub_def_leaf sub_def_mid source_line_1_5);
      my $n_stream = !grep { !exists $n->{$_} } @stream;
      my $p_stream = !grep { !exists $p->{$_} } @stream;
      my $n_sub = !grep { !exists $n->{$_} } @subdef;
      my $p_sub = !grep { !exists $p->{$_} } @subdef;

      my @parts;
      if ($n_stream && $p_stream) {
        my $n_isc = $n->{is_stream_complete} ? 1 : 0;
        my $p_isc = $p->{is_stream_complete} ? 1 : 0;
        die "$label: is_stream_complete diverge\n" if $n_isc != $p_isc;
        die "$label: is_stream_complete want true\n" if $req eq "1" && !$n_isc;
        for my $k (qw(time_line_events pid_start_events pid_end_events)) {
          my ($ni,$pi)=(0+$n->{$k}, 0+$p->{$k});
          die "$label: $k diverge native=$ni perl=$pi\n" if $ni != $pi;
        }
        my $n_reasons = -1;
        if (exists $n->{incompleteness_reasons} && exists $p->{incompleteness_reasons}) {
          my $nr = $n->{incompleteness_reasons};
          my $pr = $p->{incompleteness_reasons};
          die "$label: incompleteness_reasons not arrays\n"
            unless ref($nr) eq "ARRAY" && ref($pr) eq "ARRAY";
          my $ns = join("|", @$nr);
          my $ps = join("|", @$pr);
          die "$label: incompleteness_reasons diverge\n" if $ns ne $ps;
          $n_reasons = scalar @$nr;
          die "$label: incompleteness_reasons non-empty when complete\n"
            if $req eq "1" && $n_isc && $n_reasons != 0;
        }
        push @parts, sprintf(
          "stream=isc:%d reasons:%d tl:%s ps:%s pe:%s",
          $n_isc, $n_reasons,
          0+$n->{time_line_events}, 0+$n->{pid_start_events}, 0+$n->{pid_end_events});
      } elsif ($n_stream || $p_stream) {
        warn "NOTE: $label: stream/PID fields only on one side; skip stream equal\n";
      }

      if ($n_sub && $p_sub) {
        for my $k (qw(sub_def_leaf sub_def_mid)) {
          my ($nv,$pv)=($n->{$k},$p->{$k});
          if (!defined $nv && !defined $pv) { next; }
          die "$label: $k type\n" unless ref($nv) eq "HASH" && ref($pv) eq "HASH";
          for my $fk (qw(fid first_line last_line)) {
            die "$label: $k.$fk diverge\n" if 0+$nv->{$fk} != 0+$pv->{$fk};
          }
        }
        my ($ns,$ps)=($n->{source_line_1_5},$p->{source_line_1_5});
        if (defined $ns || defined $ps) {
          die "$label: source_line_1_5 diverge\n"
            if !defined $ns || !defined $ps || $ns ne $ps;
        }
        my $lt = $n->{sub_def_leaf};
        my $mt = $n->{sub_def_mid};
        my $src_len = defined $n->{source_line_1_5} ? length($n->{source_line_1_5}) : 0;
        push @parts, sprintf(
          "subdef=leaf:%s:%s:%s mid:%s:%s:%s src_len:%d",
          map { 0+$_ } ($lt->{fid},$lt->{first_line},$lt->{last_line},
                        $mt->{fid},$mt->{first_line},$mt->{last_line}),
          $src_len);
      } elsif ($n_sub || $p_sub) {
        warn "NOTE: $label: A9/A8 sample fields only on one side; skip subdef equal\n";
      }

      # Greppable meta required equal when both expose (CROSS-TIMEBLOCK)
      my @req_meta = qw(attribute_ticks_per_sec option_calls file_1);
      my @req_parts;
      for my $k (@req_meta) {
        my $n_has = exists $n->{$k};
        my $p_has = exists $p->{$k};
        if ($n_has && $p_has) {
          my ($nv,$pv)=($n->{$k},$p->{$k});
          if ($k eq "file_1") {
            my $n_s = defined $nv ? $nv : "";
            my $p_s = defined $pv ? $pv : "";
            if ($n_s eq $p_s) {
              # exact equal ok
            } elsif (index($n_s, "workload.pl") >= 0
                  && index($p_s, "workload.pl") >= 0) {
              # both contain workload.pl ok
            } else {
              die "$label: greppable meta file_1 diverge native=$n_s perl=$p_s\n";
            }
          } else {
            die "$label: greppable meta $k diverge\n"
              if (defined $nv ? $nv : "") ne (defined $pv ? $pv : "");
          }
          push @req_parts, sprintf("%s=%s", $k, defined $nv ? $nv : "null");
        } elsif ($n_has || $p_has) {
          warn "NOTE: $label: greppable meta $k only on one side; skip equal\n";
        }
      }
      push @parts, "grep_meta=" . join(",", @req_parts) if @req_parts;

      for my $k (qw(ticks_per_sec files_count application)) {
        next unless exists $n->{$k} && exists $p->{$k};
        my ($nv,$pv)=($n->{$k},$p->{$k});
        die "$label: optional meta $k diverge\n"
          if (defined $nv ? $nv : "") ne (defined $pv ? $pv : "");
      }

      if (!@parts) { print "\n"; exit 0; }
      print join("|", @parts), "\n";
    ' "$native_out" "$perl_out" "$label" "$require_complete" \
      || fail "$label: assert_meta_when_both failed (perl)
--- native ---
$(cat "$native_out")
--- perl ---
$(cat "$perl_out")"
  fi
}

# Extract A4/A4b greppable ints as "line_calls_1_5 block_line_calls_1_4".
extract_blocks() {
  local f="$1"
  local label="$2"
  if command -v python3 >/dev/null 2>&1; then
    python3 -c '
import json,sys
path,label=sys.argv[1],sys.argv[2]
try:
    o=json.load(open(path,encoding="utf-8"))
except Exception as e:
    sys.stderr.write("%s: JSON parse failed: %s\n" % (label, e))
    sys.exit(2)
if not isinstance(o, dict):
    sys.stderr.write("%s: not a JSON object\n" % label)
    sys.exit(2)
def geti(k):
    v=o.get(k)
    if isinstance(v, bool):
        return int(v)
    if isinstance(v, int):
        return v
    if isinstance(v, float) and v == int(v):
        return int(v)
    sys.stderr.write("%s: %s missing or not int (%r)\n" % (label, k, v))
    sys.exit(2)
print(geti("line_calls_1_5"), geti("block_line_calls_1_4"))
' "$f" "$label" || fail "$label: extract_blocks failed
$(cat "$f")"
  else
    perl -MJSON::PP -e '
      use strict; use warnings;
      my ($f,$label)=@ARGV;
      open my $fh, "<", $f or die "$label: $!\n";
      local $/; my $raw = <$fh>;
      my $o = eval { JSON::PP->new->decode($raw) };
      die "$label: JSON parse: $@\n" if $@ || ref($o) ne "HASH";
      for my $k (qw(line_calls_1_5 block_line_calls_1_4)) {
        my $v = $o->{$k};
        die "$label: $k missing or not int\n"
          unless defined $v && $v =~ /^-?\d+$/;
      }
      print join(" ",
        0+$o->{line_calls_1_5}, 0+$o->{block_line_calls_1_4}), "\n";
    ' "$f" "$label" || fail "$label: extract_blocks failed (perl)
$(cat "$f")"
  fi
}

assert_blocks_contract() {
  local tuple="$1"  # "line_calls block_line_calls"
  local label="$2"
  local lc bl
  read -r lc bl <<<"$tuple"
  [[ "$lc" == "$WANT_LINE_CALLS_1_5" ]] \
    || fail "$label: line_calls_1_5=$lc want $WANT_LINE_CALLS_1_5"
  [[ "$bl" == "$WANT_BLOCK_LINE_CALLS_1_4" ]] \
    || fail "$label: block_line_calls_1_4=$bl want $WANT_BLOCK_LINE_CALLS_1_4"
}

# ---------------------------------------------------------------------------
# Cross pair ×2 (default-calls1)
# ---------------------------------------------------------------------------
PREV_NATIVE=""
PREV_PERL=""
PREV_NATIVE_SE=""
PREV_PERL_SE=""
PREV_NATIVE_TB=""
PREV_PERL_TB=""
PREV_NATIVE_SR=""
PREV_PERL_SR=""
PREV_NATIVE_BN=""
PREV_PERL_BN=""
PREV_NATIVE_TE=""
PREV_PERL_TE=""
PREV_NATIVE_BT=""
PREV_PERL_BT=""
PREV_META_FP=""

for round in 1 2; do
  echo "=== NATIVE-QUERY-JSON-CROSS round $round (default-calls1) ==="

  NATIVE_OUT="$TMPDIR_SMOKE/native_r${round}.out"
  NATIVE_ERR="$TMPDIR_SMOKE/native_r${round}.err"
  PERL_OUT="$TMPDIR_SMOKE/perl_r${round}.out"
  PERL_ERR="$TMPDIR_SMOKE/perl_r${round}.err"

  if ! "${CLI_CMD[@]}" report --json "$PROFILE_REL" \
    >"$NATIVE_OUT" 2>"$NATIVE_ERR"; then
    cat "$NATIVE_ERR" >&2 || true
    cat "$NATIVE_OUT" >&2 || true
    fail "native report --json round $round failed"
  fi

  if ! "${ENGINE[@]}" query --json --jsonl "$GOLDEN_REL" \
    >"$PERL_OUT" 2>"$PERL_ERR"; then
    cat "$PERL_ERR" >&2 || true
    cat "$PERL_OUT" >&2 || true
    fail "perl query --json --jsonl round $round failed"
  fi

  NATIVE_T="$(extract_shared "$NATIVE_OUT" "native r$round")"
  PERL_T="$(extract_shared "$PERL_OUT" "perl r$round")"

  log "  native report --json:  $NATIVE_T"
  log "  perl query --jsonl:    $PERL_T"

  assert_shared_contract "$NATIVE_T" "native r$round"
  assert_shared_contract "$PERL_T" "perl r$round"

  # discount: require == 818 on golden/default-calls1, and equal across sides
  read -r _nleaf _nmid _nedge ndisc <<<"$NATIVE_T"
  read -r _pleaf _pmid _pedge pdisc <<<"$PERL_T"
  [[ "$ndisc" == "$WANT_DISCOUNT" ]] \
    || fail "native r$round: discount_events=$ndisc want $WANT_DISCOUNT"
  [[ "$pdisc" == "$WANT_DISCOUNT" ]] \
    || fail "perl r$round: discount_events=$pdisc want $WANT_DISCOUNT"

  assert_tuples_equal "$NATIVE_T" "$PERL_T" "round $round native vs perl"

  # Expand: sub_entry_events when both expose (default-calls1 → 0)
  assert_sub_entry_when_both "$NATIVE_OUT" "$PERL_OUT" \
    "default-calls1 r$round" "$WANT_SUB_ENTRY_DEFAULT"

  # TIMEBLOCK: time_block_events when both expose (default-calls1 → 0)
  assert_time_block_when_both "$NATIVE_OUT" "$PERL_OUT" \
    "default-calls1 r$round" "$WANT_TIME_BLOCK_DEFAULT"

  # COUNTS: event multiplicity 27/3/13/632/31 + file_1_basename when both expose
  assert_event_counts_when_both "$NATIVE_OUT" "$PERL_OUT" \
    "default-calls1 r$round"
  assert_file_basename_when_both "$NATIVE_OUT" "$PERL_OUT" \
    "default-calls1 r$round"

  # TOTAL: total_events 2474 + attribute_basetime when both expose
  assert_total_events_when_both "$NATIVE_OUT" "$PERL_OUT" \
    "default-calls1 r$round" "$WANT_TOTAL_EVENTS"
  assert_attribute_basetime_when_both "$NATIVE_OUT" "$PERL_OUT" \
    "default-calls1 r$round"

  # META: stream/PID + A9/A8 + greppable meta when both expose (pair ×2)
  META_FP="$(assert_meta_when_both "$NATIVE_OUT" "$PERL_OUT" \
    "default-calls1 meta r$round" "1")"
  if [[ -n "$META_FP" ]]; then
    log "  meta fingerprint: $META_FP"
    ok "default-calls1 r$round: stream/PID + A9/A8 + greppable meta equal native↔perl"
  else
    log "NOTE: default-calls1 r$round: stream/A9/A8 shared fields not fully on both sides; skip meta equal"
  fi

  NATIVE_SE="$(extract_optional_int "$NATIVE_OUT" "sub_entry_events" "native r$round se")"
  PERL_SE="$(extract_optional_int "$PERL_OUT" "sub_entry_events" "perl r$round se")"
  NATIVE_TB="$(extract_optional_int "$NATIVE_OUT" "time_block_events" "native r$round tb")"
  PERL_TB="$(extract_optional_int "$PERL_OUT" "time_block_events" "perl r$round tb")"
  NATIVE_SR="$(extract_optional_int "$NATIVE_OUT" "sub_return_events" "native r$round sr")"
  PERL_SR="$(extract_optional_int "$PERL_OUT" "sub_return_events" "perl r$round sr")"
  NATIVE_BN="$(extract_optional_str "$NATIVE_OUT" "file_1_basename" "native r$round bn")"
  PERL_BN="$(extract_optional_str "$PERL_OUT" "file_1_basename" "perl r$round bn")"
  NATIVE_TE="$(extract_optional_int "$NATIVE_OUT" "total_events" "native r$round te")"
  PERL_TE="$(extract_optional_int "$PERL_OUT" "total_events" "perl r$round te")"
  NATIVE_BT="$(extract_optional_str "$NATIVE_OUT" "attribute_basetime" "native r$round bt")"
  PERL_BT="$(extract_optional_str "$PERL_OUT" "attribute_basetime" "perl r$round bt")"

  if [[ -n "$PREV_NATIVE" ]]; then
    assert_tuples_equal "$PREV_NATIVE" "$NATIVE_T" "native consistency round1 vs r$round"
    assert_tuples_equal "$PREV_PERL" "$PERL_T" "perl consistency round1 vs r$round"
    if [[ -n "$PREV_NATIVE_SE" && -n "$NATIVE_SE" ]]; then
      [[ "$PREV_NATIVE_SE" == "$NATIVE_SE" ]] \
        || fail "native sub_entry_events consistency r1=$PREV_NATIVE_SE r$round=$NATIVE_SE"
    fi
    if [[ -n "$PREV_PERL_SE" && -n "$PERL_SE" ]]; then
      [[ "$PREV_PERL_SE" == "$PERL_SE" ]] \
        || fail "perl sub_entry_events consistency r1=$PREV_PERL_SE r$round=$PERL_SE"
    fi
    if [[ -n "$PREV_NATIVE_TB" && -n "$NATIVE_TB" ]]; then
      [[ "$PREV_NATIVE_TB" == "$NATIVE_TB" ]] \
        || fail "native time_block_events consistency r1=$PREV_NATIVE_TB r$round=$NATIVE_TB"
    fi
    if [[ -n "$PREV_PERL_TB" && -n "$PERL_TB" ]]; then
      [[ "$PREV_PERL_TB" == "$PERL_TB" ]] \
        || fail "perl time_block_events consistency r1=$PREV_PERL_TB r$round=$PERL_TB"
    fi
    if [[ -n "$PREV_NATIVE_SR" && -n "$NATIVE_SR" ]]; then
      [[ "$PREV_NATIVE_SR" == "$NATIVE_SR" ]] \
        || fail "native sub_return_events consistency r1=$PREV_NATIVE_SR r$round=$NATIVE_SR"
    fi
    if [[ -n "$PREV_PERL_SR" && -n "$PERL_SR" ]]; then
      [[ "$PREV_PERL_SR" == "$PERL_SR" ]] \
        || fail "perl sub_return_events consistency r1=$PREV_PERL_SR r$round=$PERL_SR"
    fi
    if [[ -n "$PREV_NATIVE_BN" && -n "$NATIVE_BN" ]]; then
      [[ "$PREV_NATIVE_BN" == "$NATIVE_BN" ]] \
        || fail "native file_1_basename consistency r1=$PREV_NATIVE_BN r$round=$NATIVE_BN"
    fi
    if [[ -n "$PREV_PERL_BN" && -n "$PERL_BN" ]]; then
      [[ "$PREV_PERL_BN" == "$PERL_BN" ]] \
        || fail "perl file_1_basename consistency r1=$PREV_PERL_BN r$round=$PERL_BN"
    fi
    if [[ -n "$PREV_NATIVE_TE" && -n "$NATIVE_TE" ]]; then
      [[ "$PREV_NATIVE_TE" == "$NATIVE_TE" ]] \
        || fail "native total_events consistency r1=$PREV_NATIVE_TE r$round=$NATIVE_TE"
    fi
    if [[ -n "$PREV_PERL_TE" && -n "$PERL_TE" ]]; then
      [[ "$PREV_PERL_TE" == "$PERL_TE" ]] \
        || fail "perl total_events consistency r1=$PREV_PERL_TE r$round=$PERL_TE"
    fi
    if [[ -n "$PREV_NATIVE_BT" && -n "$NATIVE_BT" ]]; then
      [[ "$PREV_NATIVE_BT" == "$NATIVE_BT" ]] \
        || fail "native attribute_basetime consistency r1=$PREV_NATIVE_BT r$round=$NATIVE_BT"
    fi
    if [[ -n "$PREV_PERL_BT" && -n "$PERL_BT" ]]; then
      [[ "$PREV_PERL_BT" == "$PERL_BT" ]] \
        || fail "perl attribute_basetime consistency r1=$PREV_PERL_BT r$round=$PERL_BT"
    fi
    if [[ -n "$PREV_META_FP" && -n "$META_FP" ]]; then
      [[ "$PREV_META_FP" == "$META_FP" ]] \
        || fail "meta fingerprint consistency r1=$PREV_META_FP r$round=$META_FP"
    fi
  fi
  PREV_NATIVE="$NATIVE_T"
  PREV_PERL="$PERL_T"
  PREV_NATIVE_SE="$NATIVE_SE"
  PREV_PERL_SE="$PERL_SE"
  PREV_NATIVE_TB="$NATIVE_TB"
  PREV_PERL_TB="$PERL_TB"
  PREV_NATIVE_SR="$NATIVE_SR"
  PREV_PERL_SR="$PERL_SR"
  PREV_NATIVE_BN="$NATIVE_BN"
  PREV_NATIVE_TE="$NATIVE_TE"
  PREV_PERL_TE="$PERL_TE"
  PREV_NATIVE_BT="$NATIVE_BT"
  PREV_PERL_BT="$PERL_BT"
  PREV_PERL_BN="$PERL_BN"
  PREV_META_FP="$META_FP"

  ok "round $round: shared fields equal ($NATIVE_T)${NATIVE_SE:+ sub_entry=$NATIVE_SE}${NATIVE_TB:+ time_block=$NATIVE_TB}${NATIVE_SR:+ sub_return=$NATIVE_SR}${NATIVE_BN:+ basename=$NATIVE_BN}${META_FP:+ meta=ok}"
done

ok "cross pair ×2: leaf=$WANT_LEAF mid=$WANT_MID edge=$WANT_EDGE discount=$WANT_DISCOUNT sub_entry(default)=$WANT_SUB_ENTRY_DEFAULT time_block(default)=$WANT_TIME_BLOCK_DEFAULT (when both expose); event counts 27/3/13/632/31 + file_1_basename when both expose; total_events=$WANT_TOTAL_EVENTS + attribute_basetime when both expose; stream/PID + A9/A8 + greppable meta when both expose"

# ---------------------------------------------------------------------------
# Optional: query --json via native dump of the live profile (default-calls1)
# ---------------------------------------------------------------------------
echo "=== optional: query --json via native dump of profile ==="
DUMP_OUT="$TMPDIR_SMOKE/query_profile_json.out"
DUMP_ERR="$TMPDIR_SMOKE/query_profile_json.err"
if "${ENGINE[@]}" query --json "$PROFILE_REL" \
  >"$DUMP_OUT" 2>"$DUMP_ERR"; then
  DUMP_T="$(extract_shared "$DUMP_OUT" "query --json profile")"
  log "  query --json profile:  $DUMP_T"
  assert_shared_contract "$DUMP_T" "query --json profile"
  read -r _dleaf _dmid _dedge ddisc <<<"$DUMP_T"
  # After dump-derived path: require equal to native side (and prefer 818).
  assert_tuples_equal "$PREV_NATIVE" "$DUMP_T" \
    "query --json profile vs native report --json"
  if [[ "$ddisc" != "$WANT_DISCOUNT" ]]; then
    # Equal to native already asserted; note if golden number drifted.
    log "NOTE: dump-derived discount_events=$ddisc (equal to native; golden $WANT_DISCOUNT)"
  fi
  # Prefer real native report --json out from last round for optional field
  LAST_NATIVE="$TMPDIR_SMOKE/native_r2.out"
  assert_sub_entry_when_both "$LAST_NATIVE" "$DUMP_OUT" \
    "query --json profile dump path" "$WANT_SUB_ENTRY_DEFAULT"
  assert_time_block_when_both "$LAST_NATIVE" "$DUMP_OUT" \
    "query --json profile dump path" "$WANT_TIME_BLOCK_DEFAULT"
  assert_event_counts_when_both "$LAST_NATIVE" "$DUMP_OUT" \
    "query --json profile dump path"
  assert_file_basename_when_both "$LAST_NATIVE" "$DUMP_OUT" \
    "query --json profile dump path"
  assert_total_events_when_both "$LAST_NATIVE" "$DUMP_OUT" \
    "query --json profile dump path" "$WANT_TOTAL_EVENTS"
  assert_attribute_basetime_when_both "$LAST_NATIVE" "$DUMP_OUT" \
    "query --json profile dump path"
  DUMP_META_FP="$(assert_meta_when_both "$LAST_NATIVE" "$DUMP_OUT" \
    "query --json profile dump meta" "1")"
  if [[ -n "$DUMP_META_FP" ]]; then
    log "  dump-path meta fingerprint: $DUMP_META_FP"
    if [[ -n "$PREV_META_FP" ]]; then
      [[ "$DUMP_META_FP" == "$PREV_META_FP" ]] \
        || fail "dump-path meta fingerprint vs pair: dump=$DUMP_META_FP pair=$PREV_META_FP"
    fi
    ok "query --json profile dump path: stream/PID + A9/A8 + greppable meta match native"
  fi
  ok "query --json profile: matches native shared fields ($DUMP_T)"
else
  # Fail soft only if engine could not find native for dump — still fail if
  # unexpected crash while CLI was present (we already resolved CLI above).
  cat "$DUMP_ERR" >&2 || true
  cat "$DUMP_OUT" >&2 || true
  # Engine dump path needs find_native_cli; with cargo-only mode, EngineDispatch
  # may still locate cargo. Treat failure as hard because we have native.
  fail "query --json profile (native dump path) failed while native CLI is available"
fi

# ---------------------------------------------------------------------------
# Expand: calls2-default sub_entry_events == 27 when both sides expose SUB_ENTRY
# (fixture-scoped: only the SUB_ENTRY count is asserted here, not leaf/mid/edge)
# ---------------------------------------------------------------------------
echo "=== NATIVE-QUERY-JSON-CROSS-EXPAND: calls2-default sub_entry_events ==="
if [[ -f "$ROOT/$CALLS2_PROFILE_REL" && -f "$ROOT/$CALLS2_GOLDEN_REL" ]]; then
  C2_NATIVE_OUT="$TMPDIR_SMOKE/calls2_native.out"
  C2_NATIVE_ERR="$TMPDIR_SMOKE/calls2_native.err"
  C2_PERL_OUT="$TMPDIR_SMOKE/calls2_perl.out"
  C2_PERL_ERR="$TMPDIR_SMOKE/calls2_perl.err"

  if ! "${CLI_CMD[@]}" report --json "$CALLS2_PROFILE_REL" \
    >"$C2_NATIVE_OUT" 2>"$C2_NATIVE_ERR"; then
    cat "$C2_NATIVE_ERR" >&2 || true
    cat "$C2_NATIVE_OUT" >&2 || true
    fail "native report --json calls2-default failed"
  fi

  if ! "${ENGINE[@]}" query --json --jsonl "$CALLS2_GOLDEN_REL" \
    >"$C2_PERL_OUT" 2>"$C2_PERL_ERR"; then
    cat "$C2_PERL_ERR" >&2 || true
    cat "$C2_PERL_OUT" >&2 || true
    fail "perl query --json --jsonl calls2-default failed"
  fi

  C2_N_SE="$(extract_optional_int "$C2_NATIVE_OUT" "sub_entry_events" "calls2 native")"
  C2_P_SE="$(extract_optional_int "$C2_PERL_OUT" "sub_entry_events" "calls2 perl")"
  log "  calls2 native sub_entry_events: ${C2_N_SE:-(absent)}"
  log "  calls2 perl   sub_entry_events: ${C2_P_SE:-(absent)}"

  if [[ -n "$C2_N_SE" && -n "$C2_P_SE" ]]; then
    [[ "$C2_N_SE" == "$C2_P_SE" ]] \
      || fail "calls2-default: sub_entry_events diverge native=$C2_N_SE perl=$C2_P_SE"
    [[ "$C2_N_SE" == "$WANT_SUB_ENTRY_CALLS2" ]] \
      || fail "calls2-default: sub_entry_events=$C2_N_SE want $WANT_SUB_ENTRY_CALLS2"
    ok "calls2-default: both sides sub_entry_events=$WANT_SUB_ENTRY_CALLS2"
  elif [[ -z "$C2_N_SE" && -z "$C2_P_SE" ]]; then
    log "NOTE: calls2-default: neither side exposes sub_entry_events (skip)"
  else
    log "NOTE: calls2-default: only one side exposes sub_entry_events (native=${C2_N_SE:-(absent)} perl=${C2_P_SE:-(absent)}); skip equal assert until JSON-SUB-ENTRY-MVP lands both sides"
  fi
else
  log "NOTE: calls2-default fixtures missing; skip SUB_ENTRY expand check"
  [[ -f "$ROOT/$CALLS2_PROFILE_REL" ]] || log "  missing $CALLS2_PROFILE_REL"
  [[ -f "$ROOT/$CALLS2_GOLDEN_REL" ]] || log "  missing $CALLS2_GOLDEN_REL"
fi

# ---------------------------------------------------------------------------
# Blocks (NATIVE-QUERY-JSON-CROSS-BLOCKS + CROSS-TIMEBLOCK):
# blocks-calls1 A4/A4b 780/810 + time_block_events 916 when both expose
# pair ×2 native report --json vs Perl query --json --jsonl
# ---------------------------------------------------------------------------
echo "=== NATIVE-QUERY-JSON-CROSS-BLOCKS: blocks-calls1 line/block calls + time_block ==="
[[ -f "$ROOT/$BLOCKS_PROFILE_REL" ]] || fail "missing fixture $BLOCKS_PROFILE_REL"
[[ -f "$ROOT/$BLOCKS_GOLDEN_REL" ]] || fail "missing golden dump $BLOCKS_GOLDEN_REL"

PREV_BLOCKS_NATIVE=""
PREV_BLOCKS_PERL=""
PREV_BLOCKS_NATIVE_TB=""
PREV_BLOCKS_PERL_TB=""

for round in 1 2; do
  echo "=== NATIVE-QUERY-JSON-CROSS-BLOCKS round $round (blocks-calls1) ==="

  B_NATIVE_OUT="$TMPDIR_SMOKE/blocks_native_r${round}.out"
  B_NATIVE_ERR="$TMPDIR_SMOKE/blocks_native_r${round}.err"
  B_PERL_OUT="$TMPDIR_SMOKE/blocks_perl_r${round}.out"
  B_PERL_ERR="$TMPDIR_SMOKE/blocks_perl_r${round}.err"

  if ! "${CLI_CMD[@]}" report --json "$BLOCKS_PROFILE_REL" \
    >"$B_NATIVE_OUT" 2>"$B_NATIVE_ERR"; then
    cat "$B_NATIVE_ERR" >&2 || true
    cat "$B_NATIVE_OUT" >&2 || true
    fail "native report --json blocks-calls1 round $round failed"
  fi

  if ! "${ENGINE[@]}" query --json --jsonl "$BLOCKS_GOLDEN_REL" \
    >"$B_PERL_OUT" 2>"$B_PERL_ERR"; then
    cat "$B_PERL_ERR" >&2 || true
    cat "$B_PERL_OUT" >&2 || true
    fail "perl query --json --jsonl blocks-calls1 round $round failed"
  fi

  B_NATIVE_T="$(extract_blocks "$B_NATIVE_OUT" "blocks native r$round")"
  B_PERL_T="$(extract_blocks "$B_PERL_OUT" "blocks perl r$round")"

  log "  native report --json:  line_calls_1_5 block_line_calls_1_4 = $B_NATIVE_T"
  log "  perl query --jsonl:    line_calls_1_5 block_line_calls_1_4 = $B_PERL_T"

  assert_blocks_contract "$B_NATIVE_T" "blocks native r$round"
  assert_blocks_contract "$B_PERL_T" "blocks perl r$round"

  if [[ "$B_NATIVE_T" != "$B_PERL_T" ]]; then
    fail "blocks-calls1 r$round: A4/A4b diverge
  native: $B_NATIVE_T  (line_calls_1_5 block_line_calls_1_4)
  perl:   $B_PERL_T  (line_calls_1_5 block_line_calls_1_4)"
  fi

  # CROSS-TIMEBLOCK: time_block_events == 916 when both expose
  assert_time_block_when_both "$B_NATIVE_OUT" "$B_PERL_OUT" \
    "blocks-calls1 r$round" "$WANT_TIME_BLOCK_BLOCKS"

  B_NATIVE_TB="$(extract_optional_int "$B_NATIVE_OUT" "time_block_events" "blocks native r$round tb")"
  B_PERL_TB="$(extract_optional_int "$B_PERL_OUT" "time_block_events" "blocks perl r$round tb")"

  if [[ -n "$PREV_BLOCKS_NATIVE" ]]; then
    [[ "$PREV_BLOCKS_NATIVE" == "$B_NATIVE_T" ]] \
      || fail "blocks native consistency r1=$PREV_BLOCKS_NATIVE r$round=$B_NATIVE_T"
    [[ "$PREV_BLOCKS_PERL" == "$B_PERL_T" ]] \
      || fail "blocks perl consistency r1=$PREV_BLOCKS_PERL r$round=$B_PERL_T"
    if [[ -n "$PREV_BLOCKS_NATIVE_TB" && -n "$B_NATIVE_TB" ]]; then
      [[ "$PREV_BLOCKS_NATIVE_TB" == "$B_NATIVE_TB" ]] \
        || fail "blocks native time_block consistency r1=$PREV_BLOCKS_NATIVE_TB r$round=$B_NATIVE_TB"
    fi
    if [[ -n "$PREV_BLOCKS_PERL_TB" && -n "$B_PERL_TB" ]]; then
      [[ "$PREV_BLOCKS_PERL_TB" == "$B_PERL_TB" ]] \
        || fail "blocks perl time_block consistency r1=$PREV_BLOCKS_PERL_TB r$round=$B_PERL_TB"
    fi
  fi
  PREV_BLOCKS_NATIVE="$B_NATIVE_T"
  PREV_BLOCKS_PERL="$B_PERL_T"
  PREV_BLOCKS_NATIVE_TB="$B_NATIVE_TB"
  PREV_BLOCKS_PERL_TB="$B_PERL_TB"

  ok "blocks r$round: line_calls_1_5=$WANT_LINE_CALLS_1_5 block_line_calls_1_4=$WANT_BLOCK_LINE_CALLS_1_4 equal native↔perl${B_NATIVE_TB:+ time_block=$B_NATIVE_TB}"
done

ok "cross blocks pair ×2: line_calls_1_5=$WANT_LINE_CALLS_1_5 block_line_calls_1_4=$WANT_BLOCK_LINE_CALLS_1_4 time_block(when both)=$WANT_TIME_BLOCK_BLOCKS"

# Final PERL5LIB guard (children should not have polluted parent)
if [[ "${PERL5LIB:-}" == *"/crates/"* ]]; then
  fail "PERL5LIB must not contain /crates/ after run (got: $PERL5LIB)"
fi

ok "native_query_json_cross_smoke (NATIVE-QUERY-JSON-CROSS-EXPAND + CROSS-BLOCKS + CROSS-META + CROSS-TIMEBLOCK + CROSS-COUNTS + CROSS-TOTAL) completed successfully"
exit 0
