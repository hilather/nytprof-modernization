#!/usr/bin/env perl
# Drive shipped g19_leave_discount_smoke.sh — real perl -d:NYTProfM.
# Pre-fix: leave= was parsed as a known key but ignored (no PRODUCT_LEAVE,
# no DISCOUNT). Post-fix: leave=1 installs pp_leave + nytp_emit_discount;
# default leave stays 0.
use strict;
use warnings;
use File::Basename qw(dirname);
use File::Spec;
use Cwd qw(abs_path);
use Test::More;

my $root  = abs_path( File::Spec->catdir( dirname($0), '..' ) );
my $smoke = File::Spec->catfile( $root,
    qw(scripts packaging g19_leave_discount_smoke.sh) );
ok( -f $smoke, "g19 smoke exists" );
ok( -x $smoke, "g19 smoke is executable" );

my $pm = File::Spec->catfile( $root, qw(collector xs Devel NYTProfM.pm) );
ok( -f $pm, "NYTProfM.pm exists" );
open my $fh, '<', $pm or die "open $pm: $!";
my $src = do { local $/; <$fh> };
close $fh;
like( $src, qr/PRODUCT_LEAVE/, 'NYTProfM.pm stamps PRODUCT_LEAVE' );
like( $src, qr/_product_int_opt\(\s*\$opts,\s*'leave',\s*0\s*\)/,
    'NYTProfM.pm applies leave default 0' );
like( $src, qr/install_product_leave/,
    'NYTProfM.pm calls install_product_leave' );

my $pp = File::Spec->catfile( $root, qw(collector xs pp_leave.c) );
ok( -f $pp, "pp_leave.c exists" );
open my $ppf, '<', $pp or die "open $pp: $!";
my $ppsrc = do { local $/; <$ppf> };
close $ppf;
like( $ppsrc, qr/nytp_emit_discount/, 'pp_leave.c emits via nytp_emit_discount' );
unlike( $ppsrc, qr/NYTP_write_/, 'pp_leave.c does not use FileHandle writes' );
unlike( $ppsrc, qr/nytp_emit_time_(?:line|block)/,
    'pp_leave.c does not double-write TIME_*' );

my $out = qx{bash '$smoke' 2>&1};
my $rc  = $? >> 8;
ok( $rc == 0, "g19 smoke exits 0 (got $rc)" )
  or diag $out;
like(
    $out,
    qr/G19 leave=1 DISCOUNT|SKIP: no C toolchain|SKIP: perl XS headers/,
    'g19 printed G19 success or an honest skip'
);
unlike( $out, qr/leave=1 must emit DISCOUNT/,
    'g19 did not fail the leave=1 DISCOUNT assert' );

done_testing();
