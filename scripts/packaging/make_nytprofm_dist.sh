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
         "$STAGE/perl/bin" \
         "$STAGE/perl/lib/Devel/NYTProf" \
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
cp -a "$ROOT/t/installed_scripts.t" "$STAGE/t/installed_scripts.t"
cp -a "$ROOT/t/nytprof_v5_tag_table.inc" "$STAGE/t/nytprof_v5_tag_table.inc"

# I03 report wrappers + EngineDispatch (module RPM owns /usr/bin/nytprofhtml).
for wrap in nytprof-engine nytprofhtml nytprofcsv nytprofcg; do
  cp -a "$ROOT/perl/bin/$wrap" "$STAGE/perl/bin/$wrap"
done
for pm in EngineDispatch.pm JsonlData.pm JsonlReadStream.pm LegacyBridge.pm \
          Data.pm ReadStream.pm; do
  [[ -f "$ROOT/perl/lib/Devel/NYTProf/$pm" ]] \
    || { echo "ERROR: missing perl/lib/Devel/NYTProf/$pm" >&2; exit 1; }
  cp -a "$ROOT/perl/lib/Devel/NYTProf/$pm" "$STAGE/perl/lib/Devel/NYTProf/$pm"
done

PREBUILT="$ROOT/packaging/prebuilt/el8-x86_64/nytprof-cli"
[[ -x "$PREBUILT" ]] \
  || { echo "ERROR: missing EL8 prebuilt $PREBUILT (run scripts/packaging/build_el8_nytprof_cli.sh)" >&2; exit 1; }
mkdir -p "$STAGE/prebuilt/el8-x86_64"
cp -a "$PREBUILT" "$STAGE/prebuilt/el8-x86_64/nytprof-cli"
cp -a "$PREBUILT" "$STAGE/perl/bin/nytprof-cli"
chmod 755 "$STAGE/prebuilt/el8-x86_64/nytprof-cli" "$STAGE/perl/bin/nytprof-cli"
if [[ -f "$ROOT/packaging/prebuilt/el8-x86_64/README.md" ]]; then
  cp -a "$ROOT/packaging/prebuilt/el8-x86_64/README.md" \
    "$STAGE/prebuilt/el8-x86_64/README.md"
fi

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
