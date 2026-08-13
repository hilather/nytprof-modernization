#!/usr/bin/env bash
# PR-G02 — v5-only product archive + load-only XS bootstrap smoke.
#
# Drives the real shipped make targets, inspects the produced archive with
# ar/nm, links/runs the -lz-only probe, and (when CC + Perl XS headers exist)
# builds and loads Devel::NYTProf::CollectorBootstrap.
#
# This is NOT product attach. Must not print OK: attach works or
# product_xs_attach=1. Does not rewrite or invoke G01 smokes as green attach.
# Never required cargo. Never puts crates/ on PERL5LIB.
#
# Exit 0: pass, or honest SKIP: when CC is absent (after source/Makefile asserts).
# Exit 1: archive/probe/XS failure.
# Exit 2: isolation / wrapper misuse.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

COLLECTOR="$ROOT/collector"
MAKEFILE="$COLLECTOR/Makefile"
LIB_V5="$COLLECTOR/build/libnytp_sink_v5.a"
PROBE="$COLLECTOR/build/probe_v5_product_link"
XS_BOOT="$COLLECTOR/build/xs-bootstrap"
XS_SO="$XS_BOOT/auto/Devel/NYTProf/CollectorBootstrap/CollectorBootstrap.so"
XS_PM_SRC="$COLLECTOR/xs/Devel/NYTProf/CollectorBootstrap.pm"
XS_XS="$COLLECTOR/xs/CollectorBootstrap.xs"
PROBE_SRC="$COLLECTOR/t/probe_v5_product_link.c"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
fail2() { printf 'ERROR: %s\n' "$*" >&2; exit 2; }
banner() { printf '\n=== %s ===\n' "$*"; }

banner "g02_v5_product_link_smoke (D1-B archive + load-only XS; attach NOT ready)"

# ---------------------------------------------------------------------------
# Isolation: never crates/ on PERL5LIB
# ---------------------------------------------------------------------------
if [[ -n "${PERL5LIB:-}" ]]; then
  case ":${PERL5LIB}:" in
    *"/crates/"*)
      fail2 "PERL5LIB must not contain /crates/: $PERL5LIB"
      ;;
  esac
fi
ok "parent PERL5LIB has no crates/"

