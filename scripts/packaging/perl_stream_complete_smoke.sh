#!/usr/bin/env bash
# Pure-Perl JsonlData stream-completeness smoke (PERL-STREAM-COMPLETE).
# shellcheck shell=bash
#
# Spec: docs/schemas/perl-jsonl-data-mvp-v0.md
# Contract: docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md
#
# 1. Prove golden + incomplete craft: default-calls1 complete; filtered
#    header-only incomplete → is_stream_complete false, reasons non-empty
# 2. Explicit operator evidence print (TL/TB/pid counters + complete flag)
#
# Does NOT put crates/ on oracle PERL5LIB. Does NOT require oracle Devel::NYTProf.
# Does NOT invent incomplete event semantics — filters real golden dump lines.
#
# Usage (from repo root or any cwd):
#   ./scripts/packaging/perl_stream_complete_smoke.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

FIXTURE_DIR="fixtures/v5/default-calls1"
GOLDEN="$FIXTURE_DIR/readstream.jsonl"
PM="perl/lib/Devel/NYTProf/JsonlData.pm"
STREAM_PM="perl/lib/Devel/NYTProf/JsonlReadStream.pm"
T="perl/t/jsonl_data_stream_complete_default_calls1.t"
LIB="perl/lib"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

[[ -f "$ROOT/Cargo.toml" ]] || fail "missing Cargo.toml"
[[ -f "$ROOT/$GOLDEN" ]] || fail "missing golden dump $GOLDEN"
[[ -f "$ROOT/$PM" ]] || fail "missing $PM"
[[ -f "$ROOT/$STREAM_PM" ]] || fail "missing $STREAM_PM"
[[ -f "$ROOT/$T" ]] || fail "missing $T"

# ---------------------------------------------------------------------------
# 1. Pure-Perl test against committed golden + crafted incomplete (real lines)
# ---------------------------------------------------------------------------
echo "=== JsonlData stream-complete: prove default-calls1 ==="
prove -I"$LIB" "$T" || fail "prove $T failed"
ok "prove $T (complete golden + incomplete craft)"

# Explicit aggregation print for operator evidence
AGG_OUT="$(
  perl -I"$LIB" -MDevel::NYTProf::JsonlData -e '
    my $d = Devel::NYTProf::JsonlData->from_jsonl(shift);
    my $reasons = $d->stream_incompleteness_reasons;
    printf "is_stream_complete=%d\n", $d->is_stream_complete ? 1 : 0;
    printf "time_line_events=%d\n",  $d->time_line_events;
    printf "time_block_events=%d\n", $d->time_block_events;
    printf "pid_start_events=%d\n",  $d->pid_start_events;
    printf "pid_end_events=%d\n",    $d->pid_end_events;
    printf "reasons_count=%d\n",     scalar @$reasons;
    printf "reasons=%s\n", join("|", @$reasons);
    printf "records_seen=%d\n",      $d->records_seen;
  ' "$GOLDEN"
)"
echo "$AGG_OUT"
echo "$AGG_OUT" | grep -qE '^is_stream_complete=1$' \
  || fail "golden is_stream_complete != 1"
echo "$AGG_OUT" | grep -qE '^reasons_count=0$' \
  || fail "golden reasons_count != 0"
echo "$AGG_OUT" | grep -qE '^pid_start_events=[1-9]' \
  || fail "golden pid_start_events < 1"
echo "$AGG_OUT" | grep -qE '^pid_end_events=[1-9]' \
  || fail "golden pid_end_events < 1"
# TL+TB must be positive (default-calls1 uses TIME_LINE)
TL="$(echo "$AGG_OUT" | sed -n 's/^time_line_events=//p' | head -1)"
TB="$(echo "$AGG_OUT" | sed -n 's/^time_block_events=//p' | head -1)"
[[ -n "$TL" && -n "$TB" ]] || fail "missing TL/TB counters in evidence"
SUM=$(( TL + TB ))
[[ "$SUM" -gt 0 ]] || fail "golden TIME_LINE+TIME_BLOCK == 0"
ok "golden JsonlData: complete TL=$TL TB=$TB pid balanced reasons empty"

# ---------------------------------------------------------------------------
# 2. Incomplete craft from real golden (header tags only)
# ---------------------------------------------------------------------------
TMPDIR_SMOKE="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_SMOKE"' EXIT
INC_JSONL="$TMPDIR_SMOKE/incomplete_header.jsonl"

# Keep only VERSION/ATTRIBUTE/OPTION/COMMENT/START_DEFLATE from real dump
perl -e '
  use strict; use warnings;
  my ($src, $dst) = @ARGV;
  open my $in, "<:encoding(UTF-8)", $src or die $!;
  open my $out, ">:encoding(UTF-8)", $dst or die $!;
  my %ok = map { $_ => 1 } qw(VERSION ATTRIBUTE OPTION COMMENT START_DEFLATE);
  my $n = 0;
  while (my $line = <$in>) {
    chomp $line;
    next unless length $line;
    next unless $line =~ /"tag"\s*:\s*"([^"]+)"/;
    next unless $ok{$1};
    print {$out} $line, "\n";
    $n++;
  }
  close $in; close $out;
  die "kept 0 lines from $src\n" unless $n > 0;
  print STDERR "kept $n header lines\n";
' "$GOLDEN" "$INC_JSONL"

INC_OUT="$(
  perl -I"$LIB" -MDevel::NYTProf::JsonlData -e '
    my $d = Devel::NYTProf::JsonlData->from_jsonl(shift);
    my $reasons = $d->stream_incompleteness_reasons;
    printf "is_stream_complete=%d\n", $d->is_stream_complete ? 1 : 0;
    printf "time_line_events=%d\n",  $d->time_line_events;
    printf "time_block_events=%d\n", $d->time_block_events;
    printf "pid_start_events=%d\n",  $d->pid_start_events;
    printf "pid_end_events=%d\n",    $d->pid_end_events;
    printf "reasons_count=%d\n",     scalar @$reasons;
    printf "reasons=%s\n", join("|", @$reasons);
  ' "$INC_JSONL"
)"
echo "$INC_OUT"
echo "$INC_OUT" | grep -qE '^is_stream_complete=0$' \
  || fail "incomplete is_stream_complete != 0"
echo "$INC_OUT" | grep -qE '^reasons_count=[1-9]' \
  || fail "incomplete reasons_count < 1"
echo "$INC_OUT" | grep -q 'no statement timing' \
  || fail "incomplete reasons missing timing message"
echo "$INC_OUT" | grep -qE '^time_line_events=0$' \
  || fail "incomplete should have time_line_events=0"
ok "incomplete craft: is_stream_complete=0 reasons non-empty (real header lines)"

ok "perl JsonlData stream-complete packaging smoke passed"
exit 0
