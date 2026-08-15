#!/usr/bin/env perl
# Core-only text analyzer for the Rocky 8 Docker profile demo.
#
# Core-only by design for the Rocky lab (no Time::HiRes clock; C nytp_clock).
# PR-7: Getopt::Long / Exporter / Time::HiRes now compile under -d:NYTProfM
# (INIT $DB::single + goto &$raw). This scanner stays core-only.
use strict;
use warnings;

my $dir     = $ARGV[0] or die "usage: minute_text_scanner.pl DIR [SECONDS]\n";
my $seconds = defined $ARGV[1] ? 0 + $ARGV[1] : 60;
$seconds > 0 or die "SECONDS must be > 0\n";
my $deadline = time + $seconds;

sub tokenize {
    my ($text) = @_;
    my @w = $text =~ /([A-Za-z']{2,})/g;
    return \@w;
}

sub classify {
    my ($words) = @_;
    my %freq;
    $freq{ lc $_ }++ for @$words;
    return \%freq;
}

sub merge_freq {
    my ( $acc, $part ) = @_;
    while ( my ( $k, $v ) = each %$part ) {
        $acc->{$k} += $v;
    }
    return $acc;
}

sub scan_file {
    my ($path) = @_;
    open my $fh, '<', $path or return {};
    local $/;
    my $text = <$fh> // '';
    close $fh;
    return classify( tokenize($text) );
}

sub collect_files {
    my ($root) = @_;
    my @files;
    my @stack = ($root);
    while ( my $d = shift @stack ) {
        opendir my $sd, $d or next;
        while ( my $n = readdir $sd ) {
            next if $n eq '.' or $n eq '..';
            my $p = "$d/$n";
            if    ( -d $p ) { push @stack, $p }
            elsif ( -f $p ) { push @files, $p }
        }
        closedir $sd;
    }
    return @files;
}

sub top_n {
    my ( $acc, $n ) = @_;
    my @keys = sort { $acc->{$b} <=> $acc->{$a} || $a cmp $b } keys %$acc;
    splice @keys, $n if @keys > $n;
    return @keys;
}

my @files = collect_files($dir);
@files or die "no files under $dir\n";

my $acc    = {};
my $passes = 0;
while ( time < $deadline ) {
    $passes++;
    for my $f (@files) {
        last if time >= $deadline;
        merge_freq( $acc, scan_file($f) );
    }
}

my @top  = top_n( $acc, 8 );
my $keys = scalar keys %$acc;
print "passes=$passes files=" . scalar(@files) . " vocab=$keys\n";
print "top=" . join( ',', map { "$_:$acc->{$_}" } @top ) . "\n";
