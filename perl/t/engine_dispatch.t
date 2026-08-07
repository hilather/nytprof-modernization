#!/usr/bin/env perl
# Unit tests for Devel::NYTProf::EngineDispatch pure helpers.
use strict;
use warnings;
use Test::More;
use FindBin;
use File::Spec;
use lib File::Spec->catdir( $FindBin::Bin, '..', 'lib' );

use Devel::NYTProf::EngineDispatch qw(
  ALLOWED_ENGINES
  resolve_engine
  select_runtime_engine
  peel_engine_flag
  find_repo_root
  find_native_cli
);

# ---------------------------------------------------------------------------
# resolve_engine — returns requested name (auto is NOT collapsed)
# ---------------------------------------------------------------------------

is( resolve_engine( undef, undef ), 'native', 'default is native' );
is( resolve_engine( '',    undef ), 'native', 'empty cli → native' );
is( resolve_engine( undef, '' ),    'native', 'empty env → native' );

is( resolve_engine( 'native', undef ),   'native', 'cli native' );
is( resolve_engine( 'legacy', undef ),   'legacy', 'cli legacy' );
is( resolve_engine( 'auto',   undef ),   'auto',   'cli auto stays auto' );
is( resolve_engine( 'AUTO',   undef ),   'auto',   'cli AUTO → auto' );
is( resolve_engine( 'Native', undef ),   'native', 'case insensitive native' );
is( resolve_engine( 'LEGACY', undef ),   'legacy', 'case insensitive legacy' );

is( resolve_engine( 'native', 'legacy' ), 'native', 'cli overrides env' );
is( resolve_engine( 'legacy', 'native' ), 'legacy', 'cli legacy overrides env' );
is( resolve_engine( undef,    'legacy' ), 'legacy', 'env used when cli omitted' );
is( resolve_engine( undef,    'auto' ),   'auto',   'env auto stays auto' );

{
    my $err;
    eval { resolve_engine( 'bogus', undef ); 1 } or $err = $@;
    ok( defined $err, 'bogus engine dies' );
    like( $err, qr/invalid engine/i, 'bogus message mentions invalid engine' );
    like( $err, qr/native/,          'bogus message lists native' );
    like( $err, qr/legacy/,          'bogus message lists legacy' );
    like( $err, qr/auto/,            'bogus message lists auto' );
}

is( ALLOWED_ENGINES, 'native, legacy, auto', 'ALLOWED_ENGINES constant' );

# ---------------------------------------------------------------------------
# select_runtime_engine + find_native_cli test hook
# ---------------------------------------------------------------------------

my $repo = eval { find_repo_root( File::Spec->catdir( $FindBin::Bin, '..', '..' ) ) };
ok( defined $repo && length $repo, 'find_repo_root from perl/t' )
  or diag("find_repo_root error: $@");

SKIP: {
    skip 'no repo root', 7 unless defined $repo && length $repo;

    is( select_runtime_engine( $repo, 'legacy' ),
        'legacy', 'select_runtime_engine legacy → legacy' );
    is( select_runtime_engine( $repo, 'native' ),
        'native', 'select_runtime_engine native → native' );

    # With force-no-native, auto must fall back to legacy.
    {
        local $ENV{NYTPROF_FORCE_NO_NATIVE} = '1';
        # Clear override path so only the force hook matters.
        local $ENV{NYTPROF_NATIVE_CLI};
        delete $ENV{NYTPROF_NATIVE_CLI};

        my $err;
        eval { find_native_cli($repo); 1 } or $err = $@;
        ok( defined $err, 'NYTPROF_FORCE_NO_NATIVE makes find_native_cli croak' );
        like( $err, qr/NYTPROF_FORCE_NO_NATIVE|native CLI not found/i,
            'force-no-native error message' );

        # Capture STDERR note from select_runtime_engine.
        my $stderr = '';
        my $runtime_fb;
        {
            open local *STDERR, '>', \$stderr or die $!;
            $runtime_fb = select_runtime_engine( $repo, 'auto' );
        }
        is( $runtime_fb, 'legacy', 'auto + force-no-native → legacy' );
        like(
            $stderr,
            qr/auto:.*native CLI not found|using legacy/i,
            'auto fallback prints STDERR note'
        );
    }

    # Without force hook: auto prefers native when discoverable (prefix/target/cargo).
    {
        local $ENV{NYTPROF_FORCE_NO_NATIVE};
        delete $ENV{NYTPROF_FORCE_NO_NATIVE};
        my $runtime = select_runtime_engine( $repo, 'auto' );
        ok(
            $runtime eq 'native' || $runtime eq 'legacy',
            "auto without force → $runtime (native if discoverable)"
        );
    }
}

# ---------------------------------------------------------------------------
# peel_engine_flag
# ---------------------------------------------------------------------------

{
    my ( $e, @r ) = peel_engine_flag( '--engine=native', 'report', 'foo.out' );
    is( $e, 'native', 'peel equals-form value' );
    is_deeply( \@r, [ 'report', 'foo.out' ], 'peel equals-form rest' );
}

{
    my ( $e, @r ) =
      peel_engine_flag( '--engine', 'legacy', 'verify', 'foo.out' );
    is( $e, 'legacy', 'peel space-form value' );
    is_deeply( \@r, [ 'verify', 'foo.out' ], 'peel space-form rest' );
}

{
    my ( $e, @r ) = peel_engine_flag( 'report', 'foo.out' );
    ok( !defined $e, 'peel without engine flag' );
    is_deeply( \@r, [ 'report', 'foo.out' ], 'peel rest unchanged' );
}

{
    my $err;
    eval { peel_engine_flag('--engine'); 1 } or $err = $@;
    ok( defined $err, 'peel missing value dies' );
    like( $err, qr/--engine requires a value/, 'peel missing value message' );
}

{
    my $err;
    eval { peel_engine_flag( '--engine=', 'report' ); 1 } or $err = $@;
    ok( defined $err, 'peel empty equals value dies' );
}

done_testing();
