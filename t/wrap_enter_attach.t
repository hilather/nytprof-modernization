#!/usr/bin/env perl
# Drive shipped g16_wrap_enter_smoke.sh — real perl -d:NYTProfM.
# Pre-fix: wrap used caller(0)+fid_for_filename on every instrumented
# call. Post-fix: default wrap uses wrap_push/wrap_pop; WRAP_SLOW is
# the slower control.
use strict;
use warnings;
use File::Basename qw(dirname);
use File::Spec;
use Cwd qw(abs_path);
use Test::More;

my $root  = abs_path( File::Spec->catdir( dirname($0), '..' ) );
my $smoke = File::Spec->catfile( $root,
    qw(scripts packaging g16_wrap_enter_smoke.sh) );
ok( -f $smoke, "g16 smoke exists" );
ok( -x $smoke, "g16 smoke is executable" );

my $pm = File::Spec->catfile( $root, qw(collector xs Devel NYTProfM.pm) );
ok( -f $pm, "NYTProfM.pm exists" );
open my $fh, '<', $pm or die "open $pm: $!";
my $src = do { local $/; <$fh> };
close $fh;
like( $src, qr/wrap_push/, 'NYTProfM.pm calls wrap_push' );
like( $src, qr/PRODUCT_WRAP_SLOW/,
    'NYTProfM.pm has WRAP_SLOW wrap-on control' );

my $xs = File::Spec->catfile( $root, qw(collector xs NYTProf.xs) );
ok( -f $xs, "NYTProf.xs exists" );
open my $xsf, '<', $xs or die "open $xs: $!";
my $xsrc = do { local $/; <$xsf> };
close $xsf;
like( $xsrc, qr/wrap_push/, 'NYTProf.xs has wrap_push' );
like( $xsrc, qr/wrap_pop/,  'NYTProf.xs has wrap_pop' );
like( $xsrc, qr/product_wrap_pin_cop/,
    'NYTProf.xs pins wrap site past package-DB frames' );

my $out = qx{bash '$smoke' 2>&1};
my $rc  = $? >> 8;
ok( $rc == 0, "g16 smoke exits 0 (got $rc)" )
  or diag $out;
like(
    $out,
    qr/G16 wrap_push|SKIP: no C toolchain|SKIP: perl XS headers/,
    'g16 printed G16 success or an honest skip'
);
unlike( $out, qr/default wrap still does caller/,
    'g16 did not fail the default-wrap caller+fid check' );

done_testing();
