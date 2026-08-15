#!/usr/bin/env bash
# Build unsigned perl-NYTProfM RPM for Rocky 8 / EL8 (test-drive).
#
# On EL8: rpmbuild natively.
# Elsewhere: re-exec inside rockylinux:8 (docker).
#
# Usage:
#   ./scripts/packaging/build_el8_module_rpm.sh [OUTDIR]
#   ./scripts/packaging/build_el8_module_rpm.sh --native [OUTDIR]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

NATIVE=0
if [[ "${1:-}" == "--native" ]]; then
  NATIVE=1
  shift
fi
OUT="${1:-$ROOT/dist/el8}"
IMAGE="${NYTPROF_EL8_IMAGE:-rockylinux:8}"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

if [[ "$NATIVE" -eq 0 ]]; then
  if [[ -r /etc/os-release ]]; then
    # shellcheck disable=SC1091
    . /etc/os-release
    if [[ "${ID:-}" == "rocky" && "${VERSION_ID:-}" == 8* ]]; then
      NATIVE=1
    fi
  fi
fi

if [[ "$NATIVE" -eq 0 ]]; then
  command -v docker >/dev/null 2>&1 || fail "docker required (or run --native on Rocky 8)"
  mkdir -p "$OUT"
  echo "re-exec in $IMAGE"
  docker run --rm \
    -v "$ROOT:/src:rw" \
    -w /src \
    "$IMAGE" \
    bash /src/scripts/packaging/build_el8_module_rpm.sh --native /src/dist/el8
  ls -la "$OUT"
  exit 0
fi

command -v rpmbuild >/dev/null 2>&1 || {
  echo "installing rpmbuild + D1-B BuildRequires"
  yum -y install \
    rpm-build rpmdevtools \
    gcc make binutils \
    perl-devel perl-generators perl-ExtUtils-MakeMaker \
    perl-ExtUtils-ParseXS \
    perl-Compress-Raw-Zlib \
    zlib-devel \
    which
  # perl(ExtUtils::Embed) is usually perl-devel; install by capability if present.
  yum -y install 'perl(ExtUtils::Embed)' 2>/dev/null || true
}

[[ -x "$ROOT/scripts/packaging/make_nytprofm_dist.sh" ]] \
  || fail "missing make_nytprofm_dist.sh"
[[ -f "$ROOT/packaging/rpm/perl-NYTProfM.spec" ]] \
  || fail "missing perl-NYTProfM.spec"
[[ -x "$ROOT/packaging/prebuilt/el8-x86_64/nytprof-cli" ]] \
  || fail "missing EL8 nytprof-cli prebuilt"

WORKDIR="$(mktemp -d /tmp/nytprofm-rpmbuild.XXXXXX)"
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT
mkdir -p "$WORKDIR"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}

echo "staging Source0"
DIST="$("$ROOT/scripts/packaging/make_nytprofm_dist.sh" "$WORKDIR/SOURCES")"
[[ -f "$DIST" ]] || fail "dist missing: $DIST"
cp -a "$ROOT/packaging/rpm/perl-NYTProfM.spec" "$WORKDIR/SPECS/perl-NYTProfM.spec"

echo "rpmbuild -ba perl-NYTProfM.spec"
rpmbuild -ba \
  --define "_topdir $WORKDIR" \
  --define "_sourcedir $WORKDIR/SOURCES" \
  --define "_builddir $WORKDIR/BUILD" \
  --define "_rpmdir $WORKDIR/RPMS" \
  --define "_srcrpmdir $WORKDIR/SRPMS" \
  --define "_specdir $WORKDIR/SPECS" \
  --define "_buildrootdir $WORKDIR/BUILDROOT" \
  "$WORKDIR/SPECS/perl-NYTProfM.spec"

mkdir -p "$OUT"
find "$WORKDIR/RPMS" "$WORKDIR/SRPMS" -type f -name '*.rpm' -exec cp -a {} "$OUT/" \;
ls -la "$OUT"
ok "EL8 RPMs in $OUT"
find "$OUT" -name 'perl-NYTProfM-6.15-*.rpm' | grep -q . \
  || fail "no perl-NYTProfM-6.15 RPM produced"
