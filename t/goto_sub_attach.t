#!/usr/bin/env perl
# Drive shipped g18_goto_sub_smoke.sh — real perl -d:NYTProfM + goto &other.
# Default opcode hooks OP_GOTO; wrap list stays wrap=1 only.
use strict;
use warnings;
use File::Basename qw(dirname);
use File::Spec;
use Cwd qw(abs_path);
use Test::More;

my $root  = abs_path( File::Spec->catdir( dirname($0), '..' ) );
my $smoke = File::Spec->catfile( $root,
    qw(scripts packaging g18_goto_sub_smoke.sh) );
ok( -f $smoke, "g18 smoke exists" );
ok( -x $smoke, "g18 smoke is executable" );

my $pp = File::Spec->catfile( $root, qw(collector xs pp_entersub.c) );
ok( -f $pp, "pp_entersub.c exists" );
open my $ppf, '<', $pp or die "open $pp: $!";
my $ppsrc = do { local $/; <$ppf> };
close $ppf;
like( $ppsrc, qr/product_orig_pp_goto/,
    'pp_entersub.c has separate product_orig_pp_goto' );
unlike( $ppsrc, qr/E2 owns OP_GOTO\. Never take a non-ENTERSUB/,
    'pp_entersub.c no longer early-returns OP_GOTO' );

my $pm = File::Spec->catfile( $root, qw(collector xs Devel NYTProfM.pm) );
ok( -f $pm, "NYTProfM.pm exists" );
open my $fh, '<', $pm or die "open $pm: $!";
my $src = do { local $/; <$fh> };
close $fh;
like( $src, qr/PRODUCT_GOTO_OPS/, 'NYTProfM.pm stamps PRODUCT_GOTO_OPS' );
like( $src, qr/wrap=1 \/ use_db_sub=1 only/,
    'NYTProfM.pm wrap list remains wrap=1 only' );

my $out = qx{bash '$smoke' 2>&1};
my $rc  = $? >> 8;
ok( $rc == 0, "g18 smoke exits 0 (got $rc)" )
  or diag $out;
like(
    $out,
    qr/G18 OP_GOTO attach|SKIP: no C toolchain|SKIP: perl XS headers/,
    'g18 printed G18 success or an honest skip'
);
unlike( $out, qr/caller of other must be original_caller/,
    'g18 did not fail original-caller attribution' );
unlike( $out, qr/SUB_CALLERS line must be goto site/,
    'g18 did not fail goto-site fid:line' );

done_testing();
