package Devel::NYTProf::LegacyBridge;
# ABSTRACT: Isolated oracle (baseline/6.15) legacy report bridge for the Perl facade

use strict;
use warnings;
use Carp       qw(croak);
use Cwd        qw(abs_path);
use File::Path qw(make_path);
use File::Spec;
use File::Temp qw(tempdir tempfile);

our $VERSION = '0.001';

use Exporter qw(import);
our @EXPORT_OK = qw(
  run_legacy_report
  build_oracle_env
  verify_oracle_nytprof_load
  baseline_dir
);

# Relative layout under a repo root (BASE-001 pin).
my $BASELINE_REL = File::Spec->catdir( 'baseline', '6.15' );
my $INSTALL_REL  = File::Spec->catdir( $BASELINE_REL, 'install' );
my $PERL5LIB_REL = File::Spec->catfile( $BASELINE_REL, 'oracle-perl5lib.txt' );
my $MODULE_REL   = File::Spec->catfile( $BASELINE_REL, 'oracle-module-path.txt' );
my $DUMP_REL     = File::Spec->catfile( 'tools', 'oracle', 'dump_readstream.pl' );
my $CSV_BIN_REL  = File::Spec->catfile( $INSTALL_REL, 'bin', 'nytprofcsv' );

=head1 NAME

Devel::NYTProf::LegacyBridge - Run legacy oracle report tools with isolated PERL5LIB

=head1 SYNOPSIS

  use Devel::NYTProf::LegacyBridge qw(run_legacy_report);

  my $rc = run_legacy_report($repo_root, $profile_out);
  # exit 0 when dump_readstream succeeds (line count > 0)

  # From the facade CLI:
  #   perl -Iperl/lib perl/bin/nytprof-engine --engine=legacy report fixtures/.../nytprof.out

=head1 DESCRIPTION

This module is the B<legacy> half of the Perl engine-dispatch facade
(C<docs/schemas/perl-engine-dispatch-mvp-v0.md>). It runs pinned
Devel::NYTProf B<6.15> tools from C<baseline/6.15/install> only.

Critical isolation rules (oracle / BASE-001):

=over 4

=item *

C<PERL5LIB> for oracle loads is taken from C<baseline/6.15/oracle-perl5lib.txt>
(install-tree entries only).

=item *

B<Never> put C<crates/> or candidate C<perl/> on that C<PERL5LIB> when loading
C<Devel::NYTProf> or running C<dump_readstream.pl> / C<nytprofcsv>.

=item *

The facade modules themselves may be loaded via C<-Iperl/lib> in the parent
process; child oracle processes receive a clean env from L</build_oracle_env>.

=back

Legacy success contract for C<report>:

=over 4

=item 1.

Build oracle env from the pin files and sanity-check module path.

=item 2.

Verify C<Devel::NYTProf> resolves under C<baseline/6.15/install>.

=item 3.

Run C<tools/oracle/dump_readstream.pl> on the profile; require exit 0 and
at least one output line (JSONL including C<_END>).

=item 4.

If C<install/bin/nytprofcsv> exists, attempt a CSV write into a temp
directory. Failure due to missing deps is B<non-fatal> when the dump
succeeded; a NOTE is printed.

=back

=head1 FUNCTIONS

=head2 baseline_dir($repo)

Return the absolute path to C<$repo/baseline/6.15>.

=cut

sub baseline_dir {
    my ($repo) = @_;
    $repo = _abs_repo($repo);
    return File::Spec->catdir( $repo, $BASELINE_REL );
}

=head2 build_oracle_env($repo)

Read C<oracle-perl5lib.txt> and C<oracle-module-path.txt>, validate that every
C<PERL5LIB> entry lives under C<baseline/6.15/install>, and return a hashref
suitable for C<%ENV> override:

  {
    PERL5LIB              => '...',
    PATH                  => 'install/bin:...',
    NYTPROF_ORACLE_MODULE => '...',  # from oracle-module-path.txt when present
  }

Dies if the pin files are missing or isolation checks fail.
Does B<not> append C<test-deps>, C<crates/>, or C<perl/lib>.

=cut

