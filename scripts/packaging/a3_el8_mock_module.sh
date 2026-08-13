#!/usr/bin/env bash
# A3 — maintainer-mock perl-NYTProfM on rocky+epel-8-x86_64 when mock is usable.
#
# SKIP (exit 0) if mock is absent or unusable. RED (exit 1) only after
# --init succeeded and rebuild/%check/layout/cargo-invoke failed.
# First --rebuild is online (dnf builddep). Never crates.io.
# Never puts crates/ on oracle PERL5LIB.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

SPEC="$ROOT/packaging/rpm/perl-NYTProfM.spec"
DIST="$ROOT/scripts/packaging/make_nytprofm_dist.sh"
MOCK_ROOT="${NYTPROF_MOCK_ROOT:-rocky+epel-8-x86_64}"
LOGDIR="${NYTPROF_MOCK_LOGDIR:-$ROOT/var/mock-a3}"
TIMEOUT_SEC="${NYTPROF_MOCK_TIMEOUT:-2700}"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }
skip() { printf 'SKIP: %s\n' "$*"; exit 0; }

usage() {
  cat <<'EOF'
Usage: a3_el8_mock_module.sh

A3: mock --init then --rebuild perl-NYTProfM. SKIP if mock absent/unusable.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    *) fail2 "unknown flag: $1" ;;
  esac
done

echo "a3_el8_mock_module: repo root $ROOT"
echo "collection_default remains v5; dual_path stays oracle-primary (S2 not claimed)"
echo "never crates/ on PERL5LIB; first rebuild is online BaseOS+AppStream"

if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*) fail2 "PERL5LIB must not contain /crates/: $PERL5LIB" ;;
  esac
fi

[[ -f "$SPEC" ]] || fail "missing $SPEC"
[[ -x "$DIST" ]] || fail "missing $DIST"

if ! command -v mock >/dev/null 2>&1; then
  skip "mock not installed — not maintainer-mock certified"
fi

pick_cfg() {
  local name="$1"
  if [[ -f "/etc/mock/${name}.cfg" ]]; then
    printf '%s\n' "$name"
    return 0
  fi
  return 1
}

CFG="$MOCK_ROOT"
if ! pick_cfg "$CFG" >/dev/null; then
  if pick_cfg rocky-8-x86_64 >/dev/null; then
    echo "mock cfg $CFG missing; falling back to rocky-8-x86_64"
    CFG=rocky-8-x86_64
  else
    skip "mock unusable (no /etc/mock/${MOCK_ROOT}.cfg or rocky-8-x86_64.cfg)"
  fi
fi
echo "mock chroot: $CFG"

if ! id -nG 2>/dev/null | grep -E -q '(^|[[:space:]])mock([[:space:]]|$)'; then
  if ! mock --help >/dev/null 2>&1; then
    skip "mock unusable (not in mock group and mock --help failed)"
  fi
fi

echo "running: mock -r $CFG --init (network allowed)"
set +e
INIT_OUT="$(timeout "$TIMEOUT_SEC" mock -r "$CFG" --init 2>&1)"
INIT_RC=$?
set -e
printf '%s\n' "$INIT_OUT" | tail -n 40
if [[ "$INIT_RC" -ne 0 ]]; then
  skip "mock unusable (--init failed rc=$INIT_RC; group/nspawn/namespace)"
fi
ok "mock --init succeeded; rebuild may go red"

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/nytprof-a3-XXXXXX")"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT
mkdir -p "$WORKDIR/sources" "$LOGDIR"

echo "running: make_nytprofm_dist.sh $WORKDIR/sources"
bash "$DIST" "$WORKDIR/sources" || fail "make_nytprofm_dist.sh failed"
[[ -f "$WORKDIR/sources/NYTProfM-6.15.tar.gz" ]] \
  || fail "Source0 NYTProfM-6.15.tar.gz missing after make dist"

echo "running: mock -r $CFG --buildsrpm"
set +e
BSRPM_OUT="$(timeout "$TIMEOUT_SEC" mock -r "$CFG" --buildsrpm \
  --spec "$SPEC" --sources "$WORKDIR/sources" 2>&1)"
BSRPM_RC=$?
set -e
printf '%s\n' "$BSRPM_OUT" | tail -n 40
[[ "$BSRPM_RC" -eq 0 ]] || fail "mock --buildsrpm failed (rc=$BSRPM_RC)"

SRPM="$(find /var/lib/mock/"$CFG"/result -name 'perl-NYTProfM-6.15-*.src.rpm' 2>/dev/null | head -n 1 || true)"
if [[ -z "$SRPM" || ! -f "$SRPM" ]]; then
  SRPM="$(find "$HOME" -name 'perl-NYTProfM-6.15-*.src.rpm' 2>/dev/null | head -n 1 || true)"
fi
[[ -n "$SRPM" && -f "$SRPM" ]] || fail "could not find perl-NYTProfM-6.15 src.rpm after --buildsrpm"

REBUILD=(mock -r "$CFG" --rebuild "$SRPM")
if [[ "${NYTPROF_MOCK_OFFLINE:-0}" == "1" ]]; then
  echo "NYTPROF_MOCK_OFFLINE=1 — --offline --rebuild (warm cache only)"
  REBUILD=(mock -r "$CFG" --offline --rebuild "$SRPM")
else
  echo "first rebuild online (dnf builddep BaseOS+AppStream; no crates.io)"
fi

set +e
REB_OUT="$(timeout "$TIMEOUT_SEC" "${REBUILD[@]}" 2>&1)"
REB_RC=$?
set -e
printf '%s\n' "$REB_OUT" | tail -n 80
if [[ "$REB_RC" -eq 124 ]]; then
  fail "mock timed out after ${TIMEOUT_SEC}s"
fi
[[ "$REB_RC" -eq 0 ]] || fail "mock --rebuild failed (rc=$REB_RC)"

copy_logs() {
  local src
  for src in /var/lib/mock/"$CFG"/result/root.log \
             /var/lib/mock/"$CFG"/result/build.log; do
    if [[ -f "$src" ]]; then
      cp -f "$src" "$LOGDIR/"
    fi
  done
  echo "mock logs copied to $LOGDIR"
}
copy_logs

ROOTLOG="$LOGDIR/root.log"
BUILDLOG="$LOGDIR/build.log"
if [[ -f "$ROOTLOG" ]]; then
  if grep -E '(^|[[:space:]])(Installing|Upgrading|Installed)[[:space:]].*(^|[[:space:]])(cargo|rustc|rustup)(-|[[:space:]]|$)' "$ROOTLOG" \
      || grep -E '(^|[[:space:]])(cargo|rustc|rustup)-[0-9]' "$ROOTLOG"; then
    fail "root.log shows cargo/rustc/rustup package install"
  fi
fi
if [[ -f "$BUILDLOG" ]]; then
  if grep -E '^(Executing|Building|make|[[:space:]]*/usr/bin/)' "$BUILDLOG" \
      | grep -E '(^|[[:space:]/=])(cargo|rustc|rustup)([[:space:]]|$)'; then
    fail "build.log invokes cargo/rustc/rustup"
  fi
fi
ok "no cargo/rustc/rustup install or invoke in mock logs"

ok "A3 maintainer-mock rebuild"
exit 0
