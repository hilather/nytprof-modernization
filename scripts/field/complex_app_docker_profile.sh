#!/usr/bin/env bash
# Rocky 8 Docker lab: profile a real CPAN app (Rex) under in-tree
# perl -d:NYTProfM and emit native HTML.
#
# Why Rex: operator-facing Perl CLI that loads Getopt::Long, Moo,
# YAML, and (via this Rexfile) DateTime + DateTime::Duration — the
# class of compile-time %^H / namespace::autoclean failures that the
# core-only scanner never hits.
#
# Host:
#   ./scripts/field/complex_app_docker_profile.sh
#   ./scripts/field/complex_app_docker_profile.sh --engine both
#   ./scripts/field/complex_app_docker_profile.sh --out ~/Downloads/nytprof-rex-demo
#
# Inside the container (used by the host wrapper):
#   ./scripts/field/complex_app_docker_profile.sh --inside /out
#
# Always builds in-tree xs-nytprof inside rockylinux:8 (not the
# testdrive RPM .so — that lags attach fixes). HTML is rendered on
# the host with in-tree nytprof-cli after the container returns.
#
# Not mock-certified, not COPR, not a perf claim. Not in offline_gate.
set -euo pipefail

_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$_SCRIPT_DIR/lib/attach_survival.sh" ]]; then
  ROOT="$(cd "$_SCRIPT_DIR/../.." && pwd)"
  # shellcheck source=lib/attach_survival.sh
  source "$_SCRIPT_DIR/lib/attach_survival.sh"
  # shellcheck source=workloads/complex_apps/catalog_load.sh
  source "$_SCRIPT_DIR/workloads/complex_apps/catalog_load.sh"
  catalog_load
elif [[ -f /oracle-src/attach_survival.sh ]]; then
  ROOT=""
  # shellcheck source=/dev/null
  source /oracle-src/attach_survival.sh
else
  echo "ERROR: attach_survival.sh not found" >&2
  exit 1
fi
IMAGE="${NYTPROF_EL8_IMAGE:-rockylinux:8}"
REX_VERSION="${NYTPROF_REX_VERSION:-1.16.1}"
TARGET_SECS="${NYTPROF_DEMO_SECONDS:-5}"
APP_ID="${NYTPROF_COMPLEX_APP:-rex}"
REXFILE_SRC="$ROOT/scripts/field/workloads/rex_local_lab/Rexfile"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

usage() {
  cat <<'EOF'
Usage: complex_app_docker_profile.sh [--app ID] [--out DIR] [--lab] [--seconds N] [--engine native|oracle|both]

Profile a catalog top-10 app under NYTProfM and/or pinned 6.15.
Gate is attach *survival* (success token + NYTProf 5), not exclusive-time match.

  --app ID                  catalog id (default rex). Top-10 only for attach.
  --out DIR                 default ~/Downloads/nytprof-<app>-demo
  --lab                     short integration run (default 3s)
  --seconds N               profiled wall seconds (default 5; 3 with --lab)
  --engine native|oracle|both   default native
  --inside DIR              container native attach
  --inside-oracle DIR       container 6.15 attach (isolated mounts)
  -h, --help

Env:
  NYTPROF_EL8_IMAGE / NYTPROF_ORACLE_IMAGE
  NYTPROF_DEMO_SECONDS / NYTPROF_DEMO_LAB / NYTPROF_DEMO_ENGINE
  NYTPROF_COMPLEX_APP     same as --app
  NYTPROF_REX_VERSION     default 1.16.1
  NYTPROF_REX_LOCAL       default /opt/rex-local
EOF
}

