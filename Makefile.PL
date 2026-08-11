#!/usr/bin/env perl
# Candidate packaging entry (BUILD-MAKEMAKER-OPT + BUILD-003-DEPTH).
#
# This is a MakeMaker *facade* for the modernization repo dual-path packaging
# layout. It is NOT a complete CPAN tarball of Devel::NYTProf XS and does not
# build the oracle collector under baseline/6.15/src.
#
# BUILD-003-depth (this PR lane): closer dual-build wiring —
#   cargo-build, install-facade, dual-install, packaging-status, richer stamps
#   and configure diagnostics — while legacy-only remains the default and never
#   requires Cargo. full_build003=0 always (do not claim BUILD-003 complete).
#
# Policy: docs/BUILD_SUPPORT_POLICY.md
# Spike:  docs/PACKAGING_SPIKE.md
#
# Controls:
#   NYTPROF_NATIVE=0     legacy-only (default) — no Cargo on the critical path
#   NYTPROF_NATIVE=1     require cargo for configure; enable native targets as default all
#   NYTPROF_NATIVE=auto  enable native targets when cargo is present; else legacy
#
# Reserved (accepted, not implemented in this facade — BUILD-003 full / BUILD-008):
#   NYTPROF_CODEC_ZSTD=auto|0|1
#   NYTPROF_CODEC_LZ4=auto|0|1
#   (=1 prints a configure note that MakeMaker codec wiring is not yet shipped)
#
# Typical use (from repo root):
#   perl Makefile.PL                 # default: legacy
#   make legacy-smoke                # scripts/packaging/legacy_only_smoke.sh
#   make dual-path-smoke             # scripts/packaging/dual_path_smoke.sh
#   make offline-gate                # scripts/ci/offline_gate.sh
#   make install-facade              # pure-Perl engine → $NYTPROF_PREFIX or $REPO/prefix (no cargo)
#   make native-install              # scripts/packaging/install_native.sh (needs cargo)
#   make dual-install                # native-install + install-facade (shared root; needs cargo)
#   make build003-depth-smoke        # scripts/packaging/makemaker_build003_depth_smoke.sh
#   NYTPROF_NATIVE=1 perl Makefile.PL && make   # default all → dual-install
#   make native                      # alias: native-install only
#
# Never puts crates/ on oracle PERL5LIB (smokes own isolation).

use 5.008001;
use strict;
use warnings;

use ExtUtils::MakeMaker;
use File::Spec;

my $repo_root = File::Spec->rel2abs(File::Spec->curdir);
my $pack      = File::Spec->catdir( $repo_root, 'scripts', 'packaging' );

my $native_env = exists $ENV{NYTPROF_NATIVE} ? $ENV{NYTPROF_NATIVE} : '0';
$native_env = lc($native_env);
$native_env = '0' if $native_env eq '' || $native_env eq 'false' || $native_env eq 'no' || $native_env eq 'off';
$native_env = '1' if $native_env eq 'true' || $native_env eq 'yes' || $native_env eq 'on';

unless ( $native_env =~ /^(0|1|auto)$/ ) {
    die "ERROR: NYTPROF_NATIVE must be 0, 1, or auto (got '$ENV{NYTPROF_NATIVE}')\n";
}

sub cargo_on_path {
    # Avoid shell; probe common names via PATH walk.
    my $path = $ENV{PATH} // '';
    for my $dir ( split /:/, $path ) {
        next if $dir eq '';
        my $cand = File::Spec->catfile( $dir, 'cargo' );
        return 1 if -x $cand;
    }
    return 0;
}

sub first_on_path {
    my ($name) = @_;
    my $path = $ENV{PATH} // '';
    for my $dir ( split /:/, $path ) {
        next if $dir eq '';
        my $cand = File::Spec->catfile( $dir, $name );
        return $cand if -x $cand;
    }
    return;
}

sub normalize_codec_env {
    my ($raw) = @_;
    return 'auto' if !defined $raw || $raw eq '';
    my $v = lc($raw);
    $v = '0' if $v eq 'false' || $v eq 'no' || $v eq 'off';
    $v = '1' if $v eq 'true' || $v eq 'yes' || $v eq 'on';
    return $v if $v =~ /^(0|1|auto)$/;
    die "ERROR: codec env must be 0, 1, or auto (got '$raw')\n";
}

my $codec_zstd = normalize_codec_env( $ENV{NYTPROF_CODEC_ZSTD} );
my $codec_lz4  = normalize_codec_env( $ENV{NYTPROF_CODEC_LZ4} );

