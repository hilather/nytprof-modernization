#!/usr/bin/env perl
# Drive the shipped catalog.tsv — 20 apps, 10 diverse top-10 families.
use strict;
use warnings;
use File::Basename qw(dirname);
use File::Spec;
use Cwd qw(abs_path);
use Test::More;

my $root = abs_path( File::Spec->catdir( dirname($0), '..' ) );
my $tsv  = File::Spec->catfile( $root,
    qw(scripts field workloads complex_apps catalog.tsv) );
ok( -f $tsv, "catalog.tsv exists: $tsv" );

my @rows;
my %fam;
open my $fh, '<', $tsv or die "open $tsv: $!";
while ( my $line = <$fh> ) {
    next if $line =~ /^\s*#/ || $line !~ /\S/;
    chomp $line;
    my @f = split /\t/, $line, -1;
    is( scalar @f, 8, "row $f[0] has 8 TSV fields" );
    push @rows, \@f;
    if ( $f[1] eq 'top10' ) {
        ok( $f[2], "top10 $f[0] names a primary_family" );
        ok( !exists $fam{ $f[2] },
            "top10 family '$f[2]' is unique (also $f[0])" );
        $fam{ $f[2] } = $f[0];
        like( $f[6], qr/\.pl\z/, "top10 $f[0] driver path is a .pl ($f[6])" );
        my $drv = File::Spec->catfile( $root, $f[6] );
        ok( -f $drv, "top10 $f[0] driver exists: $f[6]" );
        open my $df, '<', $drv or die $!;
        local $/;
        my $src = <$df>;
        close $df;
        like( $src, qr/\Q$f[3]\E/, "driver $f[0] prints token $f[3]" );
    }
}
close $fh;

is( scalar @rows, 20, 'exactly 20 catalog apps' );
is( scalar keys %fam, 10, 'exactly 10 top-10 primary families' );

done_testing();