write_notes() {
  local out="$1"
  cat >"$out/NOTES.txt" <<EOF
NYTProfM Rocky 8 Docker — Rex complex-app lab
=============================================

Date (UTC):     $(date -u +%Y-%m-%dT%H:%M:%SZ)
Image:          ${IMAGE}
Target wall:    ${TARGET_SECS}s under perl -d:NYTProfM
Lab mode:       ${NYTPROF_DEMO_LAB:-0}
Application:    scripts/field/workloads/rex_local_lab/run_lab.pl
                (use Rex ${REX_VERSION} + DateTime + DateTime::Duration + YAML)
                rex -T on Rexfile is an unprofiled load canary (no SSH)

How to inspect
--------------
Open:

  html/index.html
  (on the host: ~/Downloads/nytprof-rex-demo/html/index.html unless you passed --out)

Also in this directory:

  nytprof.out          raw NYTProf 5 profile (in-tree attach, collection_default=v5)
  html/                native MVP HTML (host nytprof-cli)
  meta/                os-release, perl -V, timings, attach/rex logs
  app/Rexfile          profiled Rexfile

Honesty
-------
In-tree xs-nytprof inside the container (not testdrive RPM .so).
Not mock-certified, not COPR, not a public performance claim.
Native HTML is MVP (no tablesorter / full DOM).
Rex is a field attach net — not a certified public bench.
EOF
}

resolve_host_cli() {
  if [[ -x "$ROOT/prefix/bin/nytprof-cli" ]]; then
    printf '%s\n' "$ROOT/prefix/bin/nytprof-cli"
    return 0
  fi
  if [[ -x "$ROOT/target/debug/nytprof-dump" ]]; then
    printf '%s\n' "$ROOT/target/debug/nytprof-dump"
    return 0
  fi
  if command -v cargo >/dev/null 2>&1; then
    printf 'cargo-run\n'
    return 0
  fi
  return 1
}

run_host_cli() {
  local cli="$1"
  shift
  if [[ "$cli" == "cargo-run" ]]; then
    (cd "$ROOT" && cargo run -q -p nytprof-cli -- "$@")
  else
    "$cli" "$@"
  fi
}

