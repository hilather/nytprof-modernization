#!/usr/bin/env bash
# Rocky 8 Docker testdrive: install perl-NYTProfM, download a public-domain
# corpus (+ ack v3), profile a core-only ~60s Perl analyzer, emit HTML.
#
# Host:
#   ./scripts/field/rocky8_docker_profile_demo.sh
#   ./scripts/field/rocky8_docker_profile_demo.sh --out ~/Downloads/nytprof-rocky8-demo
#
# Inside the container (used by the host wrapper):
#   ./scripts/field/rocky8_docker_profile_demo.sh --inside /out
#
# Uses the unsigned testdrive RPM (local dist/el8 or a GitHub Release asset).
# Not mock-certified, not COPR, not a perf claim. HTML is nytprofm-cli html
# (module RPM is collection-only; no stock nytprofhtml overwrite).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
IMAGE="${NYTPROF_EL8_IMAGE:-rockylinux:8}"
DEFAULT_OUT="${HOME}/Downloads/nytprof-rocky8-demo"
RPM_NAME="perl-NYTProfM-6.15-6.el8.x86_64.rpm"
RELEASE_TAG="${NYTPROF_DEMO_RELEASE:-v0.2.12}"
RELEASE_RPM_URL="${NYTPROF_DEMO_RPM_URL:-https://github.com/hilather/nytprof-modernization/releases/download/${RELEASE_TAG}/${RPM_NAME}}"
ACK_URL="${NYTPROF_DEMO_ACK_URL:-https://beyondgrep.com/ack-v3.7.0}"
ACK_FALLBACK_URL="${NYTPROF_DEMO_ACK_FALLBACK_URL:-https://raw.githubusercontent.com/beyondgrep/ack3/3.7.0/ack}"
TEXT_URL="${NYTPROF_DEMO_TEXT_URL:-https://www.gutenberg.org/files/1342/1342-0.txt}"
TARGET_SECS="${NYTPROF_DEMO_SECONDS:-60}"

ok() { printf 'OK: %s\n' "$*"; }
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
log() { printf '%s\n' "$*"; }

usage() {
  cat <<'EOF'
Usage: rocky8_docker_profile_demo.sh [--out DIR] [--lab] [--seconds N] [--engine native|oracle|both]

Run Rocky Linux 8 container(s). Default --engine native installs the testdrive
perl-NYTProfM RPM, profiles a core-only text analyzer, and writes nytprof.out
+ native HTML into DIR (default: ~/Downloads/nytprof-rocky8-demo).

  --engine native   (default) NYTProfM + native HTML → $OUT/{nytprof.out,html,meta}
  --engine oracle   Devel::NYTProf 6.15 from the committed archive → $OUT/oracle/
  --engine both     native + oracle, then migrate-then-link so $OUT/html is a
                    symlink to native/html (existing $OUT/html dirs are moved)

  (default)  ~60s operator demo: download ack + Gutenberg corpus
  --lab      integration path: generated seed, no extra downloads,
             default 3s profile. Used by rocky8_docker_profile_smoke.sh

Oracle isolation: the oracle container bind-mounts only the 6.15 tarball, the
scanner, and File::Which — never the repo root. PERL5LIB is a literal
/opt/nytprof-oracle prefix. Do not source tools/oracle/env.sh. Fail closed
if PERL5LIB contains crates or baseline/6.15/install. Oracle compile failure
is an honest SKIP of the oracle half (does not fake index.html).

Options:
  --out DIR              Host output directory (created)
  --lab                  Short offline-friendly integration run
  --seconds N            Profiled wall seconds (default 60; 3 with --lab)
  --engine native|oracle|both   default native
  --inside DIR           Container-only: native demo writing to DIR
  --inside-oracle DIR    Container-only: oracle 6.15 demo writing to DIR
  -h, --help             Show this help

Env:
  NYTPROF_EL8_IMAGE          default rockylinux:8
  NYTPROF_DEMO_SECONDS       target profiled wall seconds
  NYTPROF_DEMO_LAB           1 = same as --lab (set by the smoke)
  NYTPROF_DEMO_ENGINE        native|oracle|both (overridden by --engine)
  NYTPROF_DEMO_RPM           host path to perl-NYTProfM *.x86_64.rpm
  NYTPROF_DEMO_RPM_URL       GitHub Release RPM if no local RPM
  NYTPROF_DEMO_RELEASE       tag used to build the default RPM URL (v0.2.8)
  NYTPROF_ORACLE_IMAGE       default same as NYTPROF_EL8_IMAGE (Rocky first)
EOF
}