sub build_oracle_env {
    my ($repo) = @_;
    $repo = _abs_repo($repo);

    my $baseline = File::Spec->catdir( $repo, $BASELINE_REL );
    my $install  = File::Spec->catdir( $repo, $INSTALL_REL );
    my $p5_file  = File::Spec->catfile( $repo, $PERL5LIB_REL );
    my $mod_file = File::Spec->catfile( $repo, $MODULE_REL );

    -f $p5_file
      or croak "LegacyBridge: missing $p5_file (run scripts/baseline/build_oracle.sh)";
    -d $install
      or croak "LegacyBridge: missing install tree $install";

    open my $fh, '<', $p5_file or croak "LegacyBridge: read $p5_file: $!";
    my $perl5lib = do { local $/; <$fh> };
    close $fh;
    $perl5lib //= '';
    $perl5lib =~ s/\A\s+|\s+\z//g;
    $perl5lib ne ''
      or croak "LegacyBridge: $p5_file is empty";

    my $install_abs = abs_path($install)
      or croak "LegacyBridge: cannot resolve install path $install";

    for my $entry ( split /:/, $perl5lib ) {
        next if !defined $entry || $entry eq '';
        _reject_forbidden_lib_entry( $entry, $repo );
        my $entry_abs = abs_path($entry);
        if ( defined $entry_abs ) {
            _reject_forbidden_lib_entry( $entry_abs, $repo );
            index( $entry_abs, $install_abs ) == 0
              or croak
"LegacyBridge: PERL5LIB entry not under install tree:\n  entry=$entry\n  install=$install_abs";
        }
        else {
            # Unresolved path: still require prefix match on string form.
            index( $entry, $install ) == 0
              or index( $entry, $install_abs ) == 0
              or croak
"LegacyBridge: PERL5LIB entry not under install tree:\n  entry=$entry\n  install=$install_abs";
        }
    }

    my $module_path = '';
    if ( -f $mod_file ) {
        open my $mf, '<', $mod_file or croak "LegacyBridge: read $mod_file: $!";
        $module_path = do { local $/; <$mf> };
        close $mf;
        $module_path //= '';
        $module_path =~ s/\A\s+|\s+\z//g;
        if ( $module_path ne '' ) {
            _reject_forbidden_lib_entry( $module_path, $repo );
            my $mod_abs = abs_path($module_path) // $module_path;
            index( $mod_abs, $install_abs ) == 0
              or croak
"LegacyBridge: oracle-module-path.txt not under install:\n  $module_path";
        }
    }

    my $bin = File::Spec->catdir( $install_abs, 'bin' );
    my $path = $ENV{PATH} // '';
    $path = "$bin:$path" if -d $bin;

    my %env = (
        PERL5LIB => $perl5lib,
        PATH     => $path,
    );
    $env{NYTPROF_ORACLE_MODULE} = $module_path if $module_path ne '';

    return \%env;
}

=head2 verify_oracle_nytprof_load($repo, $env?)

Prove that C<Devel/NYTProf.pm> is found on the oracle C<PERL5LIB> under the
install tree. Uses a path scan (does not C<use Devel::NYTProf>, which would
start the collector).

Optional C<$env> is the hashref from L</build_oracle_env>; built if omitted.

Returns the absolute path to C<Devel/NYTProf.pm>. Dies on isolation failure.

=cut