run_inside() {
  local OUT="$1"
  [[ -n "$OUT" ]] || fail "--inside requires an output directory"
  mkdir -p "$OUT"/{html,meta,app}

  APP_ID="${NYTPROF_COMPLEX_APP:-rex}"
  catalog_lookup "$APP_ID" || fail "catalog lookup $APP_ID"
  [[ "$APP_TIER" == "top10" ]] || fail "attach only for top-10 catalog ids (got $APP_ID tier=$APP_TIER)"
  echo "app=$APP_ID family=$APP_FAMILY token=$APP_TOKEN" >>"$OUT/meta/timings.txt"

  log "installing Rocky 8 packages"
  # shellcheck disable=SC2086
  yum -y install \
    perl perl-libs perl-interpreter perl-core \
    gcc make tar gzip curl ca-certificates \
    perl-devel perl-ExtUtils-ParseXS perl-ExtUtils-Embed perl-ExtUtils-MakeMaker \
    zlib-devel openssl-devel expat-devel \
    which findutils procps-ng hostname \
    ${APP_YUM:-} \
    >"$OUT/meta/yum-install.log" 2>&1 \
    || fail "yum install failed (see $OUT/meta/yum-install.log)"

  [[ -d /src/collector ]] || fail "repo not mounted at /src"
  local driver_src="/src/${APP_DRIVER}"
  [[ -f "$driver_src" ]] || fail "missing driver $driver_src"
  cp -a "$driver_src" "$OUT/app/run_lab.pl"
  if [[ "$APP_ID" == "rex" ]]; then
    cp -a /src/scripts/field/workloads/rex_local_lab/Rexfile "$OUT/app/Rexfile"
  fi

  log "building in-tree product XS (xs-nytprof)"
  rm -rf /tmp/nytprof-collector
  mkdir -p /tmp/nytprof-collector
  tar -C /src/collector -cf - \
    --exclude=build --exclude='*.o' --exclude='*.so' --exclude='*.a' \
    . | tar -C /tmp/nytprof-collector -xf -
  make -C /tmp/nytprof-collector xs-nytprof \
    >"$OUT/meta/xs-nytprof.log" 2>&1 \
    || fail "in-tree xs-nytprof failed (see meta/xs-nytprof.log)"
  local xsdest=/tmp/nytprof-collector/build/xs-nytprof
  [[ -f "$xsdest/auto/Devel/NYTProfM/NYTProfM.so" ]] \
    || fail "xs-nytprof missing NYTProfM.so"
  echo "attach_perl5lib=$xsdest" >"$OUT/meta/timings.txt"

  local rex_local="${NYTPROF_REX_LOCAL:-/opt/rex-local}"
  log "cpanm --notest -L $rex_local ($APP_CPAN)"
  local cpan_args
  IFS=',' read -r -a cpan_args <<<"$APP_CPAN"
  curl -fsSL https://cpanmin.us \
    | perl - --notest -L "$rex_local" "${cpan_args[@]}" \
    >"$OUT/meta/cpanm-app.log" 2>&1 \
    || fail "cpanm ${APP_CPAN} failed (see meta/cpanm-app.log)"
  echo "cpanm=${APP_CPAN}" >>"$OUT/meta/timings.txt"

  export PERL5LIB="${rex_local}/lib/perl5:${xsdest}"
  export COLUMNS="${COLUMNS:-80}"
  export LINES="${LINES:-24}"
  echo "PERL5LIB=$PERL5LIB" >>"$OUT/meta/timings.txt"
  unset PERL5OPT || true

  if [[ "$APP_ID" == "rex" && -x "$rex_local/bin/rex" && -f "$OUT/app/Rexfile" ]]; then
    log "rex -T (load canary, unprofiled)"
    set +e
    "$rex_local/bin/rex" -q -f "$OUT/app/Rexfile" -T \
      >"$OUT/meta/rex-T.txt" 2>"$OUT/meta/rex-T.err"
    echo "rex_T_rc=$?" >>"$OUT/meta/timings.txt"
    set -e
  fi

  : "${NYTPROF_DEMO_SECONDS:=$TARGET_SECS}"
  echo "target_secs=${NYTPROF_DEMO_SECONDS}" >>"$OUT/meta/timings.txt"
  echo "lab=${NYTPROF_DEMO_LAB:-0}" >>"$OUT/meta/timings.txt"

  cat /etc/os-release >"$OUT/meta/os-release.txt" 2>/dev/null || true
  perl -V >"$OUT/meta/perl-V.txt" 2>/dev/null || true

  log "profiling $APP_ID ($APP_DRIVER) with perl -d:NYTProfM (~${NYTPROF_DEMO_SECONDS}s wall)"
  local prof_start prof_elapsed
  prof_start=$(date +%s)
  set +e
  NYTPROF="file=${OUT}/nytprof.out" \
    NYTPROF_DEMO_SECONDS="${NYTPROF_DEMO_SECONDS}" \
    perl -I"$xsdest" -I"${rex_local}/lib/perl5" -d:NYTProfM \
      "$OUT/app/run_lab.pl" \
      >"$OUT/meta/rex-profiled.txt" 2>&1
  local prof_rc=$?
  set -e
  prof_elapsed=$(( $(date +%s) - prof_start ))
  {
    echo "profiled_secs=${prof_elapsed}"
    echo "profiled_rc=${prof_rc}"
    echo "primary_profile=$APP_ID"
  } >>"$OUT/meta/timings.txt"
  log "profiled $APP_ID wall ${prof_elapsed}s rc=${prof_rc}"

  attach_fail_if_killed "$OUT/meta/rex-profiled.txt" \
    || fail "attach-kill string for $APP_ID; see meta/rex-profiled.txt"
  [[ "$prof_rc" -eq 0 ]] \
    || fail "$APP_ID profile failed (rc=$prof_rc); see meta/rex-profiled.txt"
  attach_require_token "$OUT/meta/rex-profiled.txt" "$APP_TOKEN" \
    || fail "$APP_ID missing token $APP_TOKEN"
  attach_require_nytprof5 "$OUT/nytprof.out" \
    || fail "profile is not NYTProf 5"
  ls -l "$OUT/nytprof.out" >"$OUT/meta/nytprof-out-ls.txt"

  if [[ -n "${HOST_UID:-}" ]]; then
    chown -R "${HOST_UID}:${HOST_GID:-${HOST_UID}}" "$OUT" || true
  fi

  ok "profile written to $OUT/nytprof.out"
}

