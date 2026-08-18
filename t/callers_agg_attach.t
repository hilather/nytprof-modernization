#!/usr/bin/env perl
# Drive g20_callers_agg_smoke.sh — real perl -d:NYTProfM.
# SUB_CALLERS aggregated at finish; SUB_RETURN stays 1:1.
use strict;
use warnings;
use File::Basename qw(dirname);
use File::Spec;
use Cwd qw(abs_path);
use Test::More;

my $root  = abs_path( File::Spec->catdir( dirname($0), '..' ) );
my $smoke = File::Spec->catfile( $root,
    qw(scripts packaging g20_callers_agg_smoke.sh) );
ok( -f $smoke, "g20 smoke exists" );
ok( -x $smoke, "g20 smoke is executable" );

my $c = File::Spec->catfile( $root, qw(collector xs product_callers.c) );
ok( -f $c, "product_callers.c exists" );
open my $cf, '<', $c or die "open $c: $!";
my $csrc = do { local $/; <$cf> };
close $cf;
like( $csrc, qr/product_callers_flush/, 'C flush exists' );
unlike( $csrc, qr/sub_callers_hv/,      'no Perl callers HV' );

my $pp = File::Spec->catfile( $root, qw(collector xs pp_entersub.c) );
open my $pf, '<', $pp or die "open $pp: $!";
my $ppsrc = do { local $/; <$pf> };
close $pf;
like( $ppsrc, qr/product_callers_add/, 'opcode return adds to C table' );
unlike( $ppsrc, qr/nytp_emit_sub_callers/,
    'opcode return does not emit SUB_CALLERS' );

my @cmd = ( 'bash', $smoke );
if ( $ENV{NYTPROF_NATIVE_CLI} ) {
    $ENV{NYTPROF_NATIVE_CLI} = $ENV{NYTPROF_NATIVE_CLI};
}
my $rc = system(@cmd);
is( $rc, 0, "g20_callers_agg_smoke.sh exit 0" );

done_testing();