my $cargo_present = cargo_on_path();
my $native_mode;    # 'off' | 'on' | 'optional'
my $native_note;

if ( $native_env eq '0' ) {
    $native_mode = 'off';
    $native_note = 'legacy-only (NYTPROF_NATIVE=0); Cargo not required';
}
elsif ( $native_env eq '1' ) {
    if ( !$cargo_present ) {
        die <<'EOF';
ERROR: NYTPROF_NATIVE=1 requires cargo on PATH, but cargo was not found.

  Install a Rust toolchain, or re-run with legacy default:
    NYTPROF_NATIVE=0 perl Makefile.PL
    # or simply:
    perl Makefile.PL

  Policy: docs/BUILD_SUPPORT_POLICY.md
EOF
    }
    $native_mode = 'on';
    $native_note = 'native required (NYTPROF_NATIVE=1); cargo present';
}
else {    # auto
    if ($cargo_present) {
        $native_mode = 'optional';
        $native_note = 'native optional (NYTPROF_NATIVE=auto); cargo present';
    }
    else {
        $native_mode = 'off';
        $native_note = 'legacy-only (NYTPROF_NATIVE=auto); cargo absent — native targets disabled';
    }
}

my $cargo_ver = '';
my $rustc_ver = '';
if ($cargo_present) {
    my $cargo_bin = first_on_path('cargo');
    if ($cargo_bin) {
        chomp( $cargo_ver = qx{"$cargo_bin" --version 2>/dev/null} // '' );
    }
    my $rustc_bin = first_on_path('rustc');
    if ($rustc_bin) {
        chomp( $rustc_ver = qx{"$rustc_bin" --version 2>/dev/null} // '' );
    }
}

print "nytprof-modernization packaging entry (BUILD-MAKEMAKER-OPT + BUILD-003-DEPTH)\n";
print "  mode: $native_note\n";
print "  honesty: candidate facade — NOT full Devel::NYTProf XS / CPAN dual-build (BUILD-003 full)\n";
print "  depth: BUILD-003-depth-v0 (cargo-build / install-facade / dual-install); full_build003=0\n";
print "  isolation: never put crates/ on oracle PERL5LIB\n";
print "  scripts: $pack\n";
print "  targets: legacy-smoke dual-path-smoke offline-gate native-install native\n";
print "           cargo-build install-facade dual-install packaging-status build003-depth-smoke\n";
if ($cargo_present) {
    print "  toolchain: cargo=" . ( $cargo_ver ne '' ? $cargo_ver : 'present' ) . "\n";
    print "             rustc=" . ( $rustc_ver ne '' ? $rustc_ver : 'present-or-unknown' ) . "\n";
}
else {
    print "  toolchain: cargo absent — legacy-only critical path preserved\n";
}
if ( $codec_zstd eq '1' || $codec_lz4 eq '1' ) {
    print "  NOTE: NYTPROF_CODEC_ZSTD/LZ4=1 reserved for BUILD-003/BUILD-008 full;\n";
    print "        MakeMaker codec feature wiring is not implemented in this facade (ignored).\n";
}
else {
    print "  codecs: ZSTD=$codec_zstd LZ4=$codec_lz4 (reserved; not wired in this facade)\n";
}
if ( $native_mode eq 'off' ) {
    print "  default make all/test → legacy path (no cargo)\n";
}
elsif ( $native_mode eq 'on' ) {
    print "  default make all → dual-install (cargo required: native CLI + facade)\n";
}
else {
    print "  default make all → legacy path; make dual-install / native-install available\n";
}

# Write a tiny stamp the generated Makefile can quote (also aids smoke checks).
my $stamp = File::Spec->catfile( $repo_root, 'nytprof-packaging.mode' );
{
    open my $fh, '>', $stamp or die "Cannot write $stamp: $!\n";
    print {$fh} "NYTPROF_NATIVE=$native_env\n";
    print {$fh} "native_mode=$native_mode\n";
    print {$fh} "cargo_present=" . ( $cargo_present ? '1' : '0' ) . "\n";
    print {$fh} "packaging_entry=BUILD-MAKEMAKER-OPT\n";
    print {$fh} "packaging_depth=BUILD-003-depth-v0\n";
    print {$fh} "full_build003=0\n";
    print {$fh} "not_full_xs_cpan=1\n";
    print {$fh} "NYTPROF_CODEC_ZSTD=$codec_zstd\n";
    print {$fh} "NYTPROF_CODEC_LZ4=$codec_lz4\n";
    print {$fh} "cargo_version=$cargo_ver\n" if $cargo_ver ne '';
    print {$fh} "rustc_version=$rustc_ver\n" if $rustc_ver ne '';
    close $fh;
}

# Expose mode to postamble BEFORE WriteMakefile (postamble runs during configure).
$main::NYTPROF_PACK_NATIVE_MODE = $native_mode;

# Thin WriteMakefile: no XS, no oracle sources, no claim to ship full NYTProf.
# PM is empty — this entry only exposes packaging targets via postamble.
# Pure-Perl facade install is via make install-facade (prefix layout), not blib.
WriteMakefile(
    NAME             => 'NYTProf::Modernization::PackagingEntry',
    VERSION          => '0.002',
    ABSTRACT         => 'MakeMaker dual-path packaging entry + BUILD-003 depth (not full Devel::NYTProf XS)',
    AUTHOR           => 'nytprof-modernization contributors',
    LICENSE          => 'perl',
    MIN_PERL_VERSION => '5.008001',
    # No installable dist payload yet — packaging facade only.
    PM               => {},
    EXE_FILES        => [],
    # Avoid pulling real NYTProf into PREREQ for this candidate entry.
    PREREQ_PM        => {},
    NO_META          => 0,
    NORECURS         => 1,
    META_MERGE       => {
        'meta-spec' => { version => 2 },
        no_index    => {
            directory => [qw( baseline crates fixtures prefix target tools scripts perl )],
        },
        prereqs => {
            runtime => {
                requires => {},
            },
        },
        x_nytprof_modernization => {
            board_id           => 'BUILD-003-DEPTH',
            packaging_entry    => 1,
            packaging_depth    => 'BUILD-003-depth-v0',
            full_xs_cpan       => 0,
            full_build003      => 0,
            nytprof_native_env => $native_env,
            native_mode        => $native_mode,
            codec_zstd         => $codec_zstd,
            codec_lz4          => $codec_lz4,
            policy             => 'docs/BUILD_SUPPORT_POLICY.md',
        },
    },
    clean     => { FILES => 'nytprof-packaging.mode' },
    realclean => { FILES => 'nytprof-packaging.mode' },
);

# Cargo-free default test: do not invent a fake XS suite; run legacy-smoke only.
sub MY::test {
    my $T = "\t";
    return "test : legacy-smoke\n"
      . $T
      . '$(NOECHO) $(ECHO) OK: packaging entry test path is legacy-smoke (candidate facade; not full XS suite)'
      . "\n\n";
}

sub MY::postamble {
    my $mode = $main::NYTPROF_PACK_NATIVE_MODE // 'off';
    # Relative script paths from repo root (Makefile.PL cwd).
    my $legacy_sh    = 'scripts/packaging/legacy_only_smoke.sh';
    my $dual_sh      = 'scripts/packaging/dual_path_smoke.sh';
    my $offline_sh   = 'scripts/ci/offline_gate.sh';
    my $install_sh   = 'scripts/packaging/install_native.sh';
    my $facade_sh    = 'scripts/packaging/install_facade.sh';
    my $native_sm_sh = 'scripts/packaging/native_install_smoke.sh';
    my $depth_sm_sh  = 'scripts/packaging/makemaker_build003_depth_smoke.sh';

    # Make recipes require real TAB characters. Build lines with "\t" (not
    # backslash-t inside a heredoc, which would stay literal).
    my $T = "\t";
    my $lines = sub {
        return join '', map { $T . $_ . "\n" } @_;
    };

    my $all_body;
    if ( $mode eq 'on' ) {
        $all_body = $lines->(
            '$(NOECHO) $(ECHO) packaging entry: native required - running dual-install (CLI + facade)',
            '$(MAKE) dual-install',
        );
    }
    else {
        $all_body = $lines->(
            '$(NOECHO) $(ECHO) packaging entry: legacy default (no cargo on critical path)',
            '$(NOECHO) $(ECHO) mode=' . $mode . ' - use make legacy-smoke, install-facade, or dual-path-smoke',
            '$(NOECHO) $(ECHO) optional: NYTPROF_NATIVE=1 perl Makefile.PL && make dual-install',
        );
    }

    my $help_body = $lines->(
        '$(NOECHO) $(ECHO) "Packaging entry (BUILD-MAKEMAKER-OPT + BUILD-003-DEPTH; not full XS CPAN)"',
        '$(NOECHO) $(ECHO) "  make legacy-smoke          # no cargo"',
        '$(NOECHO) $(ECHO) "  make dual-path-smoke       # legacy + optional native"',
        '$(NOECHO) $(ECHO) "  make offline-gate          # CI-OFFLINE-GATE-EXPAND"',
        '$(NOECHO) $(ECHO) "  make install-facade        # pure-Perl engine → prefix (no cargo)"',
        '$(NOECHO) $(ECHO) "  make cargo-build           # cargo build -p nytprof-cli (needs cargo)"',
        '$(NOECHO) $(ECHO) "  make native-install        # CLI → prefix/bin (needs cargo)"',
        '$(NOECHO) $(ECHO) "  make dual-install          # native-install + install-facade"',
        '$(NOECHO) $(ECHO) "  make packaging-status      # show nytprof-packaging.mode"',
        '$(NOECHO) $(ECHO) "  make build003-depth-smoke  # closer dual-build regression"',
        '$(NOECHO) $(ECHO) "  make native / native-smoke # aliases"',
        '$(NOECHO) $(ECHO) "  make test                  # legacy-smoke (cargo-free)"',
        '$(NOECHO) $(ECHO) "  NYTPROF_NATIVE=0|1|auto perl Makefile.PL"',
    );

    my $legacy_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make legacy-smoke -> ' . $legacy_sh . ' ==="',
        'bash ' . $legacy_sh,
    );
    my $dual_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make dual-path-smoke -> ' . $dual_sh . ' ==="',
        'bash ' . $dual_sh,
    );
    my $offline_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make offline-gate -> ' . $offline_sh . ' ==="',
        'bash ' . $offline_sh,
    );
    my $cargo_build_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make cargo-build (nytprof-cli) ==="',
        '@command -v cargo >/dev/null 2>&1 || (echo "ERROR: cargo required for cargo-build" >&2; exit 1)',
        'cargo build -q -p nytprof-cli',
        '$(NOECHO) $(ECHO) OK: cargo build -p nytprof-cli',
    );
    my $native_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make native-install -> ' . $install_sh . ' ==="',
        '@command -v cargo >/dev/null 2>&1 || (echo "ERROR: cargo required for native-install" >&2; exit 1)',
        'bash ' . $install_sh,
    );
    my $facade_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make install-facade -> ' . $facade_sh . ' ==="',
        'bash ' . $facade_sh,
    );
    my $dual_install_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make dual-install (native CLI + pure-Perl facade) ==="',
        '@command -v cargo >/dev/null 2>&1 || (echo "ERROR: cargo required for dual-install (use make install-facade for facade-only)" >&2; exit 1)',
        '$(MAKE) native-install',
        '$(MAKE) install-facade',
        '$(NOECHO) $(ECHO) "OK: dual-install complete (toward BUILD-003; not full XS CPAN)"',
    );
    my $native_smoke_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make native-smoke -> ' . $native_sm_sh . ' ==="',
        'bash ' . $native_sm_sh,
    );
    my $depth_smoke_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make build003-depth-smoke -> ' . $depth_sm_sh . ' ==="',
        'bash ' . $depth_sm_sh,
    );
    my $status_body = $lines->(
        '$(NOECHO) $(ECHO) "=== packaging-status (nytprof-packaging.mode) ==="',
        '@if [ -f nytprof-packaging.mode ]; then cat nytprof-packaging.mode; else echo "missing nytprof-packaging.mode — run: perl Makefile.PL"; exit 1; fi',
        '$(NOECHO) $(ECHO) "honesty: full_build003=0 - not a complete XS CPAN dual-build"',
    );

    return <<"MAKE_FRAG";

# ---------------------------------------------------------------------------
# BUILD-MAKEMAKER-OPT + BUILD-003-DEPTH packaging targets
# Thin wrappers over scripts/packaging/. Never put crates/ on oracle PERL5LIB.
# full_build003=0 — closer dual-build only; not full Devel::NYTProf XS CPAN.
# ---------------------------------------------------------------------------

.PHONY: legacy-smoke dual-path-smoke offline-gate native-install native native-smoke \\
        cargo-build install-facade dual-install packaging-help packaging-all \\
        packaging-status build003-depth-smoke

# Hook default `all` so `perl Makefile.PL && make` is meaningful.
all :: packaging-all

packaging-all:
$all_body
packaging-help:
$help_body
packaging-status:
$status_body
legacy-smoke:
$legacy_body
dual-path-smoke:
$dual_body
offline-gate:
$offline_body
cargo-build:
$cargo_build_body
install-facade:
$facade_body
native-install:
$native_body
native: native-install

dual-install:
$dual_install_body
native-smoke: native-install
$native_smoke_body
build003-depth-smoke:
$depth_smoke_body
MAKE_FRAG
}
