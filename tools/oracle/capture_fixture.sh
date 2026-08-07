#!/usr/bin/env bash
# Capture a golden v5 profile + ReadStream dump using the pinned oracle.
# Usage: capture_fixture.sh <fixture_name> [NYTPROF options without file=]
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=env.sh
source "$DIR/env.sh"

NAME="${1:?fixture name required}"
OPTS="${2:-trace=0:start=begin:calls=1}"
ROOT="$NYTPROF_MOD_ROOT"
OUT_DIR="$ROOT/fixtures/v5/$NAME"
mkdir -p "$OUT_DIR"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

cat >"$WORK/workload.pl" <<'PERL'
use strict;
use warnings;
sub leaf {
    my $x = 0;
    $x++ for 1 .. 50;
    return $x;
}
sub mid {
    my $s = 0;
    $s += leaf() for 1 .. 5;
    return $s;
}
my $total = 0;
$total += mid() for 1 .. 3;
print "total=$total\n";
PERL

PROFILE="$OUT_DIR/nytprof.out"
rm -f "$PROFILE"
export NYTPROF="${OPTS}:file=${PROFILE}"

echo "Capturing with NYTPROF=$NYTPROF"
# Profiler is enabled via -d:NYTProf (not merely by loading the .pm)
perl -d:NYTProf "$WORK/workload.pl" >"$OUT_DIR/workload.stdout"

if [[ ! -s "$PROFILE" ]]; then
  echo "ERROR: profile not created at $PROFILE" >&2
  ls -la "$OUT_DIR" >&2 || true
  exit 1
fi

cp "$WORK/workload.pl" "$OUT_DIR/workload.pl"

perl "$DIR/dump_readstream.pl" "$PROFILE" >"$OUT_DIR/readstream.jsonl"
printf '%s\n' "$NYTPROF_ORACLE_MODULE" >"$OUT_DIR/oracle-module-path.txt"

(
  cd "$OUT_DIR"
  sha256sum nytprof.out readstream.jsonl workload.pl > SHA256SUMS
)

python3 - <<PY
import json, hashlib, os
from pathlib import Path
from datetime import datetime, timezone
out = Path("$OUT_DIR")
def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()
meta = {
  "name": "$NAME",
  "created_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
  "nytprof_env": os.environ.get("NYTPROF"),
  "oracle_module": Path("$OUT_DIR/oracle-module-path.txt").read_text().strip(),
  "files": {
    "nytprof.out": sha(out/"nytprof.out"),
    "readstream.jsonl": sha(out/"readstream.jsonl"),
    "workload.pl": sha(out/"workload.pl"),
  },
  "event_count_approx": sum(1 for _ in open(out/"readstream.jsonl")),
}
(out/"fixture.json").write_text(json.dumps(meta, indent=2)+"\n")
print(json.dumps(meta, indent=2))
PY

echo "Fixture written: $OUT_DIR"