# 6.15 oracle inside a container. Host mounts archive + driver + Which
# only — never the repo root / crates/.
run_oracle_inside() {
  local OUT="$1"
  [[ -n "$OUT" ]] || fail "--inside-oracle requires an output directory"
  mkdir -p "$OUT"/{html,meta,app}

  unset PERL5LIB
  export PERL5LIB=""
  unset PERL5OPT || true

  log "oracle: installing Rocky 8 packages"
  yum -y install \
    perl perl-libs perl-interpreter perl-core \
    gcc make tar gzip curl ca-certificates \
    perl-devel perl-ExtUtils-MakeMaker \
    zlib-devel openssl-devel expat-devel \
    which findutils procps-ng hostname \
    >"$OUT/meta/yum-install.log" 2>&1 \
    || fail "oracle yum failed (see $OUT/meta/yum-install.log)"

  local archive="/oracle-src/Devel-NYTProf-6.15.tar.gz"
  local driver_src="/oracle-src/run_lab.pl"
  local rexfile_src="/oracle-src/Rexfile"
  local vendor="/oracle-src/vendor"
  [[ -f "$archive" ]] || fail "missing $archive"
  [[ -f "$driver_src" ]] || fail "missing $driver_src"
  cp -a "$driver_src" "$OUT/app/run_lab.pl"
  if [[ -f "$rexfile_src" ]]; then
    cp -a "$rexfile_src" "$OUT/app/Rexfile"
  fi

  local prefix="/opt/nytprof-oracle"
  mkdir -p "$prefix"
  if [[ ! -x "$prefix/bin/nytprofhtml" || ! -f "$prefix/lib/perl5/Devel/NYTProf.pm" ]]; then
    log "oracle: building Devel::NYTProf 6.15 into $prefix"
    rm -rf /tmp/Devel-NYTProf-6.15 /tmp/devel-nytprof-6.15
    tar -xzf "$archive" -C /tmp \
      >"$OUT/meta/oracle-extract.log" 2>&1 \
      || fail "oracle tar extract failed"
    local srcdir
    srcdir="$(find /tmp -maxdepth 1 -type d \( -name 'Devel-NYTProf-6.15' -o -name 'devel-nytprof-6.15' \) | head -1)"
    [[ -n "$srcdir" && -f "$srcdir/Makefile.PL" ]] \
      || fail "oracle extract missing Makefile.PL"
    (
      cd "$srcdir"
      perl Makefile.PL INSTALL_BASE="$prefix"
      make
      make install
    ) >"$OUT/meta/oracle-build.log" 2>&1 \
      || fail "oracle 6.15 compile failed (see meta/oracle-build.log)"
  else
    log "oracle: reusing cached $prefix"
    echo "oracle_prefix_cache=hit" >>"$OUT/meta/timings.txt"
  fi

  local arch
  arch="$(perl -MConfig -e 'print $Config{archname}')"
  local rex_local="${NYTPROF_REX_LOCAL:-/opt/rex-local}"
  local app_cpan="${NYTPROF_APP_CPAN:-Rex@1.16.1,DateTime,YAML,Module::Load}"
  local app_token="${NYTPROF_APP_TOKEN:-rex_lab_ok}"
  log "oracle: cpanm $app_cpan"
  local ocpan
  IFS=',' read -r -a ocpan <<<"$app_cpan"
  curl -fsSL https://cpanmin.us \
    | perl - --notest -L "$rex_local" "${ocpan[@]}" \
    >"$OUT/meta/cpanm-app.log" 2>&1 \
    || fail "oracle cpanm failed (see meta/cpanm-app.log)"
  echo "cpanm=${app_cpan}" >>"$OUT/meta/timings.txt"

  PERL5LIB="${prefix}/lib/perl5/${arch}:${prefix}/lib/perl5:${rex_local}/lib/perl5"
  if [[ -d "$vendor" ]]; then
    PERL5LIB="${PERL5LIB}:${vendor}"
  fi
  export PERL5LIB
  export COLUMNS="${COLUMNS:-80}"
  export LINES="${LINES:-24}"
  echo "PERL5LIB=${PERL5LIB}" >"$OUT/meta/perl5lib.txt"
  case ":${PERL5LIB}:" in
    *"/crates/"*|*"crates/"*)
      fail "oracle PERL5LIB must not contain crates/ (got: $PERL5LIB)"
      ;;
  esac
  case ":${PERL5LIB}:" in
    *"/baseline/6.15/install"*)
      fail "oracle PERL5LIB must not contain baseline/6.15/install"
      ;;
  esac

  perl -MDevel::NYTProf -e 'print $Devel::NYTProf::VERSION, "\n"' \
    >"$OUT/meta/nytprof-version.txt" \
    || fail "Devel::NYTProf did not load from oracle prefix"
  [[ -x "$prefix/bin/nytprofhtml" ]] || fail "nytprofhtml missing"

  : "${NYTPROF_DEMO_SECONDS:=$TARGET_SECS}"
  {
    echo "target_secs=${NYTPROF_DEMO_SECONDS}"
    echo "lab=${NYTPROF_DEMO_LAB:-0}"
    echo "engine=oracle"
    echo "primary_profile=run_lab"
  } >>"$OUT/meta/timings.txt"
  cat /etc/os-release >"$OUT/meta/os-release.txt" 2>/dev/null || true

  if [[ -x "$rex_local/bin/rex" && -f "$OUT/app/Rexfile" ]]; then
    set +e
    "$rex_local/bin/rex" -q -f "$OUT/app/Rexfile" -T \
      >"$OUT/meta/rex-T.txt" 2>"$OUT/meta/rex-T.err"
    echo "rex_T_rc=$?" >>"$OUT/meta/timings.txt"
    set -e
  fi

  log "oracle: profiling run_lab.pl with perl -d:NYTProf (~${NYTPROF_DEMO_SECONDS}s)"
  PATH="${prefix}/bin:${PATH}"
  export PATH
  local prof_start
  prof_start=$(date +%s)
  set +e
  NYTPROF="file=${OUT}/nytprof.out" \
    NYTPROF_DEMO_SECONDS="${NYTPROF_DEMO_SECONDS}" \
    perl -d:NYTProf "$OUT/app/run_lab.pl" \
    >"$OUT/meta/rex-profiled.txt" 2>&1
  local prof_rc=$?
  set -e
  {
    echo "profiled_rex_secs=$(( $(date +%s) - prof_start ))"
    echo "profiled_rex_rc=${prof_rc}"
  } >>"$OUT/meta/timings.txt"

  if grep -F -q 'as an ARRAY ref' "$OUT/meta/rex-profiled.txt" 2>/dev/null; then
    fail "oracle hit %^H ARRAY-ref (unexpected for 6.15); see meta/rex-profiled.txt"
  fi
  [[ "$prof_rc" -eq 0 ]] \
    || fail "oracle run_lab failed (rc=$prof_rc); see meta/rex-profiled.txt"
  attach_fail_if_killed "$OUT/meta/rex-profiled.txt" \
    || fail "oracle attach-kill string; see meta/rex-profiled.txt"
  attach_require_token "$OUT/meta/rex-profiled.txt" "$app_token" \
    || fail "oracle missing token $app_token"
  [[ -s "$OUT/nytprof.out" ]] || fail "missing oracle nytprof.out"
  head -c 9 "$OUT/nytprof.out" | grep -q 'NYTProf 5' \
    || fail "oracle nytprof.out is not NYTProf 5"

  log "oracle: nytprofhtml --no-flame (bounded)"
  set +e
  timeout 240 nytprofhtml --no-flame -o "$OUT/html" -f "$OUT/nytprof.out" \
    >"$OUT/meta/nytprofhtml.out" 2>"$OUT/meta/nytprofhtml.err"
  local html_rc=$?
  set -e
  if [[ "$html_rc" -eq 0 && -f "$OUT/html/index.html" ]]; then
    echo "html_secs=ok" >>"$OUT/meta/timings.txt"
  else
    echo "html_skip=1 rc=${html_rc}" >>"$OUT/meta/timings.txt"
    echo "oracle nytprofhtml skipped or failed rc=${html_rc} (attach still counts)" \
      >"$OUT/meta/oracle-html-skip.txt"
    log "oracle: nytprofhtml skipped/failed rc=${html_rc} — attach survival still stands"
  fi

  if [[ -n "${HOST_UID:-}" ]]; then
    chown -R "${HOST_UID}:${HOST_GID:-${HOST_UID}}" "$OUT" || true
  fi
  ok "oracle profile in $OUT"
}

