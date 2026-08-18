#!/usr/bin/env perl
# Drive shipped g19_slowops_full_smoke.sh plus g08/g09 on default slowops=2.
# Real perl -d:NYTProfM. Default =2 must install the 6.15 full table
# (CORE:stat/sleep/prtf), not PRINT/MATCH only.
use strict;
use warnings;
use File::Basename qw(dirname);
use File::Spec;
use Cwd qw(abs_path);
use Test::More;

my $root = abs_path( File::Spec->catdir( dirname($0), '..' ) );
my $g19  = File::Spec->catfile( $root,
    qw(scripts packaging g19_slowops_full_smoke.sh) );
my $g08  = File::Spec->catfile( $root,
    qw(scripts packaging g08_slowops_times_smoke.sh) );
my $g09  = File::Spec->catfile( $root,
    qw(scripts packaging g09_tokenize_excl_smoke.sh) );

ok( -f $g19, "g19 smoke exists" );
ok( -x $g19, "g19 smoke is executable" );
ok( -f $g08, "g08 smoke exists" );
ok( -f $g09, "g09 smoke exists" );

my $pm = File::Spec->catfile( $root, qw(collector xs Devel NYTProfM.pm) );
ok( -f $pm, "NYTProfM.pm exists" );
open my $fh, '<', $pm or die "open $pm: $!";
my $src = do { local $/; <$fh> };
close $fh;
like( $src, qr/eq 'full'/, 'NYTProfM.pm accepts slowops=full' );
like( $src, qr/install_product_slowops_full/,
    'NYTProfM.pm installs full table' );
like( $src, qr/PRODUCT_SLOWOPS >= 2/,
    'NYTProfM.pm default/2/3 share the full-table installer' );

my $h = File::Spec->catfile( $root, qw(collector xs slowops.h) );
ok( -f $h, "collector/xs/slowops.h exists" );
open my $hf, '<', $h or die "open $h: $!";
my $hs = do { local $/; <$hf> };
close $hf;
like( $hs, qr/devel-nytprof-6.15\/slowops\.h/,
    'slowops.h records pin-archive provenance' );
like( $hs, qr/pp_slowop_profiler/, 'slowops.h assigns pp_slowop_profiler' );

my $out = qx{bash '$g19' 2>&1};
my $rc  = $? >> 8;
ok( $rc == 0, "g19 smoke exits 0 (got $rc)" )
  or diag $out;
like(
    $out,
    qr/G19 default slowops=2 is the full 6\.15 table|SKIP: no C toolchain|SKIP: perl XS headers/,
    'g19 printed G19 success or an honest skip'
);
unlike( $out, qr/PRINT\/MATCH only \(no stat/,
    'g19 no longer treats default as PRINT/MATCH-only' );
like(
    $out,
    qr/full 6\.15 table|SKIP: no C toolchain|SKIP: perl XS headers/,
    'g19 reports the 6.15 full table on default or honest skip'
);

# g08 + g09 must stay green on default slowops=2 (no ATTACH_OPTS).
for my $pair ( [ $g08, qr/G08 slowops PRINT\/MATCH|SKIP: no C toolchain|SKIP: perl XS headers/ ],
    [ $g09, qr/G09 tokenize exclusive split|SKIP: no C toolchain|SKIP: perl XS headers/ ] )
{
    my ( $script, $re ) = @$pair;
    my $sout = qx{bash '$script' 2>&1};
    my $src_ = $? >> 8;
    ok( $src_ == 0, "$script exits 0 (got $src_)" )
      or diag $sout;
    like( $sout, $re, "$script printed success or honest skip" );
}

done_testing();