fetch() {
  local url="$1" dest="$2"
  curl -fsSL --retry 3 --retry-delay 2 -o "$dest" "$url"
}

write_notes() {
  local out="$1"
  cat >"$out/NOTES.txt" <<EOF
NYTProfM Rocky 8 Docker profile demo
====================================

Date (UTC):     $(date -u +%Y-%m-%dT%H:%M:%SZ)
Image:          ${IMAGE}
Target wall:    ${TARGET_SECS}s under perl -d:NYTProfM
Lab mode:       ${NYTPROF_DEMO_LAB:-0}
Application:    scripts/field/workloads/minute_text_scanner.pl
                (core-only: tokenize / classify / merge over the corpus)
Downloaded:     lab=1 generated seed only; otherwise ack v3.7.0 + Gutenberg
Corpus:         seed copies under corpus/

How to inspect
--------------
Open:

  html/index.html
  (on the host: ~/Downloads/nytprof-rocky8-demo/html/index.html unless you passed --out)

Also in this directory:

  nytprof.out          raw NYTProf 5 profile (D1-B / collection_default=v5)
  html/                native MVP HTML site (not oracle nytprofhtml DOM)
  meta/                os-release, perl -V, capability, timings, logs
  app/ack              downloaded ack (unprofiled; see honesty)
  app/minute_text_scanner.pl

Commands used (inside rockylinux:8)
-----------------------------------
  rpm -Uvh ${RPM_NAME}
  NYTPROF=file=/out/nytprof.out perl -d:NYTProfM \\
    app/minute_text_scanner.pl corpus ${TARGET_SECS}
  nytprofm-cli html /out/nytprof.out --out-dir /out/html

Honesty
-------
Unsigned testdrive RPM. Not mock-certified, not COPR, not GPG-signed.
Not a public performance claim. Native HTML is MVP (no tablesorter / full DOM).
ack is downloaded and syntax-checked but not profiled: the demo still
uses the core-only scanner. PR-7 landed compile-safe start + goto for
Getopt/Exporter (g07_getopt_compile_smoke.sh); ack retry is later.
EOF
}