migrate_then_link() {
  local dest="$1" target="$2"
  if [[ -L "$dest" ]]; then
    ln -sfn "$target" "$dest"
    return 0
  fi
  if [[ -d "$dest" ]]; then
    local parent name
    parent="$(dirname "$dest")"
    mkdir -p "$parent/native"
    name="$(basename "$dest")"
    if [[ ! -e "$parent/native/$name" ]]; then
      mv "$dest" "$parent/native/$name"
    else
      rm -rf "$dest"
    fi
  elif [[ -e "$dest" ]]; then
    rm -f "$dest"
  fi
  ln -sfn "$target" "$dest"
}

write_compare() {
  local root="$1"
  local cmp="$root/COMPARE.txt"
  {
    echo "Rex + DateTime attach survival (oracle 6.15 vs native NYTProfM)"
    echo "=============================================================="
    echo
    echo "Same driver, same --seconds, same Rex/DateTime/YAML. Gate is:"
    echo "  both print rex_lab_ok and write NYTProf 5."
    echo "Do NOT treat exclusive-second gaps as a native bug (AGENTS.md §5)."
    echo
    echo "native profiled:"
    if [[ -f "$root/native/meta/rex-profiled.txt" ]]; then
      cat "$root/native/meta/rex-profiled.txt"
    else
      echo "(missing native/meta/rex-profiled.txt)"
    fi
    echo
    echo "oracle profiled:"
    if [[ -f "$root/oracle/meta/rex-profiled.txt" ]]; then
      cat "$root/oracle/meta/rex-profiled.txt"
    elif [[ -f "$root/oracle/meta/oracle-skip.txt" ]]; then
      echo "SKIP: $(cat "$root/oracle/meta/oracle-skip.txt")"
    else
      echo "(no oracle half)"
    fi
    echo
    echo "Open:"
    echo "  native: $root/html/index.html"
    echo "  oracle: $root/oracle/html/index.html  (if nytprofhtml finished)"
  } >"$cmp"
}

