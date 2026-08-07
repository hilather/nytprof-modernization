#!/usr/bin/env perl
# Structural comparator for two ReadStream JSONL dumps (canonical-event-dump-v0).
#
# Compares tag+args only; seq is ignored (may differ if renumbered).
# This tool is intentionally pure: it does not normalize volatile fields.
#
# For golden / cross-run compare, normalize first:
#   python3 tools/oracle/normalize_jsonl.py a.jsonl > a.norm.jsonl
#   python3 tools/oracle/normalize_jsonl.py b.jsonl > b.norm.jsonl
#   perl tools/oracle/compare_jsonl.pl a.norm.jsonl b.norm.jsonl
#
# See: docs/schemas/canonical-event-dump-v0.md
#      tools/oracle/normalize_jsonl.py
#      tools/oracle/selftest_harness.sh
#
# Exit 0 if equal (tag+args); 1 on first mismatch; 2 on usage/IO error.
use strict;
use warnings;
use JSON::PP qw(decode_json);

my @files;
for my $arg (@ARGV) {
    if ( $arg eq '--help' || $arg eq '-h' ) {
        print <<'USAGE';
Usage: compare_jsonl.pl expected.jsonl actual.jsonl

Compare two canonical event JSONL dumps by tag+args (seq ignored).

Does not normalize volatiles (COMMENT, basetime, application paths, …).
Run tools/oracle/normalize_jsonl.py on both sides first for golden compare.
USAGE
        exit 0;
    }
    if ( $arg eq '--normalize' ) {
        die "$0: --normalize is not supported; pipe both files through "
          . "normalize_jsonl.py first (keeps compare pure).\n";
    }
    if ( $arg =~ /^-/ ) {
        die "Unknown option: $arg (try --help)\n";
    }
    push @files, $arg;
}

@files == 2 or die "Usage: $0 expected.jsonl actual.jsonl\n";
open my $a, '<:raw', $files[0] or die "open $files[0]: $!\n";
open my $b, '<:raw', $files[1] or die "open $files[1]: $!\n";

my $line = 0;
while (1) {
    my $la = <$a>;
    my $lb = <$b>;
    last if !defined $la && !defined $lb;
    $line++;
    if ( !defined $la || !defined $lb ) {
        die "Length mismatch at line $line\n";
    }
    chomp $la;
    chomp $lb;
    next if $la eq '' && $lb eq '';
    my $ja = decode_json($la);
    my $jb = decode_json($lb);
    # Compare tag+args; seq may differ if renumbered
    my $ka = JSON::PP->new->canonical(1)->encode(
        { tag => $ja->{tag}, args => $ja->{args} } );
    my $kb = JSON::PP->new->canonical(1)->encode(
        { tag => $jb->{tag}, args => $jb->{args} } );
    if ( $ka ne $kb ) {
        warn "Mismatch at line $line seq_a=$ja->{seq} seq_b=$jb->{seq}\n";
        warn "  expected: $ka\n";
        warn "  actual:   $kb\n";
        exit 1;
    }
}
print "OK: $line records match (tag+args)\n";
exit 0;
