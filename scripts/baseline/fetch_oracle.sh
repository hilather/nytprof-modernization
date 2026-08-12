#!/usr/bin/env bash
# Fetch and pin Devel::NYTProf 6.15 source (BASE-001).
set -euo pipefail
# shellcheck source=common.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

mkdir -p "$ARCHIVE_DIR" "$SRC_DIR"
cd "$ARCHIVE_DIR"

TARBALL="Devel-NYTProf-6.15.tar.gz"
if [[ ! -f "$TARBALL" ]]; then
  echo "Fetching $ORACLE_TARBALL_URL ..."
  if ! curl -fsSL -o "$TARBALL" "$ORACLE_TARBALL_URL"; then
    echo "CPAN fetch failed; trying GitHub tag archive..."
    curl -fsSL -o "$TARBALL" "$ORACLE_GITHUB_ARCHIVE_URL"
  fi
fi

# Portable hash (sha256sum on Linux; shasum/openssl/python3 on macOS GHA).
SHA256="$(sha256_file "$TARBALL")"
echo "$SHA256  $TARBALL" | tee "$ARCHIVE_DIR/SHA256SUMS"
echo "Archive SHA-256: $SHA256"

# Prefer checked-in pin when present (BASE-001 integrity); fail closed on mismatch.
PIN_FILE="$BASELINE_DIR/oracle-archive.sha256"
if [[ -f "$PIN_FILE" ]]; then
  PIN_SHA="$(tr -d '[:space:]' <"$PIN_FILE")"
  if [[ -n "$PIN_SHA" && "$SHA256" != "$PIN_SHA" ]]; then
    echo "ERROR: archive SHA-256 mismatch vs $PIN_FILE" >&2
    echo "  expected: $PIN_SHA" >&2
    echo "  actual:   $SHA256" >&2
    echo "  tarball:  $ARCHIVE_DIR/$TARBALL" >&2
    exit 1
  fi
  echo "Archive SHA-256 matches pin $PIN_FILE"
fi

# Extract into a clean src tree
rm -rf "$SRC_DIR"
mkdir -p "$SRC_DIR"
tar -xzf "$ARCHIVE_DIR/$TARBALL" -C "$SRC_DIR" --strip-components=1

# Prefer git metadata when the tree is a clone; for tarball, record tag only
if [[ -d "$SRC_DIR/.git" ]]; then
  FULL_COMMIT="$(git -C "$SRC_DIR" rev-parse HEAD)"
else
  # Resolve full commit from GitHub API for the tag when possible
  FULL_COMMIT=""
  if API_JSON="$(curl -fsSL "https://api.github.com/repos/timbunce/devel-nytprof/git/refs/tags/${ORACLE_TAG}" 2>/dev/null || true)"; then
    FULL_COMMIT="$(printf '%s' "$API_JSON" | python3 -c 'import sys,json; d=json.load(sys.stdin); print(d.get("object",{}).get("sha",""))' 2>/dev/null || true)"
  fi
  if [[ -z "$FULL_COMMIT" ]]; then
    FULL_COMMIT="$ORACLE_COMMIT_FULL"
  fi
fi

printf '%s\n' "$FULL_COMMIT" > "$BASELINE_DIR/oracle-commit.txt"
printf '%s\n' "$ORACLE_TAG" > "$BASELINE_DIR/oracle-tag.txt"
printf '%s\n' "$SHA256" > "$BASELINE_DIR/oracle-archive.sha256"

# Prove key source files exist
test -f "$SRC_DIR/NYTProf.xs"
test -f "$SRC_DIR/FileHandle.h"
test -f "$SRC_DIR/Makefile.PL"
test -f "$SRC_DIR/lib/Devel/NYTProf.pm"

VERSION_PM="$(perl -ne 'print $1 if /VERSION\s*=\s*['\''"]?([0-9.]+)/' "$SRC_DIR/lib/Devel/NYTProf.pm" | head -1)"
echo "Source tree ready: $SRC_DIR (\$VERSION from pm ≈ ${VERSION_PM:-unknown})"
echo "Pin commit recorded: $(cat "$BASELINE_DIR/oracle-commit.txt")"