run_inside() {
  local OUT="$1"
  [[ -n "$OUT" ]] || fail "--inside requires an output directory"
  mkdir -p "$OUT"/{html,meta,app,corpus}

  log "installing Rocky 8 packages"
  yum -y install \
    perl perl-libs perl-interpreter \
    zlib gzip tar curl ca-certificates \
    which findutils \
    >"$OUT/meta/yum-install.log" 2>&1 \
    || fail "yum install failed (see $OUT/meta/yum-install.log)"

  local rpm_src=""
  if [[ -n "${NYTPROF_DEMO_RPM:-}" && -f "${NYTPROF_DEMO_RPM}" ]]; then
    rpm_src="${NYTPROF_DEMO_RPM}"
  elif [[ -f "/rpm/${RPM_NAME}" ]]; then
    rpm_src="/rpm/${RPM_NAME}"
  fi

  local rpm_file="$OUT/meta/${RPM_NAME}"
  if [[ -n "$rpm_src" ]]; then
    cp -a "$rpm_src" "$rpm_file"
    log "using local RPM $rpm_src"
  else
    log "downloading RPM $RELEASE_RPM_URL"
    fetch "$RELEASE_RPM_URL" "$rpm_file" \
      || fail "could not download $RELEASE_RPM_URL (set NYTPROF_DEMO_RPM)"
  fi

  log "installing $RPM_NAME"
  rpm -Uvh "$rpm_file" >"$OUT/meta/rpm-install.log" 2>&1 \
    || fail "rpm install failed (see $OUT/meta/rpm-install.log)"

  command -v perl >/dev/null || fail "perl missing after install"
  if ! command -v nytprofm-cli >/dev/null; then
    echo "SKIP: nytprofm-cli not on PATH after RPM install (HTML residual)" \
      >"$OUT/meta/html.skip"
  fi
  perl -MDevel::NYTProfM -e 'print $Devel::NYTProfM::VERSION, "\n"' \
    >"$OUT/meta/nytprofm-version.txt" \
    || fail "Devel::NYTProfM did not load"

  {
    echo "=== /etc/os-release ==="
    cat /etc/os-release
    echo
    echo "=== perl -V ==="
    perl -V
    echo
    echo "=== rpm -q perl-NYTProfM ==="
    rpm -q perl-NYTProfM
    echo
    echo "=== nytprofm-cli capability ==="
    command -v nytprofm-cli >/dev/null && nytprofm-cli capability || echo "SKIP: no nytprofm-cli"
    echo
    echo "=== nytprofm-cli capability --json ==="
    command -v nytprofm-cli >/dev/null && nytprofm-cli capability --json || echo "SKIP: no nytprofm-cli"
  } >"$OUT/meta/environment.txt" 2>&1

  local seed="$OUT/meta/seed.txt"
  local lab="${NYTPROF_DEMO_LAB:-0}"
  write_generated_seed() {
    perl -e '
      print "It is a truth universally acknowledged that a profiler in want of a report must be in search of a long-running Perl application.\n" x 400;
      for my $i (1..200) {
        print "Record $i: sub process { my (\$line) = @_; \$line =~ s/\\s+/ /g; return length \$line }\n";
      }
    ' >"$seed"
  }

  if [[ "$lab" == "1" ]]; then
    log "lab mode: generated seed (no Gutenberg / ack download)"
    write_generated_seed
  else
    log "downloading ack v3.7.0"
    if ! fetch "$ACK_URL" "$OUT/app/ack"; then
      log "primary ack URL failed; trying GitHub raw"
      fetch "$ACK_FALLBACK_URL" "$OUT/app/ack" \
        || fail "could not download ack"
    fi
    chmod 755 "$OUT/app/ack"
    head -1 "$OUT/app/ack" | grep -q perl \
      || fail "downloaded ack does not look like a Perl script"
    perl -c "$OUT/app/ack" >"$OUT/meta/ack-syntax.txt" 2>&1 \
      || fail "ack -c failed (see $OUT/meta/ack-syntax.txt)"

    log "downloading text corpus"
    if ! fetch "$TEXT_URL" "$seed"; then
      log "Gutenberg download failed; generating a local seed text"
      write_generated_seed
    fi
  fi
  [[ -s "$seed" ]] || fail "empty corpus seed"

  local copies=2
  if [[ "$lab" != "1" ]]; then
    copies=12
  fi
  log "building corpus ($copies copies of seed)"
  local i
  for i in $(seq 1 "$copies"); do
    mkdir -p "$OUT/corpus/batch-$(printf '%02d' $((i % 10)))"
    cp -a "$seed" "$OUT/corpus/batch-$(printf '%02d' $((i % 10)))/chapter-${i}.txt"
  done

  if [[ "$lab" != "1" ]]; then
    local probe_start probe_elapsed
    probe_start=$(date +%s)
    perl "$OUT/app/ack" --nocolor --nogroup -i 'elizabeth|darcy|truth' "$OUT/corpus" \
      >"$OUT/meta/ack-unprofiled.txt" || true
    probe_elapsed=$(( $(date +%s) - probe_start ))
    echo "unprofiled_ack_secs=${probe_elapsed}" >"$OUT/meta/timings.txt"
    log "unprofiled ack wall ${probe_elapsed}s"

    local factor=1
    if [[ "$probe_elapsed" -le 0 ]]; then
      factor=3
    elif [[ "$probe_elapsed" -eq 1 ]]; then
      factor=2
    fi
    if [[ "$factor" -gt 1 ]]; then
      log "replicating corpus ×${factor} so profiled run is ~${TARGET_SECS}s"
      local src="$OUT/corpus"
      local extra="$OUT/corpus-scaled"
      mkdir -p "$extra"
      local r
      for r in $(seq 1 "$factor"); do
        mkdir -p "$extra/rep-${r}"
        cp -a "$src/." "$extra/rep-${r}/"
      done
      rm -rf "$src"
      mv "$extra" "$src"
    fi
  fi

  find "$OUT/corpus" -type f | wc -l >"$OUT/meta/corpus-file-count.txt"
  du -sh "$OUT/corpus" >"$OUT/meta/corpus-size.txt"

  local scanner_src="/src/scripts/field/workloads/minute_text_scanner.pl"
  [[ -f "$scanner_src" ]] || fail "missing $scanner_src (repo not mounted at /src?)"
  cp -a "$scanner_src" "$OUT/app/minute_text_scanner.pl"
  chmod 755 "$OUT/app/minute_text_scanner.pl"
  perl -c "$OUT/app/minute_text_scanner.pl" >"$OUT/meta/scanner-syntax.txt" 2>&1 \
    || fail "scanner -c failed (see $OUT/meta/scanner-syntax.txt)"

  : >"$OUT/meta/timings.txt"
  echo "target_secs=${TARGET_SECS}" >>"$OUT/meta/timings.txt"
  echo "lab=${lab}" >>"$OUT/meta/timings.txt"

  if [[ "$lab" != "1" && -x "$OUT/app/ack" ]]; then
    set +e
    NYTPROF="file=${OUT}/nytprof-ack-attempt.out" \
      perl -d:NYTProfM "$OUT/app/ack" --nocolor --nogroup -i 'truth' \
      "$OUT/corpus/batch-01" \
      >"$OUT/meta/ack-profiled.txt" \
      2>"$OUT/meta/ack-profiled.err"
    local ack_rc=$?
    set -e
    echo "profiled_ack_rc=${ack_rc}" >>"$OUT/meta/timings.txt"
  fi

  local attach_perl=(perl)
  if [[ "$lab" == "1" && -d /src/collector ]]; then
    log "lab: building in-tree product XS for Rocky attach (not testdrive RPM .so)"
    yum -y install gcc make perl-devel perl-ExtUtils-ParseXS perl-ExtUtils-Embed zlib-devel \
      >>"$OUT/meta/yum-install.log" 2>&1 \
      || fail "yum install XS build deps failed"
    rm -rf /tmp/nytprof-collector
    mkdir -p /tmp/nytprof-collector
    # Do not copy host collector/build — those .so files need host glibc.
    tar -C /src/collector -cf - \
      --exclude=build --exclude='*.o' --exclude='*.so' --exclude='*.a' \
      . | tar -C /tmp/nytprof-collector -xf -
    make -C /tmp/nytprof-collector xs-nytprof \
      >"$OUT/meta/xs-nytprof.log" 2>&1 \
      || fail "in-tree xs-nytprof failed (see meta/xs-nytprof.log)"
    local xsdest=/tmp/nytprof-collector/build/xs-nytprof
    [[ -f "$xsdest/auto/Devel/NYTProfM/NYTProfM.so" ]] \
      || fail "xs-nytprof missing NYTProfM.so"
    attach_perl=(perl -I"$xsdest")
    echo "attach_perl5lib=$xsdest" >>"$OUT/meta/timings.txt"
  fi

  log "profiling minute_text_scanner.pl with perl -d:NYTProfM (~${TARGET_SECS}s wall)"
  local prof_start prof_elapsed
  prof_start=$(date +%s)
  set +e
  NYTPROF="file=${OUT}/nytprof.out" \
    "${attach_perl[@]}" -d:NYTProfM "$OUT/app/minute_text_scanner.pl" \
    "$OUT/corpus" "$TARGET_SECS" \
    >"$OUT/meta/scanner-profiled.txt" \
    2>"$OUT/meta/scanner-profiled.err"
  local prof_rc=$?
  set -e
  prof_elapsed=$(( $(date +%s) - prof_start ))
  {
    echo "profiled_scanner_secs=${prof_elapsed}"
    echo "profiled_scanner_rc=${prof_rc}"
    echo "primary_profile=minute_text_scanner"
  } >>"$OUT/meta/timings.txt"
  log "profiled scanner wall ${prof_elapsed}s rc=${prof_rc}"

  [[ "$prof_rc" -eq 0 ]] \
    || fail "scanner profile failed (rc=$prof_rc); see meta/scanner-profiled.err"
  [[ -s "$OUT/nytprof.out" ]] || fail "missing nytprof.out"
  ls -l "$OUT/nytprof.out" >"$OUT/meta/nytprof-out-ls.txt"

  log "generating HTML report"
  local html_start html_elapsed
  html_start=$(date +%s)
  if command -v nytprofm-cli >/dev/null; then
    nytprofm-cli html "$OUT/nytprof.out" --out-dir "$OUT/html" \
      >"$OUT/meta/nytprofm-cli-html.out" 2>"$OUT/meta/nytprofm-cli-html.err" \
      || fail "nytprofm-cli html failed (see $OUT/meta/nytprofm-cli-html.err)"
    html_elapsed=$(( $(date +%s) - html_start ))
    echo "html_secs=${html_elapsed}" >>"$OUT/meta/timings.txt"
    [[ -f "$OUT/html/index.html" ]] \
      || fail "nytprofm-cli html did not write html/index.html"
    nytprofm-cli report "$OUT/nytprof.out" \
      >"$OUT/meta/report.txt" 2>"$OUT/meta/report.err" || true
    nytprofm-cli verify "$OUT/nytprof.out" \
      >"$OUT/meta/verify.txt" 2>"$OUT/meta/verify.err" || true
  else
    echo "SKIP: nytprofm-cli not on PATH — no native HTML from this RPM" \
      | tee "$OUT/meta/html.skip"
    echo "html_secs=0" >>"$OUT/meta/timings.txt"
  fi

  write_notes "$OUT"
  # Host bind-mounts this dir; drop a convenience launcher.
  cat >"$OUT/open-report.sh" <<'EOS'
#!/usr/bin/env bash
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
xdg-open "$DIR/html/index.html" 2>/dev/null || \
  sensible-browser "$DIR/html/index.html" 2>/dev/null || \
  echo "Open $DIR/html/index.html in a browser"
EOS
  chmod 755 "$OUT/open-report.sh"

  ok "profile + HTML in $OUT"
  ok "open $OUT/html/index.html"
}

