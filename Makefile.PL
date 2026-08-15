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
#   NYTPROF_NATIVE=1     require cargo for configure; make all → native-install (I02)
#   NYTPROF_NATIVE=auto  make all → native-install when cargo present; cargo-free if absent
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

print "NYTProfM / Devel::NYTProfM packaging entry (J01 hygiene; BUILD-MAKEMAKER-OPT)\n";
print "  mode: $native_note\n";
print "  honesty: NAME/VERSION from product module — NOT full XS CPAN dual-build (BUILD-003); CPAN-TRIAL residual\n";
print "  isolation: never put crates/ on oracle PERL5LIB\n";
print "  scripts: $pack\n";
print "  targets: legacy-smoke dual-path-smoke offline-gate native-install native install-product-xs product-legacy-smoke install-product-scripts i03-dist-scripts-smoke j01-cpan-hygiene-smoke j02-cpan-trial-notes-smoke rocky8-docker-lab-smoke\n";
if ( $native_mode eq 'off' ) {
    print "  default make all/test → legacy path (no cargo)\n";
}
elsif ( $native_mode eq 'on' ) {
    print "  default make all → native-install (cargo required)\n";
}
else {
    print "  default make all → native-install (NYTPROF_NATIVE=auto + cargo)\n";
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
    print {$fh} "packaging_i02=1\n";
    print {$fh} "packaging_i03=1\n";
    print {$fh} "packaging_j01=1\n";
    print {$fh} "packaging_j02=1\n";
    print {$fh} "full_build003=0\n";
    print {$fh} "cpan_trial_ready=0\n";
    print {$fh} "cpan_trial_notes=1\n";
    print {$fh} "cpan_trial_uploaded=0\n";
    close $fh;
}

# Expose mode to postamble BEFORE WriteMakefile (postamble runs during configure).
$main::NYTPROF_PACK_NATIVE_MODE = $native_mode;

