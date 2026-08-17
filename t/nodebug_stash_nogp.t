#!/usr/bin/env perl
# Drive shipped g11_nodebug_stash_nogp_smoke.sh — real DB::nodebug_stash
# + live perl -d:NYTProfM on a planted GP-less stash GV (pre-fix SEGV).
use strict;
use warnings;
use File::Basename qw(dirname);
use File::Spec;
use Cwd qw(abs_path);
use Test::More;

my $root  = abs_path( File::Spec->catdir( dirname($0), '..' ) );
my $smoke = File::Spec->catfile( $root,
    qw(scripts packaging g11_nodebug_stash_nogp_smoke.sh) );
ok( -f $smoke, "g11 smoke exists: $smoke" );
ok( -x $smoke, "g11 smoke is executable" );

my $xs = File::Spec->catfile( $root, qw(collector xs NYTProf.xs) );
ok( -f $xs, "NYTProf.xs exists" );
open my $fh, '<', $xs or die "open $xs: $!";
my $src = do { local $/; <$fh> };
close $fh;
like( $src, qr/product_stash_val_cv/,
    'NYTProf.xs has product_stash_val_cv (GP-safe stash walk)' );
like( $src, qr/isGV_with_GP/,
    'NYTProf.xs uses isGV_with_GP before GvCV' );

my $out = qx{bash '$smoke' 2>&1};
my $rc  = $? >> 8;
ok( $rc == 0, "g11 smoke exits 0 (got $rc)" )
  or diag $out;
like( $out, qr/ok-nogp|SKIP: no C toolchain|SKIP: perl XS headers/,
    'g11 smoke printed ok-nogp or an honest skip' );
unlike( $out, qr/Segmentation fault|core dumped/,
    'g11 smoke did not core-dump' );

done_testing();
