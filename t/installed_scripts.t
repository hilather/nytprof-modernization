#!/usr/bin/env perl
# Prove installed I03 scripts (nytprofhtml → nytprof-engine) from the RPM tree.
# Cargo-free. Optional NYTPROF_JSONL runs query --json --jsonl 15/3/15.
use strict;
use warnings;
use File::Spec;
use Cwd qw(abs_path);

my $bindir = $ENV{NYTPROF_BINDDIR};
if ( !$bindir || !-d $bindir ) {
    for my $inc (@INC) {
        next unless defined $inc && length $inc;
        my $cand = File::Spec->catdir( $inc, File::Spec->updir(),
            File::Spec->updir(), 'bin' );
        $cand = eval { abs_path($cand) } || $cand;
        if ( -x File::Spec->catfile( $cand, 'nytprof-engine' ) ) {
            $bindir = $cand;
            last;
        }
    }
}
$bindir or die "NYTPROF_BINDDIR not set and no nytprof-engine next to PERL5LIB\n";

my $engine = File::Spec->catfile( $bindir, 'nytprof-engine' );
my $html   = File::Spec->catfile( $bindir, 'nytprofhtml' );
my $csv    = File::Spec->catfile( $bindir, 'nytprofcsv' );
my $cg     = File::Spec->catfile( $bindir, 'nytprofcg' );
-x $engine or die "missing executable $engine\n";
-x $html   or die "missing executable $html\n";
-x $csv    or die "missing executable $csv\n";
-x $cg     or die "missing executable $cg\n";

for my $wrap ( $html, $csv, $cg ) {
    open my $fh, '<', $wrap or die "open $wrap: $!\n";
    my $src = do { local $/; <$fh> };
    $src =~ /nytprof-engine/ or die "$wrap does not exec nytprof-engine\n";
    $src =~ /baseline\/6\.15/ and die "$wrap must not be oracle nytprofhtml\n";
}

my $cli = File::Spec->catfile( $bindir, 'nytprof-cli' );
-x $cli or die "missing sibling nytprof-cli at $cli (EL8 prebuilt)\n";

require Devel::NYTProf::EngineDispatch;
print "OK: installed scripts nytprofhtml/csv/cg + nytprof-engine + EngineDispatch\n";
print "OK: sibling nytprof-cli present\n";

if ( my $jsonl = $ENV{NYTPROF_JSONL} ) {
    -f $jsonl or die "NYTPROF_JSONL not readable: $jsonl\n";
    my $out = `$^X $engine query --json --jsonl $jsonl 2>&1`;
    my $rc  = $? >> 8;
    die "installed nytprof-engine query failed (rc=$rc): $out\n" if $rc != 0;
    $out =~ /leaf_returns"?\s*[:=]\s*15/ or die "query missing leaf_returns=15\n$out";
    $out =~ /mid_returns"?\s*[:=]\s*3/   or die "query missing mid_returns=3\n$out";
    $out =~ /mid_leaf_edge"?\s*[:=]\s*15/
      or die "query missing mid_leaf_edge=15\n$out";
    print "OK: installed nytprof-engine query leaf=15 mid=3 edge=15\n";
}
exit 0;