# ---------------------------------------------------------------------------
# Sources + Makefile target must exist even on honest skip
# ---------------------------------------------------------------------------
[[ -f "$MAKEFILE" ]] || fail "missing $MAKEFILE"
[[ -f "$PROBE_SRC" ]] || fail "missing $PROBE_SRC"
[[ -f "$XS_XS" ]] || fail "missing $XS_XS"
[[ -f "$XS_PM_SRC" ]] || fail "missing $XS_PM_SRC"
[[ -f "$COLLECTOR/include/nytp_sink_v5.h" ]] || fail "missing nytp_sink_v5.h"
[[ -f "$COLLECTOR/src/nytp_sink_v5.c" ]] || fail "missing nytp_sink_v5.c"
grep -q 'LIB_V5' "$MAKEFILE" || fail "Makefile missing LIB_V5"
grep -q 'libnytp_sink_v5.a' "$MAKEFILE" || fail "Makefile missing libnytp_sink_v5.a target"
grep -q 'probe-v5' "$MAKEFILE" || fail "Makefile missing probe-v5 target"
grep -q 'xs-bootstrap' "$MAKEFILE" || fail "Makefile missing xs-bootstrap target"
# Product archive must not list v6/dual in SRC_V5.
if awk '
  /^SRC_V5[[:space:]]*:?=/ {flag=1}
  flag && /^[^[:space:]#]/ && !/^SRC_V5/ && !/^[[:space:]]/ {flag=0}
  flag {print}
' "$MAKEFILE" | grep -E -q 'nytp_sink_v6|nytp_sink_dual'; then
  fail "Makefile SRC_V5 must not include nytp_sink_v6 or nytp_sink_dual"
fi
ok "G02 sources and Makefile targets present"

resolve_cc() {
  if [[ -n "${CC-}" ]] && command -v "$CC" >/dev/null 2>&1; then
    printf '%s\n' "$CC"
    return 0
  fi
  for c in cc gcc clang; do
    if command -v "$c" >/dev/null 2>&1; then
      printf '%s\n' "$c"
      return 0
    fi
  done
  return 1
}

if ! CC_BIN="$(resolve_cc)"; then
  echo "SKIP: no C toolchain (cc/gcc/clang) — G02 archive/probe/XS not built"
  echo "  (honest skip; G02 archive/probe requires CC)"
  echo "product_xs_attach: no"
  echo "product_xs_attach: not-ready"
  ok "g02_v5_product_link_smoke completed (skip — no CC)"
  exit 0
fi
ok "C toolchain: $CC_BIN"

# ---------------------------------------------------------------------------
# 1. Real shipped make target for the v5-only archive
# ---------------------------------------------------------------------------
banner "make -C collector libnytp_sink_v5.a"
make -C "$COLLECTOR" libnytp_sink_v5.a
[[ -f "$LIB_V5" ]] || fail "make libnytp_sink_v5.a did not produce $LIB_V5"
ok "produced $LIB_V5"

# ---------------------------------------------------------------------------
# 2. Inspect the real archive (ar t / nm) — not a reimplemented member list
# ---------------------------------------------------------------------------
banner "ar t / nm $LIB_V5"
command -v ar >/dev/null 2>&1 || fail "ar is required to inspect the product archive"
command -v nm >/dev/null 2>&1 || fail "nm is required to inspect the product archive"

AR_LIST="$(ar t "$LIB_V5")"
printf '%s\n' "$AR_LIST"
[[ -n "$AR_LIST" ]] || fail "ar t produced an empty member list"

for member in nytp_sink.o nytp_sink_v5.o nytp_sink_counting.o nytp_batch.o nytp_clock.o nytp_fork.o; do
  grep -F -x -q -- "$member" <<<"$AR_LIST" \
    || fail "libnytp_sink_v5.a missing required member: $member"
done
ok "archive members include v5/batch/clock/fork/counting/sink"

if grep -E -q 'nytp_sink_v6|nytp_sink_dual' <<<"$AR_LIST"; then
  fail "libnytp_sink_v5.a must not contain nytp_sink_v6 or nytp_sink_dual members"
fi
ok "archive has no v6/dual members"

NM_OUT="$(nm "$LIB_V5")"
[[ -n "$NM_OUT" ]] || fail "nm produced empty output for $LIB_V5"
for sym in nytp_v5_sink_create nytp_v5_sink_is_v5 nytp_v5_sink_wire nytp_sink_destroy; do
  grep -E -q "[[:space:]]${sym}$|[[:space:]]${sym}[[:space:]]" <<<"$NM_OUT" \
    || fail "libnytp_sink_v5.a nm missing symbol: $sym"
done
ok "archive nm has v5 create/is_v5/wire + sink_destroy"

if grep -E -q 'nytp_v6_sink_create|nytp_dual_sink_create|nytp_v6_sink_is_v6|nytp_dual_sink_is_dual' <<<"$NM_OUT"; then
  fail "libnytp_sink_v5.a nm must not define v6/dual symbols"
fi
ok "archive nm has no v6/dual create symbols"

# ---------------------------------------------------------------------------
# 3. Real -lz-only probe (shipped make target)
# ---------------------------------------------------------------------------
banner "make -C collector probe-v5"
PROBE_OUT="$(make -C "$COLLECTOR" probe-v5)"
printf '%s\n' "$PROBE_OUT"
[[ -x "$PROBE" ]] || fail "probe-v5 did not produce $PROBE"
grep -F -q 'OK: v5-only product link probe' <<<"$PROBE_OUT" \
  || fail "probe-v5 did not report v5 header success"
grep -F -q 'NYTProf 5' <<<"$PROBE_OUT" \
  || fail "probe-v5 output missing NYTProf 5"

assert_no_zstd_lz4() {
  local bin="$1"
  local label="$2"
  local deps=""
  if command -v readelf >/dev/null 2>&1; then
    deps="$(readelf -d "$bin" 2>/dev/null || true)"
  fi
  if [[ -z "$deps" ]] && command -v ldd >/dev/null 2>&1; then
    deps="$(ldd "$bin" 2>/dev/null || true)"
  fi
  [[ -n "$deps" ]] || fail "$label: need readelf -d or ldd to inspect dynamic libs"
  if grep -Ei -q 'zstd|lz4' <<<"$deps"; then
    fail "$label is linked with zstd/lz4 (D1-B product path is -lz only)"
  fi
  if ! grep -Ei -q 'libz\.so|NEEDED[[:space:]]+\[libz\.so' <<<"$deps"; then
    # Static zlib is acceptable; require either libz NEEDED or zlib symbols.
    if ! nm "$bin" 2>/dev/null | grep -E -q 'deflate|inflate|zlibVersion|gz'; then
      fail "$label: expected zlib (-lz) dependency or symbols"
    fi
  fi
  ok "$label: no zstd/lz4; zlib present"
}

assert_no_zstd_lz4 "$PROBE" "probe_v5_product_link"
ok "probe-v5 ran against real v5 archive"

# ---------------------------------------------------------------------------
# 4. Bootstrap XS when perl + XS headers exist
# ---------------------------------------------------------------------------
have_xs_headers=0
if command -v perl >/dev/null 2>&1; then
  if perl -MConfig -e 'exit((-f "$Config{archlibexp}/CORE/EXTERN.h") ? 0 : 1)'; then
    have_xs_headers=1
  fi
fi

if [[ "$have_xs_headers" -ne 1 ]]; then
  echo "SKIP: perl XS headers (EXTERN.h) not present — bootstrap .so not built"
  echo "product_xs_attach: no"
  echo "product_xs_attach: not-ready"
  ok "g02_v5_product_link_smoke completed (archive + probe green; XS skipped)"
  exit 0
fi
ok "perl + EXTERN.h present"

banner "make -C collector xs-bootstrap"
make -C "$COLLECTOR" xs-bootstrap
[[ -f "$XS_SO" ]] || fail "xs-bootstrap did not produce $XS_SO"
[[ -f "$XS_BOOT/Devel/NYTProf/CollectorBootstrap.pm" ]] \
  || fail "xs-bootstrap did not install CollectorBootstrap.pm into build/"
ok "xs-bootstrap produced .so + .pm under collector/build/"

assert_no_zstd_lz4 "$XS_SO" "CollectorBootstrap.so"

# Linked .so must contain the real v5 archive symbols (not a stub).
SO_NM="$(nm "$XS_SO" 2>/dev/null || nm -D "$XS_SO" 2>/dev/null || true)"
[[ -n "$SO_NM" ]] || fail "nm produced empty output for $XS_SO"
grep -E -q 'nytp_v5_sink_create' <<<"$SO_NM" \
  || fail "CollectorBootstrap.so does not contain nytp_v5_sink_create (not linked to v5 archive)"
if grep -E -q 'nytp_v6_sink_create|nytp_dual_sink_create' <<<"$SO_NM"; then
  fail "CollectorBootstrap.so must not contain v6/dual symbols"
fi
ok "CollectorBootstrap.so contains v5 sink symbols, not v6/dual"

banner "perl -MDevel::NYTProf::CollectorBootstrap"
# Isolated load path: product bootstrap only. Never crates/.
LOAD_OUT="$(
  PERL5LIB="$XS_BOOT" perl -I"$XS_BOOT" -MDevel::NYTProf::CollectorBootstrap -e '
    die "loaded() false\n" unless Devel::NYTProf::CollectorBootstrap::loaded();
    my $f = Devel::NYTProf::CollectorBootstrap::product_link_flavor();
    die "flavor=$f want v5-only\n" unless defined $f && $f eq "v5-only";
    my $a = Devel::NYTProf::CollectorBootstrap::product_xs_attach();
    die "product_xs_attach=$a want 0\n" if $a;
    my $p = Devel::NYTProf::CollectorBootstrap::probe_v5_header();
    die "probe_v5_header failed\n" unless $p;
    if (defined $Devel::NYTProf::PRODUCT_XS_ATTACH && $Devel::NYTProf::PRODUCT_XS_ATTACH) {
      die "must not set \$Devel::NYTProf::PRODUCT_XS_ATTACH true\n";
    }
    print "OK: CollectorBootstrap loaded flavor=$f product_xs_attach=$a probe_v5_header=$p\n";
  '
)"
printf '%s\n' "$LOAD_OUT"

grep -F -q 'OK: CollectorBootstrap loaded' <<<"$LOAD_OUT" \
  || fail "bootstrap load did not report success"
grep -F -q 'flavor=v5-only' <<<"$LOAD_OUT" \
  || fail "bootstrap load missing flavor=v5-only"
grep -F -q 'product_xs_attach=0' <<<"$LOAD_OUT" \
  || fail "bootstrap load missing product_xs_attach=0"
if grep -F -q 'OK: attach works' <<<"$LOAD_OUT"; then
  fail "bootstrap output must not contain OK: attach works"
fi
if grep -F -q 'product_xs_attach=1' <<<"$LOAD_OUT"; then
  fail "bootstrap output must not contain product_xs_attach=1"
fi
ok "CollectorBootstrap load-only (v5-only; attach not ready)"

echo "product_xs_attach: no"
echo "product_xs_attach: not-ready"
echo "G02 is archive/bootstrap; live attach: g04_v5_parity_smoke.sh"
ok "g02_v5_product_link_smoke passed (scaffold; live attach is G04)"
exit 0
