#!/usr/bin/env perl
# Drive shipped scripts/field/lib/attach_survival.sh on real files.
use strict;
use warnings;
use File::Basename qw(dirname);
use File::Spec;
use File::Temp qw(tempdir);
use Cwd qw(abs_path);
use Test::More;

my $root = abs_path( File::Spec->catdir( dirname($0), '..' ) );
my $lib  = File::Spec->catfile( $root, qw(scripts field lib attach_survival.sh) );
ok( -f $lib, "attach_survival.sh exists" );

my $tmp = tempdir( CLEANUP => 1 );
my $clean = File::Spec->catfile( $tmp, 'clean.txt' );
my $kill  = File::Spec->catfile( $tmp, 'kill.txt' );
my $prof  = File::Spec->catfile( $tmp, 'nytprof.out' );
{
    open my $o, '>', $clean or die $!;
    print {$o} "mojo_lab_ok passes=3\n";
    close $o;
    open my $k, '>', $kill or die $!;
    print {$k} "Can't use string (\"#pod\\n\") as an ARRAY ref at B/Hooks/EndOfScope/XS.pm line 39\n";
    close $k;
    open my $p, '>', $prof or die $!;
    print {$p} "NYTProf 5 0\n";
    close $p;
}

my $bash = qq{
  set -euo pipefail
  source '$lib'
  attach_fail_if_killed '$clean'
  attach_require_token '$clean' 'mojo_lab_ok'
  attach_require_nytprof5 '$prof'
};
is( system( 'bash', '-c', $bash ), 0, 'clean log + token + NYTProf 5 pass shipped helpers' );

my $bash_kill = qq{
  set -euo pipefail
  source '$lib'
  attach_fail_if_killed '$kill'
};
isnt( system( 'bash', '-c', $bash_kill ), 0,
    'ARRAY ref + EndOfScope/XS.pm fail-closes via shipped attach_fail_if_killed' );

my $dbso = File::Spec->catfile( $tmp, 'dbso.txt' );
{
    open my $d, '>', $dbso or die $!;
    print {$d} "Can't locate loadable object for module DB in \@INC\n";
    close $d;
}
my $bash_db = qq{
  set -euo pipefail
  source '$lib'
  attach_fail_if_killed '$dbso'
};
isnt( system( 'bash', '-c', $bash_db ), 0,
    'loadable object for module DB fail-closes via shipped helper' );

done_testing();
