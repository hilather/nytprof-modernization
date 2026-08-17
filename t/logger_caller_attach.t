#!/usr/bin/env perl
# Drive shipped g13_logger_caller_smoke.sh — real perl -d:NYTProfM.
# Pre-fix: logger caller is NYTProfM.pm (eval { &$raw } at line 308).
use strict;
use warnings;
use File::Basename qw(dirname);
use File::Spec;
use Cwd qw(abs_path);
use Test::More;

my $root  = abs_path( File::Spec->catdir( dirname($0), '..' ) );
my $smoke = File::Spec->catfile( $root,
    qw(scripts packaging g13_logger_caller_smoke.sh) );
ok( -f $smoke, "g13 smoke exists" );
ok( -x $smoke, "g13 smoke is executable" );

my $pm = File::Spec->catfile( $root, qw(collector xs Devel NYTProfM.pm) );
ok( -f $pm, "NYTProfM.pm exists" );
open my $fh, '<', $pm or die "open $pm: $!";
my $src = do { local $/; <$fh> };
close $fh;
unlike( $src, qr/eval\s*\{[^}]*\&\$raw/,
    'NYTProfM.pm does not eval-wrap &$raw' );
like( $src, qr/ProductWrapGuard/,
    'NYTProfM.pm has DESTROY guard for die-path finish' );

my $out = qx{bash '$smoke' 2>&1};
my $rc  = $? >> 8;
ok( $rc == 0, "g13 smoke exits 0 (got $rc)" )
  or diag $out;
like( $out, qr/ok-logger|SKIP: no C toolchain|SKIP: perl XS headers/,
    'g13 printed ok-logger or an honest skip' );
unlike( $out, qr/logger caller is NYTProfM/,
    'g13 did not report NYTProfM.pm as logger caller' );

done_testing();
