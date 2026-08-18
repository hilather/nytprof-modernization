#!/usr/bin/env perl
# Drive shipped g15_dbstate_timeline_smoke.sh — real perl -d:NYTProfM.
# Pre-fix: default attach sets $DB::single=1 and enters Perl DB::DB
# on every statement (caller + fid XSUB). Post-fix: C OP_DBSTATE
# emits TIME_LINE and $DB::single stays 0.
use strict;
use warnings;
use File::Basename qw(dirname);
use File::Spec;
use Cwd qw(abs_path);
use Test::More;

my $root  = abs_path( File::Spec->catdir( dirname($0), '..' ) );
my $smoke = File::Spec->catfile( $root,
    qw(scripts packaging g15_dbstate_timeline_smoke.sh) );
ok( -f $smoke, "g15 smoke exists" );
ok( -x $smoke, "g15 smoke is executable" );

my $pm = File::Spec->catfile( $root, qw(collector xs Devel NYTProfM.pm) );
ok( -f $pm, "NYTProfM.pm exists" );
open my $fh, '<', $pm or die "open $pm: $!";
my $src = do { local $/; <$fh> };
close $fh;
like( $src, qr/PRODUCT_DBSTATE_LINE/,
    'NYTProfM.pm stamps PRODUCT_DBSTATE_LINE' );
like( $src, qr/install_product_dbstate_timeline/,
    'NYTProfM.pm calls install_product_dbstate_timeline' );

my $xs = File::Spec->catfile( $root, qw(collector xs NYTProf.xs) );
ok( -f $xs, "NYTProf.xs exists" );
open my $xsf, '<', $xs or die "open $xs: $!";
my $xsrc = do { local $/; <$xsf> };
close $xsf;
like( $xsrc, qr/pp_product_dbstate_line/,
    'NYTProf.xs has pp_product_dbstate_line' );
like( $xsrc, qr/product_seed_last_site/,
    'NYTProf.xs restarts last-site clock after TIME_LINE emit' );

my $out = qx{bash '$smoke' 2>&1};
my $rc  = $? >> 8;
ok( $rc == 0, "g15 smoke exits 0 (got $rc)" )
  or diag $out;
like(
    $out,
    qr/ok-G15|G15 C OP_DBSTATE|SKIP: no C toolchain|SKIP: perl XS headers/,
    'g15 printed G15 success or an honest skip'
);
unlike( $out, qr/default stmts=1 must set PRODUCT_DBSTATE_LINE=1/,
    'g15 did not fail the C TIME_LINE stamp' );

done_testing();