# Oracle 6.15 inside a container. Mounts are archive + scanner + vendor only
# (see host dispatch). Never inherit host PERL5LIB. Never source env.sh.
run_oracle_inside() {
  local OUT="$1"
  [[ -n "$OUT" ]] || fail "--inside-oracle requires an output directory"
  mkdir -p "$OUT"/{html,meta,app,corpus}

  unset PERL5LIB
  export PERL5LIB=""

  log "oracle: installing Rocky 8 build packages"
  yum -y install \
    perl perl-libs perl-interpreter \
    perl-ExtUtils-MakeMaker perl-devel \
    gcc make zlib-devel gzip tar \
    which findutils ca-certificates \
    >"$OUT/meta/yum-install.log" 2>&1 \
    || fail "oracle yum install failed (see $OUT/meta/yum-install.log)"

  local archive="/oracle-src/Devel-NYTProf-6.15.tar.gz"
  local scanner_src="/oracle-src/minute_text_scanner.pl"
  local vendor="/oracle-src/vendor"
  [[ -f "$archive" ]] || fail "missing $archive (archive not mounted)"
  [[ -f "$scanner_src" ]] || fail "missing $scanner_src (scanner not mounted)"

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
      || fail "oracle extract missing Makefile.PL (see meta/oracle-extract.log)"
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
  PERL5LIB="${prefix}/lib/perl5/${arch}:${prefix}/lib/perl5"
  if [[ -d "$vendor" ]]; then
    PERL5LIB="${PERL5LIB}:${vendor}"
  fi
  export PERL5LIB
  echo "PERL5LIB=${PERL5LIB}" >"$OUT/meta/perl5lib.txt"

  case ":${PERL5LIB}:" in
    *"/crates/"*|*"crates/"*)
      fail "oracle PERL5LIB must not contain crates/ (got: $PERL5LIB)"
      ;;
  esac
  case ":${PERL5LIB}:" in
    *"/baseline/6.15/install"*|*"baseline/6.15/install"*)
      fail "oracle PERL5LIB must not contain baseline/6.15/install (got: $PERL5LIB)"
      ;;
  esac

  command -v perl >/dev/null || fail "perl missing"
  [[ -x "$prefix/bin/nytprofhtml" ]] || fail "nytprofhtml missing after install"
  perl -MDevel::NYTProf -e 'print $Devel::NYTProf::VERSION, "\n"' \
    >"$OUT/meta/nytprof-version.txt" \
    || fail "Devel::NYTProf did not load from oracle prefix"

  {
    echo "=== /etc/os-release ==="
    cat /etc/os-release
    echo
    echo "=== perl -V ==="
    perl -V
    echo
    echo "=== PERL5LIB ==="
    echo "$PERL5LIB"
  } >"$OUT/meta/environment.txt" 2>&1

  local seed="$OUT/meta/seed.txt"
  perl -e '
    print "It is a truth universally acknowledged that a profiler in want of a report must be in search of a long-running Perl application.\n" x 400;
    for my $i (1..200) {
      print "Record $i: sub process { my (\$line) = @_; \$line =~ s/\\s+/ /g; return length \$line }\n";
    }
  ' >"$seed"
  [[ -s "$seed" ]] || fail "empty oracle corpus seed"
  local copies=2
  local i
  for i in $(seq 1 "$copies"); do
    mkdir -p "$OUT/corpus/batch-$(printf '%02d' $((i % 10)))"
    cp -a "$seed" "$OUT/corpus/batch-$(printf '%02d' $((i % 10)))/chapter-${i}.txt"
  done

  cp -a "$scanner_src" "$OUT/app/minute_text_scanner.pl"
  chmod 755 "$OUT/app/minute_text_scanner.pl"
  perl -c "$OUT/app/minute_text_scanner.pl" >"$OUT/meta/scanner-syntax.txt" 2>&1 \
    || fail "oracle scanner -c failed"

  : >>"$OUT/meta/timings.txt"
  echo "target_secs=${TARGET_SECS}" >>"$OUT/meta/timings.txt"
  echo "lab=${NYTPROF_DEMO_LAB:-0}" >>"$OUT/meta/timings.txt"
  echo "engine=oracle" >>"$OUT/meta/timings.txt"
  echo "primary_profile=minute_text_scanner" >>"$OUT/meta/timings.txt"

  PATH="${prefix}/bin:${PATH}"
  export PATH

  log "oracle: profiling minute_text_scanner.pl with perl -d:NYTProf (~${TARGET_SECS}s wall)"
  local prof_start prof_elapsed
  prof_start=$(date +%s)
  set +e
  NYTPROF="file=${OUT}/nytprof.out" \
    perl -d:NYTProf "$OUT/app/minute_text_scanner.pl" \
    "$OUT/corpus" "$TARGET_SECS" \
    >"$OUT/meta/scanner-profiled.txt" \
    2>"$OUT/meta/scanner-profiled.err"
  local prof_rc=$?
  set -e
  prof_elapsed=$(( $(date +%s) - prof_start ))
  {
    echo "profiled_scanner_secs=${prof_elapsed}"
    echo "profiled_scanner_rc=${prof_rc}"
  } >>"$OUT/meta/timings.txt"
  log "oracle scanner wall ${prof_elapsed}s rc=${prof_rc}"
  [[ "$prof_rc" -eq 0 ]] \
    || fail "oracle scanner profile failed (rc=$prof_rc); see meta/scanner-profiled.err"
  [[ -s "$OUT/nytprof.out" ]] || fail "missing oracle nytprof.out"

  log "oracle: nytprofhtml"
  local html_start html_elapsed
  html_start=$(date +%s)
  nytprofhtml -o "$OUT/html" -f "$OUT/nytprof.out" \
    >"$OUT/meta/nytprofhtml.out" 2>"$OUT/meta/nytprofhtml.err" \
    || fail "oracle nytprofhtml failed (see meta/nytprofhtml.err)"
  html_elapsed=$(( $(date +%s) - html_start ))
  echo "html_secs=${html_elapsed}" >>"$OUT/meta/timings.txt"
  [[ -f "$OUT/html/index.html" ]] \
    || fail "oracle nytprofhtml did not write html/index.html"

  cat >"$OUT/NOTES.txt" <<EOF
