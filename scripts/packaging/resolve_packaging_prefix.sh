#!/usr/bin/env bash
# Shared packaging install-root resolution (BUILD-003-DEPTH).
#
# Sourced by install_native.sh and install_facade.sh so dual-install always
# lands CLI + facade under the *same* root (no split heuristics).
#
# Resolution order:
#   1. NYTPROF_PREFIX (always wins — preferred under MakeMaker recipes)
#   2. bare PREFIX only when not a MakeMaker/local::lib-style denylist path
#   3. default $ROOT/prefix
#
# Denylist for bare PREFIX (after stripping one trailing slash):
#   - exact $HOME/perl5 (local::lib / PERL_MM_OPT INSTALL_BASE default)
#   - any path ending in /perl5 (MakeMaker-style install bases elsewhere)
#
# Usage (from another packaging script, after ROOT is set):
#   # shellcheck source=resolve_packaging_prefix.sh
#   source "$ROOT/scripts/packaging/resolve_packaging_prefix.sh"
#   PREFIX="$(resolve_packaging_prefix "$ROOT")"
#
# Never puts crates/ on oracle PERL5LIB.

# Resolve install root for packaging scripts. Prints absolute-ish path to stdout.
# $1 = repo root (required).
resolve_packaging_prefix() {
  local root="${1:-}"
  if [[ -z "$root" ]]; then
    echo "resolve_packaging_prefix: ROOT required" >&2
    return 2
  fi

  local candidate=""
  if [[ -n "${NYTPROF_PREFIX:-}" ]]; then
    candidate="${NYTPROF_PREFIX}"
    # Operator-chosen NYTPROF_PREFIX always wins (including intentional */perl5).
    candidate="${candidate%/}"
    if [[ -z "$candidate" ]]; then
      candidate="$root/prefix"
    fi
    printf '%s\n' "$candidate"
    return 0
  fi

  if [[ -n "${PREFIX:-}" ]]; then
    candidate="${PREFIX}"
    candidate="${candidate%/}"
    local home_perl5="${HOME:-}/perl5"
    home_perl5="${home_perl5%/}"

    # MakeMaker / local::lib denylist — identical for native + facade.
    if [[ -n "$candidate" && "$candidate" != "$home_perl5" && "$candidate" != */perl5 ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  fi

  printf '%s\n' "$root/prefix"
  return 0
}