render_native_html() {
  local dest="$1"
  local cli
  cli="$(resolve_host_cli)" || fail "no host nytprof-cli"
  mkdir -p "$dest/html" "$dest/meta"
  log "rendering native HTML with host $cli --no-flame"
  run_host_cli "$cli" html "$dest/nytprof.out" --out-dir "$dest/html" --no-flame \
    >"$dest/meta/host-html.out" 2>"$dest/meta/host-html.err" \
    || fail "host nytprof-cli html failed (see $dest/meta/host-html.err)"
  [[ -f "$dest/html/index.html" ]] || fail "html/index.html missing"
  run_host_cli "$cli" report "$dest/nytprof.out" \
    >"$dest/meta/report.txt" 2>"$dest/meta/report.err" || true
  run_host_cli "$cli" verify "$dest/nytprof.out" \
    >"$dest/meta/verify.txt" 2>"$dest/meta/verify.err" || true
  write_notes "$dest"
  cat >"$dest/open-report.sh" <<'EOS'
#!/usr/bin/env bash
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
xdg-open "$DIR/html/index.html" 2>/dev/null || \
  sensible-browser "$DIR/html/index.html" 2>/dev/null || \
  echo "Open $DIR/html/index.html in a browser"
EOS
  chmod 755 "$dest/open-report.sh"
}

# --- host wrapper / flag parse ---
OUT=""
INSIDE=""
INSIDE_ORACLE=""
ENGINE="${NYTPROF_DEMO_ENGINE:-native}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --app)
      APP_ID="${2:-}"
      [[ -n "$APP_ID" ]] || fail "--app needs a catalog id"
      shift 2
      ;;
    --out)
      OUT="${2:-}"
      [[ -n "$OUT" ]] || fail "--out needs a directory"
      shift 2
      ;;
    --lab)
      export NYTPROF_DEMO_LAB=1
      TARGET_SECS="${NYTPROF_DEMO_SECONDS:-3}"
      shift
      ;;
    --seconds)
      TARGET_SECS="${2:-}"
      [[ -n "$TARGET_SECS" ]] || fail "--seconds needs N"
      shift 2
      ;;
    --engine)
      ENGINE="${2:-}"
      [[ -n "$ENGINE" ]] || fail "--engine needs native|oracle|both"
      shift 2
      ;;
    --inside)
      INSIDE="${2:-}"
      [[ -n "$INSIDE" ]] || fail "--inside needs a directory"
      shift 2
      ;;
    --inside-oracle)
      INSIDE_ORACLE="${2:-}"
      [[ -n "$INSIDE_ORACLE" ]] || fail "--inside-oracle needs a directory"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "unknown flag: $1"
      ;;
  esac