sub verify_oracle_nytprof_load {
    my ( $repo, $env ) = @_;
    $repo = _abs_repo($repo);
    $env ||= build_oracle_env($repo);

    my $install_abs = abs_path( File::Spec->catdir( $repo, $INSTALL_REL ) )
      or croak "LegacyBridge: cannot resolve install path";

    my $loaded;
    for my $d ( split /:/, ( $env->{PERL5LIB} // '' ) ) {
        next if !defined $d || $d eq '';
        my $p = File::Spec->catfile( $d, 'Devel', 'NYTProf.pm' );
        if ( -f $p ) {
            $loaded = abs_path($p) // $p;
            last;
        }
        # Arch-specific layouts sometimes nest under multi-arch dirs already
        # listed in oracle-perl5lib.txt; also try .../Devel/NYTProf.pm only.
    }

    defined $loaded && length $loaded
      or croak "LegacyBridge: Devel/NYTProf.pm not found on oracle PERL5LIB";

    _reject_forbidden_lib_entry( $loaded, $repo );
    index( $loaded, $install_abs ) == 0
      or croak
"LegacyBridge: Devel::NYTProf not under install tree:\n  loaded=$loaded\n  install=$install_abs";

    if ( my $recorded = $env->{NYTPROF_ORACLE_MODULE} ) {
        my $rec_abs = abs_path($recorded) // $recorded;
        if ( $rec_abs ne $loaded ) {
            # Relocations / multi-arch: non-fatal when still under install.
            print
"NOTE: live NYTProf.pm path differs from oracle-module-path.txt (still under install)\n";
            print "  recorded: $recorded\n";
            print "  live:     $loaded\n";
        }
    }

    return $loaded;
}

=head2 run_legacy_report($repo, $profile)

Execute the legacy report success contract for C<$profile> (typically a
C<nytprof.out>). Prints operator-facing C<OK:> / C<NOTE:> lines to stdout
(and errors to stderr via croak).

Returns C<0> on success. Dies (or can be wrapped) on hard failure.

Optional third argument: hashref of options:

  {
    keep_temp => 0,          # keep temp dirs (debug)
    skip_csv  => 0,          # skip optional nytprofcsv
    dump_out  => undef,      # if set, write dump JSONL to this path
  }

=cut

sub run_legacy_report {
    my ( $repo, $profile, $opts ) = @_;
    $opts ||= {};
    $repo = _abs_repo($repo);

    defined $profile && length $profile
      or croak "LegacyBridge: profile path required";
    -f $profile
      or croak "LegacyBridge: profile not readable: $profile";
    $profile = abs_path($profile) // $profile;

    my $env = build_oracle_env($repo);
    my $loaded = verify_oracle_nytprof_load( $repo, $env );
    print "OK: Devel::NYTProf loads from install: $loaded\n";
    print "OK: oracle PERL5LIB isolated to baseline/6.15/install\n";

    my $dump_pl = File::Spec->catfile( $repo, $DUMP_REL );
    -f $dump_pl
      or croak "LegacyBridge: missing dump helper $dump_pl";

    my $tmp = tempdir(
        'nytprof-legacy-XXXXXX',
        TMPDIR  => 1,
        CLEANUP => $opts->{keep_temp} ? 0 : 1,
    );

    my $dump_path = $opts->{dump_out}
      // File::Spec->catfile( $tmp, 'readstream.jsonl' );

    my ( $dump_rc, $dump_err ) = _run_with_env(
        $env,
        [ $^X, $dump_pl, $profile ],
        $dump_path,    # stdout -> file
    );

    if ( $dump_rc != 0 ) {
        my $err = $dump_err // '';
        $err =~ s/\s+\z//;
        croak "LegacyBridge: dump_readstream.pl failed (exit $dump_rc)"
          . ( length $err ? ":\n$err" : '' );
    }

    open my $df, '<', $dump_path
      or croak "LegacyBridge: cannot read dump output $dump_path: $!";
    my $line_count = 0;
    my $has_tag    = 0;
    my $has_end    = 0;
    while ( my $line = <$df> ) {
        $line_count++;
        $has_tag = 1 if index( $line, '"tag"' ) >= 0;
        $has_end = 1 if index( $line, '"_END"' ) >= 0 || index( $line, "'_END'" ) >= 0;
    }
    close $df;

    $line_count > 0
      or croak "LegacyBridge: dump_readstream.pl produced no lines";
    ( $has_tag && $has_end )
      or croak
"LegacyBridge: dump output missing expected tags (\"tag\" / _END); lines=$line_count";

    print
"OK: legacy report smoke = stream dump ($line_count JSONL lines from dump_readstream.pl)\n";

    unless ( $opts->{skip_csv} ) {
        my $csv_bin = File::Spec->catfile( $repo, $CSV_BIN_REL );
        if ( -x $csv_bin || ( -f $csv_bin && -r $csv_bin ) ) {
            my $csv_out = File::Spec->catdir( $tmp, 'nytprofcsv-out' );
            make_path($csv_out);
            my ( $csv_rc, $csv_err ) = _run_with_env(
                $env,
                [ $^X, $csv_bin, '-f', $profile, '-o', $csv_out ],
                File::Spec->devnull(),
            );
            if ( $csv_rc == 0 ) {
                print "OK: oracle nytprofcsv wrote reports under $csv_out\n";
            }
            else {
                my $err = $csv_err // '';
                $err =~ s/\s+\z//;
                print
"NOTE: nytprofcsv failed (ignored; dump succeeded) exit=$csv_rc"
                  . ( length $err ? " -- $err" : '' ) . "\n";
            }
        }
        else {
            print "NOTE: install/bin/nytprofcsv not present; dump-only legacy report\n";
        }
    }

    return 0;
}

# ---------------------------------------------------------------------------
# Internals
# ---------------------------------------------------------------------------

sub _abs_repo {
    my ($repo) = @_;
    defined $repo && length $repo
      or croak "LegacyBridge: repo root required";
    my $abs = abs_path($repo);
    defined $abs
      or croak "LegacyBridge: cannot resolve repo path: $repo";
    -d $abs
      or croak "LegacyBridge: repo root is not a directory: $abs";
    return $abs;
}

sub _reject_forbidden_lib_entry {
    my ( $path, $repo ) = @_;
    return unless defined $path && length $path;

    if ( $path =~ m{/crates(/|\z)} || $path =~ m{/crates$} ) {
        croak "LegacyBridge: forbidden crates/ path on oracle load: $path";
    }

    # Candidate facade must never appear on oracle PERL5LIB / module path.
    my $perl_lib = File::Spec->catdir( $repo, 'perl', 'lib' );
    my $perl_lib_abs = abs_path($perl_lib);
    if ( defined $perl_lib_abs && index( $path, $perl_lib_abs ) == 0 ) {
        croak "LegacyBridge: forbidden candidate perl/ path on oracle load: $path";
    }
    if ( $path =~ m{/perl/lib(/|\z)} ) {
        # String form (unresolved) -- still reject obvious candidate paths.
        my $repo_perl = File::Spec->catdir( $repo, 'perl' );
        if ( index( $path, $repo_perl ) == 0 || $path =~ m{\A.*/perl/lib} ) {
            # Only reject if it looks like this repo's facade, not random /perl/lib.
            if ( index( $path, File::Spec->catdir( $repo, 'perl' ) ) == 0 ) {
                croak "LegacyBridge: forbidden candidate perl/ path on oracle load: $path";
            }
        }
    }
    return;
}

# Run command with isolated env in a forked child (avoids open3 pipe deadlocks
# when redirecting large stdout to a file). Returns (exit_code, stderr_text).
sub _run_with_env {
    my ( $env, $cmd, $stdout_path ) = @_;
    $cmd = [@$cmd];

    my ( $err_fh, $err_path ) = tempfile(
        'nytprof-legacy-err-XXXXXX',
        TMPDIR  => 1,
        UNLINK  => 1,
    );
    close $err_fh;    # reopen in child / parent reader

    my $pid = fork();
    if ( !defined $pid ) {
        return ( 1, "LegacyBridge: fork failed: $!" );
    }

    if ( $pid == 0 ) {
        # Child: apply oracle env only; never inherit parent PERL5LIB overrides
        # beyond what $env supplies.
        for my $k ( keys %$env ) {
            $ENV{$k} = $env->{$k};
        }
        delete $ENV{PERL5OPT};
        $ENV{NYTPROF} = 'start=no';

        open STDIN,  '<', File::Spec->devnull() or exit 127;
        open STDOUT, '>', $stdout_path         or exit 127;
        open STDERR, '>', $err_path            or exit 127;

        exec { $cmd->[0] } @$cmd;
        exit 127;
    }

    waitpid $pid, 0;
    my $status = $?;
    my $rc;
    if ( $status == -1 ) {
        $rc = 127;
    }
    elsif ( $status & 127 ) {
        $rc = 128 + ( $status & 127 );
    }
    else {
        $rc = $status >> 8;
    }

    my $err_text = '';
    if ( open my $ef, '<', $err_path ) {
        local $/;
        $err_text = <$ef> // '';
        close $ef;
    }

    return ( $rc, $err_text );
}

1;

__END__

=head1 ENVIRONMENT

=over 4

=item PERL5LIB (child)

Set exclusively from C<baseline/6.15/oracle-perl5lib.txt> for oracle
subprocesses. Parent C<-Iperl/lib> is not inherited as PERL5LIB.

=item PATH (child)

Prepends C<baseline/6.15/install/bin> so C<nytprofcsv> resolves when present.

=item NYTPROF

Child processes get C<start=no> to avoid accidental collector startup.

=back

=head1 SEE ALSO

L<Devel::NYTProf::EngineDispatch>,
C<docs/schemas/perl-engine-dispatch-mvp-v0.md>,
C<tools/oracle/env.sh>,
C<scripts/packaging/legacy_only_smoke.sh>,
C<scripts/packaging/perl_engine_dispatch_smoke.sh>

=head1 AUTHOR

NYTProf modernisation packaging wave

=cut