# J01: advertised dist identity is the coordinated CPAN name + product $VERSION
# from the real debugger module (not PackagingEntry / 0.001). PM stays empty —
# this entry still only exposes packaging targets; not BUILD-003-FULL / not TRIAL.
my $product_pm = 'collector/xs/Devel/NYTProfM.pm';
WriteMakefile(
    NAME             => 'Devel::NYTProfM',
    DISTNAME         => 'NYTProfM',
    VERSION_FROM     => $product_pm,
    ABSTRACT         => 'NYTProfM product packaging entry (J01 hygiene; not full XS CPAN / not TRIAL)',
    AUTHOR           => 'nytprof-modernization contributors',
    LICENSE          => 'perl',
    MIN_PERL_VERSION => '5.008001',
    PM               => {},
    EXE_FILES        => [],
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
            board_id           => 'CPAN-TRIAL-READY',
            packaging_entry    => 1,
            full_xs_cpan       => 0,
            cpan_trial_ready   => 0,
            cpan_trial_notes   => 1,
            cpan_trial_uploaded => 0,
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
    my $prod_xs_sh   = 'scripts/packaging/install_product_xs.sh';
    my $prod_leg_sh  = 'scripts/packaging/product_legacy_smoke.sh';
    my $prod_scr_sh  = 'scripts/packaging/install_product_scripts.sh';
    my $i03_sm_sh    = 'scripts/packaging/i03_dist_scripts_smoke.sh';
    my $j01_sm_sh    = 'scripts/packaging/j01_cpan_hygiene_smoke.sh';
    my $j02_sm_sh    = 'scripts/packaging/j02_cpan_trial_notes_smoke.sh';
    my $k01_sm_sh    = 'scripts/packaging/k01_el8_module_rpm_smoke.sh';
    my $k02_sm_sh    = 'scripts/packaging/k02_el8_tools_rpm_smoke.sh';
    my $p01_sm_sh    = 'scripts/packaging/p01_ga_candidate_smoke.sh';
    my $p02_sm_sh    = 'scripts/packaging/p02_sec_cut_smoke.sh';
    my $r8lab_sm_sh  = 'scripts/field/rocky8_docker_profile_smoke.sh';

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
    elsif ( $mode eq 'optional' ) {
        $all_body = $lines->(
            '$(NOECHO) $(ECHO) packaging entry: NYTPROF_NATIVE=auto + cargo - installing nytprof-cli',
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
        '$(NOECHO) $(ECHO) "  make install-product-xs  # I01 cargo-free product Devel::NYTProfM XS"',
        '$(NOECHO) $(ECHO) "  make product-legacy-smoke # I01 P-PRODUCT-LEGACY install+attach"',
        '$(NOECHO) $(ECHO) "  make install-product-scripts # I03 cargo-free EngineDispatch + nytprofhtml/csv"',
        '$(NOECHO) $(ECHO) "  make i03-dist-scripts-smoke  # I03 prefix scripts + query 15/3/15"',
        '$(NOECHO) $(ECHO) "  make j01-cpan-hygiene-smoke  # J01 META NYTProfM 6.15"',
        '$(NOECHO) $(ECHO) "  make j02-cpan-trial-notes-smoke # J02 TRIAL notes-ready (not PAUSE)"',
        '$(NOECHO) $(ECHO) "  make k01-el8-module-rpm-smoke # K01 EL8 D1-B module spec"',
        '$(NOECHO) $(ECHO) "  make k02-el8-tools-rpm-smoke  # K02 EL8 nytprof-cli companion spec"',
        '$(NOECHO) $(ECHO) "  make p01-ga-candidate-smoke   # P01 GA-candidate honesty"',
        '$(NOECHO) $(ECHO) "  make p02-sec-cut-smoke        # P02 SEC-012 checklist + SEC-002 job MVP"',
        '$(NOECHO) $(ECHO) "  make rocky8-docker-lab-smoke  # Rocky 8 Docker profile lab (honest docker SKIP)"',
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
    my $prod_xs_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make install-product-xs -> ' . $prod_xs_sh . ' ==="',
        'bash ' . $prod_xs_sh,
    );
    my $prod_leg_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make product-legacy-smoke -> ' . $prod_leg_sh . ' ==="',
        'bash ' . $prod_leg_sh,
    );
    my $prod_scr_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make install-product-scripts -> ' . $prod_scr_sh . ' ==="',
        'bash ' . $prod_scr_sh,
    );
    my $i03_sm_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make i03-dist-scripts-smoke -> ' . $i03_sm_sh . ' ==="',
        'bash ' . $i03_sm_sh,
    );
    my $j01_sm_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make j01-cpan-hygiene-smoke -> ' . $j01_sm_sh . ' ==="',
        'bash ' . $j01_sm_sh,
    );
    my $j02_sm_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make j02-cpan-trial-notes-smoke -> ' . $j02_sm_sh . ' ==="',
        'bash ' . $j02_sm_sh,
    );
    my $k01_sm_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make k01-el8-module-rpm-smoke -> ' . $k01_sm_sh . ' ==="',
        'bash ' . $k01_sm_sh,
    );
    my $k02_sm_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make k02-el8-tools-rpm-smoke -> ' . $k02_sm_sh . ' ==="',
        'bash ' . $k02_sm_sh,
    );
    my $p01_sm_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make p01-ga-candidate-smoke -> ' . $p01_sm_sh . ' ==="',
        'bash ' . $p01_sm_sh,
    );
    my $p02_sm_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make p02-sec-cut-smoke -> ' . $p02_sm_sh . ' ==="',
        'bash ' . $p02_sm_sh,
    );
    my $r8lab_sm_body = $lines->(
        '$(NOECHO) $(ECHO) "=== make rocky8-docker-lab-smoke -> ' . $r8lab_sm_sh . ' ==="',
        'bash ' . $r8lab_sm_sh,
    );

    return <<"MAKE_FRAG";

# ---------------------------------------------------------------------------
# BUILD-MAKEMAKER-OPT packaging targets (thin wrappers over scripts/packaging/)
# Never put crates/ on oracle PERL5LIB; child smokes own isolation.
# ---------------------------------------------------------------------------

.PHONY: legacy-smoke dual-path-smoke offline-gate collector-sink-smoke native-install native native-smoke packaging-help packaging-all install-product-xs product-legacy-smoke install-product-scripts i03-dist-scripts-smoke j01-cpan-hygiene-smoke j02-cpan-trial-notes-smoke k01-el8-module-rpm-smoke k02-el8-tools-rpm-smoke p01-ga-candidate-smoke p02-sec-cut-smoke rocky8-docker-lab-smoke

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
install-product-xs:
$prod_xs_body
product-legacy-smoke:
$prod_leg_body
install-product-scripts:
$prod_scr_body
i03-dist-scripts-smoke:
$i03_sm_body
j01-cpan-hygiene-smoke:
$j01_sm_body
j02-cpan-trial-notes-smoke:
$j02_sm_body
k01-el8-module-rpm-smoke:
$k01_sm_body
k02-el8-tools-rpm-smoke:
$k02_sm_body
p01-ga-candidate-smoke:
$p01_sm_body
p02-sec-cut-smoke:
$p02_sm_body
rocky8-docker-lab-smoke:
$r8lab_sm_body
MAKE_FRAG
}