done

case "$ENGINE" in
  native|oracle|both) ;;
  *) fail "--engine must be native, oracle, or both (got: $ENGINE)" ;;
esac

if [[ -n "$INSIDE" ]]; then
  TARGET_SECS="${NYTPROF_DEMO_SECONDS:-$TARGET_SECS}"
  export NYTPROF_DEMO_SECONDS="$TARGET_SECS"
  run_inside "$INSIDE"
  exit 0
fi
if [[ -n "$INSIDE_ORACLE" ]]; then
  TARGET_SECS="${NYTPROF_DEMO_SECONDS:-$TARGET_SECS}"
  export NYTPROF_DEMO_SECONDS="$TARGET_SECS"
  run_oracle_inside "$INSIDE_ORACLE"
  exit 0
fi

catalog_lookup "$APP_ID" || fail "unknown --app $APP_ID"
[[ "$APP_TIER" == "top10" ]] || fail "--app $APP_ID is not a top-10 attach target"
DRIVER_SRC="$ROOT/$APP_DRIVER"
[[ -f "$DRIVER_SRC" ]] || fail "missing driver $DRIVER_SRC"
if [[ -n "${NYTPROF_DEMO_LAB:-}" && "$NYTPROF_DEMO_LAB" == "1" ]]; then
  TARGET_SECS="${NYTPROF_DEMO_SECONDS:-3}"
fi
export NYTPROF_DEMO_SECONDS="$TARGET_SECS"
export NYTPROF_DEMO_ENGINE="$ENGINE"
export NYTPROF_COMPLEX_APP="$APP_ID"
export NYTPROF_APP_TOKEN="$APP_TOKEN"
export NYTPROF_APP_CPAN="$APP_CPAN"

if [[ -z "$OUT" ]]; then
  OUT="${HOME}/Downloads/nytprof-${APP_ID}-demo"
fi
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"

if ! command -v docker >/dev/null 2>&1; then
  fail "docker not on PATH"
fi
if ! docker info >/dev/null 2>&1; then
  fail "docker daemon not reachable"
fi

log "image: $IMAGE  out: $OUT  seconds: $TARGET_SECS  app: $APP_ID  engine: $ENGINE"
docker pull "$IMAGE" >"$OUT/docker-pull.log" 2>&1 || fail "docker pull $IMAGE failed (see $OUT/docker-pull.log)"

ARCHIVE="$ROOT/baseline/6.15/archives/Devel-NYTProf-6.15.tar.gz"
WHICH_PM="$ROOT/baseline/6.15/test-deps/lib/perl5/File/Which.pm"
ORACLE_IMAGE="${NYTPROF_ORACLE_IMAGE:-$IMAGE}"

run_native_host() {
  local dest="$1"
  mkdir -p "$dest"
  log "native container → $dest"
  set +e
  docker run --rm \
    -v "$ROOT:/src:ro" \
    -v "$dest:/out" \
    -v nytprof-rex-cpan:/opt/rex-local \
    -e NYTPROF_DEMO_SECONDS \
    -e NYTPROF_DEMO_LAB \
    -e NYTPROF_COMPLEX_APP="$APP_ID" \
    -e NYTPROF_REX_VERSION="$REX_VERSION" \
    -e HOST_UID="$(id -u)" \
    -e HOST_GID="$(id -g)" \
    "$IMAGE" \
    bash /src/scripts/field/complex_app_docker_profile.sh --inside /out
  local rc=$?
  set -e
  [[ "$rc" -eq 0 ]] || fail "native container attach failed (rc=$rc); see $dest/meta/"
  [[ -s "$dest/nytprof.out" ]] || fail "native missing nytprof.out"
  render_native_html "$dest"
}