NYTProf 6.15 oracle Rocky Docker profile
=======================================

Date (UTC):     $(date -u +%Y-%m-%dT%H:%M:%SZ)
Image:          ${IMAGE}
Target wall:    ${TARGET_SECS}s under perl -d:NYTProf
Prefix:         $prefix
PERL5LIB:       $PERL5LIB

Isolation: archive + scanner + File::Which only. No crates/ on PERL5LIB.
No tools/oracle/env.sh. No baseline/6.15/install.
EOF

  ok "oracle profile + HTML in $OUT"
}

# Move a leftover real directory/file at $1 aside, then ln -sfn $2 $1.
# GNU ln -sfn does not replace an existing directory (it nests a symlink inside).
migrate_then_link() {
  local dest="$1" target="$2"
  if [[ -L "$dest" ]]; then
    ln -sfn "$target" "$dest"
    return 0
  fi
  if [[ -d "$dest" ]]; then
    local parent
    parent="$(dirname "$dest")"
    mkdir -p "$parent/native"
    local name
    name="$(basename "$dest")"
    if [[ -e "$parent/native/$name" ]]; then
      rm -rf "$dest"
    else
      mv "$dest" "$parent/native/$name"
    fi
  elif [[ -e "$dest" ]]; then
    local parent name
    parent="$(dirname "$dest")"
    name="$(basename "$dest")"
    mkdir -p "$parent/native"
    if [[ -e "$parent/native/$name" ]]; then
      rm -f "$dest"
    else
      mv "$dest" "$parent/native/$name"
    fi
  fi
  ln -sfn "$target" "$dest"
}

