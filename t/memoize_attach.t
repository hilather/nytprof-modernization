#!/usr/bin/env perl
# Drive shipped g12_memoize_caller_smoke.sh — real perl -d:NYTProfM
# + Memoize::memoize('expensive'). Pre-fix: Cannot operate on nonexistent
# function `expensive' (caller is DB).
use strict;
use warnings;
use File::Basename qw(dirname);
use File::Spec;
use Cwd qw(abs_path);
use Test::More;

my $root  = abs_path( File::Spec->catdir( dirname($0), '..' ) );
my $smoke = File::Spec->catfile( $root,
    qw(scripts packaging g12_memoize_caller_smoke.sh) );
ok( -f $smoke, "g12 smoke exists: $smoke" );
ok( -x $smoke, "g12 smoke is executable" );

my $pm = File::Spec->catfile( $root, qw(collector xs Devel NYTProfM.pm) );
ok( -f $pm, "NYTProfM.pm exists" );
open my $fh, '<', $pm or die "open $pm: $!";
my $src = do { local $/; <$fh> };
close $fh;
like( $src, qr/Memoize\(\?:::\|\\z\)/,
    'NYTProfM.pm has Memoize(?:::|\\z) on _product_needs_goto' );

my $out = qx{bash '$smoke' 2>&1};
my $rc  = $? >> 8;
ok( $rc == 0, "g12 smoke exits 0 (got $rc)" )
  or diag $out;
like(
    $out,
    qr/ok-memoize|SKIP: no C toolchain|SKIP: perl XS headers|SKIP: Memoize/,
    'g12 smoke printed ok-memoize or an honest skip'
);
unlike( $out, qr/Cannot operate on nonexistent function/,
    'g12 smoke did not hit Memoize caller=DB croak' );

done_testing();
