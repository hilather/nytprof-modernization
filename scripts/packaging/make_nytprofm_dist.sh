#!/usr/bin/env bash
# RPM-01 / PR-A1 — stage a clean NYTProfM-6.15 tree and tar it for EL8 %setup.
#
# Does NOT grow root Makefile.PL PM/OBJECT (that is BUILD-003-FULL / DI-11).
# Does NOT include baseline/, crates/, target/, prefix/, collector/build/, fixtures/.
# %build of the module RPM is still: make -C collector xs-nytprof
#
# Usage: make_nytprofm_dist.sh [DEST_DIR]
# Writes DEST_DIR/NYTProfM-6.15.tar.gz (default: $REPO/dist).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEST="${1:-$ROOT/dist}"
NAME="NYTProfM-6.15"
STAGE_PARENT="$(mktemp -d "${TMPDIR:-/tmp}/nytprofm-dist.XXXXXX")"
STAGE="$STAGE_PARENT/$NAME"

cleanup() { rm -rf "$STAGE_PARENT"; }
trap cleanup EXIT

mkdir -p "$STAGE/collector/include" \
         "$STAGE/collector/src" \
         "$STAGE/collector/xs/Devel/NYTProfM" \
         "$STAGE/t"

cp -a "$ROOT/collector/Makefile" "$STAGE/collector/Makefile"
cp -a "$ROOT/collector/include/." "$STAGE/collector/include/"
cp -a "$ROOT/collector/src/." "$STAGE/collector/src/"
cp -a "$ROOT/collector/xs/NYTProf.xs" "$STAGE/collector/xs/NYTProf.xs"
cp -a "$ROOT/collector/xs/Devel/NYTProfM.pm" "$STAGE/collector/xs/Devel/NYTProfM.pm"
cp -a "$ROOT/collector/xs/Devel/NYTProfM/Core.pm" "$STAGE/collector/xs/Devel/NYTProfM/Core.pm"
if [[ -f "$ROOT/collector/README.md" ]]; then
  cp -a "$ROOT/collector/README.md" "$STAGE/collector/README.md"
fi

cp -a "$ROOT/Changes" "$STAGE/Changes"
cp -a "$ROOT/t/workload-calls1.pl" "$STAGE/t/workload-calls1.pl"
cp -a "$ROOT/t/installed_attach.t" "$STAGE/t/installed_attach.t"

# Minimal facade: identity only. %build does not run this for XS.
cat > "$STAGE/Makefile.PL" <<'PL'
use strict;
use warnings;
use ExtUtils::MakeMaker;
WriteMakefile(
    NAME      => 'Devel::NYTProfM',
    DISTNAME  => 'NYTProfM',
    VERSION   => '6.15',
    ABSTRACT  => 'NYTProfM 6.15 collection (D1-B staged dist; not BUILD-003-FULL)',
    AUTHOR    => 'nytprof-modernization',
    LICENSE   => 'perl',
    PM        => {},
);
PL

# Refuse to ship pin / crates / build trees.
if find "$STAGE" \( -path '*/baseline/*' -o -path '*/crates/*' -o -path '*/target/*' \
     -o -path '*/prefix/*' -o -path '*/collector/build/*' -o -path '*/fixtures/*' \) -print -quit \
     | grep -q .; then
  echo "ERROR: staged tree leaked baseline/crates/target/prefix/build/fixtures" >&2
  exit 1
fi

mkdir -p "$DEST"
OUT="$DEST/$NAME.tar.gz"
tar -C "$STAGE_PARENT" -czf "$OUT" "$NAME"
echo "$OUT"