run_oracle_host() {
  local dest="$1"
  mkdir -p "$dest"
  if [[ ! -f "$ARCHIVE" ]]; then
    log "SKIP: missing $ARCHIVE — oracle half not run"
    mkdir -p "$dest/meta"
    echo "missing Devel-NYTProf-6.15.tar.gz" >"$dest/meta/oracle-skip.txt"
    return 0
  fi
  if [[ ! -f "$WHICH_PM" ]]; then
    log "SKIP: File::Which not in checkout — oracle half not run"
    mkdir -p "$dest/meta"
    echo "missing File::Which (baseline test-deps gitignored)" \
      >"$dest/meta/oracle-skip.txt"
    return 0
  fi
  log "oracle container → $dest (isolated: no repo root / crates/)"
  set +e
  docker run --rm \
    -e PERL5LIB= \
    -e NYTPROF_DEMO_SECONDS \
    -e NYTPROF_DEMO_LAB \
    -e NYTPROF_COMPLEX_APP="$APP_ID" \
    -e NYTPROF_APP_TOKEN="$APP_TOKEN" \
    -e NYTPROF_APP_CPAN="$APP_CPAN" \
    -e NYTPROF_REX_VERSION="$REX_VERSION" \
    -e HOST_UID="$(id -u)" \
    -e HOST_GID="$(id -g)" \
    -v nytprof-oracle-prefix:/opt/nytprof-oracle \
    -v nytprof-rex-cpan:/opt/rex-local \
    -v "$ARCHIVE:/oracle-src/Devel-NYTProf-6.15.tar.gz:ro" \
    -v "$DRIVER_SRC:/oracle-src/run_lab.pl:ro" \
    -v "$ROOT/scripts/field/lib/attach_survival.sh:/oracle-src/attach_survival.sh:ro" \
    -v "$WHICH_PM:/oracle-src/vendor/File/Which.pm:ro" \
    -v "$ROOT/scripts/field/complex_app_docker_profile.sh:/oracle-src/demo.sh:ro" \
    -v "$dest:/out:rw" \
    "$ORACLE_IMAGE" \
    bash /oracle-src/demo.sh --inside-oracle /out
  local orc=$?
  set -e
  if [[ "$orc" -ne 0 ]]; then
    log "SKIP: oracle container failed (rc=$orc) — not faking index.html"
    mkdir -p "$dest/meta"
    echo "oracle_skip=1 rc=$orc" >"$dest/meta/oracle-skip.txt"
    return 0
  fi
  [[ -s "$dest/nytprof.out" ]] || fail "oracle missing nytprof.out after success"
  return 0
}

case "$ENGINE" in
  native)
    run_native_host "$OUT"
    ;;
  oracle)
    run_oracle_host "$OUT/oracle"
    [[ -s "$OUT/oracle/nytprof.out" ]] \
      || fail "oracle half skipped or missing; --engine oracle requires a profile"
    ;;
  both)
    run_native_host "$OUT/native"
    run_oracle_host "$OUT/oracle"
    migrate_then_link "$OUT/html" "native/html"
    migrate_then_link "$OUT/meta" "native/meta"
    migrate_then_link "$OUT/nytprof.out" "native/nytprof.out"
    [[ -L "$OUT/html" ]] || fail "html must be a symlink after --engine both"
    write_compare "$OUT"
    ;;
esac

ok "Rex profile + HTML in $OUT"
if [[ -f "$OUT/html/index.html" ]]; then
  ok "native HTML $OUT/html/index.html"
fi
if [[ -f "$OUT/oracle/html/index.html" ]]; then
  ok "oracle HTML $OUT/oracle/html/index.html"
elif [[ -f "$OUT/oracle/meta/oracle-skip.txt" ]]; then
  log "oracle SKIP: $(cat "$OUT/oracle/meta/oracle-skip.txt")"
fi
if [[ -f "$OUT/COMPARE.txt" ]]; then
  log "--- COMPARE.txt ---"
  cat "$OUT/COMPARE.txt"
fi
exit 0
