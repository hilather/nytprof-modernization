#!/usr/bin/env perl
# Candidate packaging entry (BUILD-MAKEMAKER-OPT).
#
# This is a thin MakeMaker *facade* for the modernization repo dual-path
# packaging layout. It is NOT a complete CPAN tarball of Devel::NYTProf XS
# and does not build the oracle collector under baseline/6.15/src.
#
# Policy: docs/BUILD_SUPPORT_POLICY.md
# Spike:  docs/PACKAGING_SPIKE.md
#
# Controls:
#   NYTPROF_NATIVE=0     legacy-only (default) — no Cargo on the critical path
#   NYTPROF_NATIVE=1     require cargo for configure; enable native targets as default all
#   NYTPROF_NATIVE=auto  enable native targets when cargo is present; else legacy
#
# Typical use (from repo root):
#   perl Makefile.PL                 # default: legacy
#   make legacy-smoke                # scripts/packaging/legacy_only_smoke.sh
#   make dual-path-smoke             # scripts/packaging/dual_path_smoke.sh
#   make offline-gate                # scripts/ci/offline_gate.sh (CI-OFFLINE-GATE-EXPAND)
#   make native-install              # scripts/packaging/install_native.sh (needs cargo)
#   NYTPROF_NATIVE=1 perl Makefile.PL && make native-install
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

print "nytprof-modernization packaging entry (BUILD-MAKEMAKER-OPT)\n";
print "  mode: $native_note\n";
print "  honesty: candidate facade — NOT full Devel::NYTProf XS / CPAN dual-build (BUILD-003)\n";
print "  isolation: never put crates/ on oracle PERL5LIB\n";
print "  scripts: $pack\n";
print "  targets: legacy-smoke dual-path-smoke offline-gate native-install native\n";
if ( $native_mode eq 'off' ) {
    print "  default make all/test → legacy path (no cargo)\n";
}
elsif ( $native_mode eq 'on' ) {
    print "  default make all → native-install (cargo required)\n";
}
else {
    print "  default make all → legacy path; make native-install available\n";
}

# Write a tiny stamp the generated Makefile can quote (also aids smoke checks).
my $stamp = File::Spec->catfile( $repo_root, 'nytprof-packaging.mode' );
{
    open my $fh, '>', $stamp or die "Cannot write $stamp: $!\n";
    print {$fh} "NYTPROF_NATIVE=$native_env\n";
    print {$fh} "native_mode=$native_mode\n";
    print {$fh} "cargo_present=" . ( $cargo_present ? '1' : '0' ) . "\n";
    print {$fh} "packaging_entry=BUILD-MAKEMAKER-OPT\n";
    print {$fh} "not_full_xs_cpan=1\n";
    close $fh;
}

# Expose mode to postamble BEFORE WriteMakefile (postamble runs during configure).
$main::NYTPROF_PACK_NATIVE_MODE = $native_mode;

# Thin WriteMakefile: no XS, no oracle sources, no claim to ship full NYTProf.
# PM is empty — this entry only exposes packaging targets via postamble.
WriteMakefile(
    NAME             => 'NYTProf::Modernization::PackagingEntry',
    VERSION          => '0.001',
    ABSTRACT         => 'Candidate MakeMaker dual-path packaging entry (not full Devel::NYTProf XS)',
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
            board_id           => 'BUILD-MAKEMAKER-OPT',
            packaging_entry    => 1,
            full_xs_cpan       => 0,
            nytprof_native_env => $native_env,
            native_mode        => $native_mode,
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
    my $native_sm_sh = 'scripts/packaging/native_install_smoke.sh';

    # Make recipes require real TAB characters. Build lines with "\t" (not
    # backslash-t inside a heredoc, which would stay literal).
    my $T = "\t";
    my $lines = sub {
        return join '', map { $T . $_ . "\n" } @_;
    };

    my $all_body;
    if ( $mode eq 'on' ) {
        $all_body = $lines->(
            '$(NOECHO) $(ECHO) packaging entry: native required - running native-install',
            '$(MAKE) native-install',
        );
    }
    else {
        $all_body = $lines->(
            '$(NOECHO) $(ECHO) packaging entry: legacy default (no cargo on critical path)',
            '$(NOECHO) $(ECHO) mode=' . $mode . ' - use make legacy-smoke or make dual-path-smoke',
            '$(NOECHO) $(ECHO) optional: NYTPROF_NATIVE=1 perl Makefile.PL && make native-install',
        );
    }

    my $help_body = $lines->(
        '$(NOECHO) $(ECHO) "Candidate packaging entry (not full XS CPAN)"',
        '$(NOECHO) $(ECHO) "  make legacy-smoke      # no cargo"',
        '$(NOECHO) $(ECHO) "  make dual-path-smoke   # legacy + optional native"',
        '$(NOECHO) $(ECHO) "  make offline-gate      # CI-OFFLINE-GATE-EXPAND (cargo? + harness + dual-path + auto-fallback + JsonlData + collector sink)"',
        '$(NOECHO) $(ECHO) "  make collector-sink-smoke  # COL-001 overlay (CC? honest skip)"',
        '$(NOECHO) $(ECHO) "  make native-install    # requires cargo"',
        '$(NOECHO) $(ECHO) "  make native            # alias for native-install"',
        '$(NOECHO) $(ECHO) "  make test              # legacy-smoke (cargo-free)"',
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
    my $collector_sink_sh = 'scripts/packaging/collector_sink_smoke.sh';
    my $collector_sink_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make collector-sink-smoke -> ' . $collector_sink_sh . ' ==="',
        'bash ' . $collector_sink_sh,
    );
    my $native_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make native-install -> ' . $install_sh . ' ==="',
        '@command -v cargo >/dev/null 2>&1 || (echo "ERROR: cargo required for native-install" >&2; exit 1)',
        'bash ' . $install_sh,
    );
    my $native_smoke_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make native-smoke -> ' . $native_sm_sh . ' ==="',
        'bash ' . $native_sm_sh,
    );

    return <<"MAKE_FRAG";

# ---------------------------------------------------------------------------
# BUILD-MAKEMAKER-OPT packaging targets (thin wrappers over scripts/packaging/)
# Never put crates/ on oracle PERL5LIB; child smokes own isolation.
# ---------------------------------------------------------------------------

.PHONY: legacy-smoke dual-path-smoke offline-gate collector-sink-smoke native-install native native-smoke packaging-help packaging-all

# Hook default `all` so `perl Makefile.PL && make` is meaningful.
all :: packaging-all

packaging-all:
$all_body
packaging-help:
$help_body
legacy-smoke:
$legacy_body
dual-path-smoke:
$dual_body
offline-gate:
$offline_body
collector-sink-smoke:
$collector_sink_body
native-install:
$native_body
native: native-install

native-smoke: native-install
$native_smoke_body
MAKE_FRAG
}