# --- host / dispatch --------------------------------------------------------

OUT="$DEFAULT_OUT"
INSIDE=0
INSIDE_ORACLE=0
LAB="${NYTPROF_DEMO_LAB:-0}"
SECONDS_SET=0
ENGINE="${NYTPROF_DEMO_ENGINE:-native}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out)
      [[ $# -ge 2 ]] || fail "--out requires a path"
      OUT="$2"
      shift 2
      ;;
    --lab)
      LAB=1
      shift
      ;;
    --seconds)
      [[ $# -ge 2 ]] || fail "--seconds requires an integer"
      TARGET_SECS="$2"
      SECONDS_SET=1
      shift 2
      ;;
    --engine)
      [[ $# -ge 2 ]] || fail "--engine requires native|oracle|both"
      ENGINE="$2"
      shift 2
      ;;
    --inside)
      INSIDE=1
      if [[ $# -ge 2 && "$2" != -* ]]; then
        OUT="$2"
        shift 2
      else
        shift
      fi
      ;;
    --inside-oracle)
      INSIDE_ORACLE=1
      if [[ $# -ge 2 && "$2" != -* ]]; then
        OUT="$2"
        shift 2
      else
        shift
      fi
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "unknown argument: $1"
      ;;
  esac
done

if [[ "$LAB" == "1" && "$SECONDS_SET" -eq 0 && "${NYTPROF_DEMO_SECONDS:-}" == "" ]]; then
  TARGET_SECS=3
fi
export NYTPROF_DEMO_LAB="$LAB"
export NYTPROF_DEMO_ENGINE="$ENGINE"

case "$ENGINE" in
  native|oracle|both) ;;
  *) fail "--engine must be native, oracle, or both (got: $ENGINE)" ;;
esac

if [[ "$INSIDE" -eq 1 ]]; then
  run_inside "$OUT"
  exit 0
fi
if [[ "$INSIDE_ORACLE" -eq 1 ]]; then
  run_oracle_inside "$OUT"
  exit 0
fi

command -v docker >/dev/null 2>&1 || fail "docker is required on the host"
docker info >/dev/null 2>&1 || fail "docker daemon is not reachable"

mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"

local_rpm="${NYTPROF_DEMO_RPM:-}"
if [[ -z "$local_rpm" && -f "$ROOT/dist/el8/${RPM_NAME}" ]]; then
  local_rpm="$ROOT/dist/el8/${RPM_NAME}"
fi

ORACLE_IMAGE="${NYTPROF_ORACLE_IMAGE:-$IMAGE}"
ARCHIVE="$ROOT/baseline/6.15/archives/Devel-NYTProf-6.15.tar.gz"
SCANNER="$ROOT/scripts/field/workloads/minute_text_scanner.pl"
WHICH_PM="$ROOT/baseline/6.15/test-deps/lib/perl5/File/Which.pm"

run_native_host() {
  local dest="$1"
  mkdir -p "$dest"
  log "re-exec native in $IMAGE → $dest"
  local docker_args=(
    run --rm
    -e NYTPROF_DEMO_SECONDS="$TARGET_SECS"
    -e NYTPROF_DEMO_LAB="$LAB"
    -e NYTPROF_DEMO_ACK_URL="$ACK_URL"
    -e NYTPROF_DEMO_ACK_FALLBACK_URL="$ACK_FALLBACK_URL"
    -e NYTPROF_DEMO_TEXT_URL="$TEXT_URL"
    -e NYTPROF_DEMO_RPM_URL="$RELEASE_RPM_URL"
    -v "$ROOT:/src:ro"
    -v "$dest:/out:rw"
  )
  if [[ -n "$local_rpm" ]]; then
    docker_args+=(-v "$local_rpm:/rpm/${RPM_NAME}:ro")
    log "RPM bind-mount $local_rpm"
  else
    log "no local RPM; container will download $RELEASE_RPM_URL"
  fi
  docker "${docker_args[@]}" \
    "$IMAGE" \
    bash /src/scripts/field/rocky8_docker_profile_demo.sh --inside /out
  if [[ "$(id -u)" -ne 0 ]]; then
    docker run --rm -v "$dest:/out:rw" "$IMAGE" \
      chown -R "$(id -u):$(id -g)" /out \
      || log "warning: could not chown $dest"
  fi
  [[ -f "$dest/html/index.html" ]] || fail "expected $dest/html/index.html"
  [[ -s "$dest/nytprof.out" ]] || fail "expected $dest/nytprof.out"
}

run_oracle_host() {
  local dest="$1"
  mkdir -p "$dest"
  [[ -f "$ARCHIVE" ]] || fail "missing oracle archive $ARCHIVE"
  [[ -f "$SCANNER" ]] || fail "missing $SCANNER"
  if [[ ! -f "$WHICH_PM" ]]; then
    # test-deps/ is gitignored; GHA checkouts do not have File::Which.
    log "SKIP: File::Which not in checkout ($WHICH_PM) — oracle half not run"
    mkdir -p "$dest/meta"
    echo "oracle_skip=1 missing File::Which (baseline test-deps gitignored)" \
      >"$dest/meta/oracle-skip.txt"
    echo "oracle_skip=1" >>"$dest/meta/timings.txt"
    return 0
  fi

  log "re-exec oracle in $ORACLE_IMAGE → $dest"
  # Isolation: archive + scanner + File::Which only — never the repo root.
  local docker_args=(
    run --rm
    -e PERL5LIB=
    -e NYTPROF_DEMO_SECONDS="$TARGET_SECS"
    -e NYTPROF_DEMO_LAB="$LAB"
    -v nytprof-oracle-prefix:/opt/nytprof-oracle
    -v "$ARCHIVE:/oracle-src/Devel-NYTProf-6.15.tar.gz:ro"
    -v "$SCANNER:/oracle-src/minute_text_scanner.pl:ro"
    -v "$WHICH_PM:/oracle-src/vendor/File/Which.pm:ro"
    -v "$dest:/out:rw"
    -v "$ROOT/scripts/field/rocky8_docker_profile_demo.sh:/oracle-src/demo.sh:ro"
  )
  set +e
  docker "${docker_args[@]}" \
    "$ORACLE_IMAGE" \
    bash /oracle-src/demo.sh --inside-oracle /out
  local orc=$?
  set -e
  if [[ "$(id -u)" -ne 0 ]]; then
    docker run --rm -v "$dest:/out:rw" "$ORACLE_IMAGE" \
      chown -R "$(id -u):$(id -g)" /out \
      || log "warning: could not chown $dest"
  fi
  if [[ "$orc" -ne 0 ]]; then
    log "SKIP: oracle container failed (rc=$orc) — not faking index.html"
    mkdir -p "$dest/meta"
    echo "oracle_skip=1" >>"$dest/meta/timings.txt"
    echo "oracle_skip=1 rc=$orc" >>"$dest/meta/oracle-skip.txt"
    return 0
  fi
  [[ -f "$dest/html/index.html" ]] || fail "oracle missing html/index.html after success"
  return 0
}

case "$ENGINE" in
  native)
    run_native_host "$OUT"
    ;;
  oracle)
    run_oracle_host "$OUT/oracle"
    [[ -f "$OUT/oracle/html/index.html" ]] \
      || fail "oracle half skipped or missing; --engine oracle requires a report"
    ;;
  both)
    run_native_host "$OUT/native"
    run_oracle_host "$OUT/oracle"
    migrate_then_link "$OUT/html" "native/html"
    migrate_then_link "$OUT/meta" "native/meta"
    migrate_then_link "$OUT/nytprof.out" "native/nytprof.out"
    [[ -L "$OUT/html" ]] || fail "KD-LAYOUT: $OUT/html must be a symlink after --engine both"
    [[ -f "$OUT/html/index.html" ]] || fail "expected $OUT/html/index.html via native symlink"
    ;;
esac

ok "Rocky 8 profile report is in $OUT"
if [[ -f "$OUT/html/index.html" ]]; then
  ok "open $OUT/html/index.html"
  ls -lh "$OUT/nytprof.out" "$OUT/html/index.html" 2>/dev/null || true
fi
if [[ -f "$OUT/oracle/html/index.html" ]]; then
  ok "oracle HTML $OUT/oracle/html/index.html"
fi
if [[ -f "$OUT/meta/timings.txt" ]]; then
  log "--- timings ---"
  cat "$OUT/meta/timings.txt"
fi
