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

# Portable SHA-256 hex digest of a file (Linux GHA has sha256sum; macOS has shasum).
# Used by fetch_oracle and other baseline helpers so BUILD-006-MVP macOS runners work
# without brew coreutils. Prefer: sha256sum → shasum -a 256 → openssl → python3.
sha256_file() {
  local f="$1"
  if [[ ! -f "$f" ]]; then
    echo "ERROR: sha256_file: not a file: $f" >&2
    return 1
  fi
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$f" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$f" | awk '{print $1}'
  elif command -v openssl >/dev/null 2>&1; then
    # "SHA256(file)= hex" or "SHA2-256(file)= hex" depending on openssl version
    openssl dgst -sha256 "$f" | awk '{print $NF}'
  elif command -v python3 >/dev/null 2>&1; then
    python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$f"
  else
    echo "ERROR: no SHA-256 tool found (need sha256sum, shasum, openssl, or python3)" >&2
    return 1
  fi
}

mkdir -p "$BASELINE_DIR" "$ARCHIVE_DIR" "$LOG_DIR" "$INSTALL_DIR"
