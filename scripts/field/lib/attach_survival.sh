# Shared attach-survival helpers. Sourced by the complex-app lab/smoke.
# Fail closed on wrap/import/XSLoader-class kills — not exclusive times.

attach_kill_pattern() {
  printf '%s' 'as an ARRAY ref|EndOfScope/XS.pm|loadable object for module DB|Global symbol "\$VERSION"|heavy_(eval)'
}

# Usage: attach_fail_if_killed FILE
# Exit 1 if FILE contains a known attach-kill string.
attach_fail_if_killed() {
  local f="$1"
  [[ -f "$f" ]] || return 0
  if grep -E -q "$(attach_kill_pattern)" "$f"; then
    printf 'ERROR: attach-kill string in %s\n' "$f" >&2
    grep -E -n "$(attach_kill_pattern)" "$f" | head -20 >&2 || true
    return 1
  fi
  return 0
}

# Usage: attach_require_token FILE TOKEN
attach_require_token() {
  local f="$1" tok="$2"
  [[ -f "$f" ]] || {
    printf 'ERROR: missing profiled output %s\n' "$f" >&2
    return 1
  }
  grep -F -q "$tok" "$f" || {
    printf 'ERROR: %s missing success token %s\n' "$f" "$tok" >&2
    return 1
  }
  return 0
}

# Usage: attach_require_nytprof5 FILE
attach_require_nytprof5() {
  local f="$1"
  [[ -s "$f" ]] || {
    printf 'ERROR: missing profile %s\n' "$f" >&2
    return 1
  }
  head -c 9 "$f" | grep -q 'NYTProf 5' || {
    printf 'ERROR: %s is not NYTProf 5\n' "$f" >&2
    return 1
  }
  return 0
}
