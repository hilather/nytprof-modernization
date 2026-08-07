#!/usr/bin/env bash
# Shared paths for BASE-001 oracle pin. Oracle builds must never require
# candidate crates/ or perl/ trees on PERL5LIB.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NYTPROF_MOD_ROOT="$ROOT"

BASELINE_DIR="$ROOT/baseline/6.15"
export BASELINE_DIR
SRC_DIR="$BASELINE_DIR/src"
export SRC_DIR
INSTALL_DIR="$BASELINE_DIR/install"
export INSTALL_DIR
ARCHIVE_DIR="$BASELINE_DIR/archives"
export ARCHIVE_DIR
LOG_DIR="$BASELINE_DIR/logs"
export LOG_DIR

# Pinned upstream identity (SOURCES.md / plan SOURCES)
export ORACLE_TAG="v6.15"
export ORACLE_COMMIT_FULL="7578f4b"  # short pin from plan; full hash resolved at fetch
export ORACLE_REPO_URL="https://github.com/timbunce/devel-nytprof.git"
export ORACLE_TARBALL_URL="https://cpan.metacpan.org/authors/id/T/TI/TIMB/Devel-NYTProf-6.15.tar.gz"
# Fallback GitHub archive if CPAN is unreachable
export ORACLE_GITHUB_ARCHIVE_URL="https://github.com/timbunce/devel-nytprof/archive/refs/tags/v6.15.tar.gz"

mkdir -p "$BASELINE_DIR" "$ARCHIVE_DIR" "$LOG_DIR" "$INSTALL_DIR"
