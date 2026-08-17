#!/usr/bin/env perl
# Bounded DBI + DBD::SQLite attach driver — in-memory DB, no daemon.
use strict;
use warnings;
use DBI;

sub lab_run {
    my $secs = $ENV{NYTPROF_DEMO_SECONDS} || 2;
    $secs = 2 if $secs !~ /^\d+$/ || $secs < 1;
    $secs = 30 if $secs > 30;
    my $dbh = DBI->connect( 'dbi:SQLite:dbname=:memory:', '', '',
        { RaiseError => 1, PrintError => 0 } );
    $dbh->do('CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, n INT)');
    my $ins = $dbh->prepare('INSERT INTO t (name, n) VALUES (?, ?)');
    my $sel = $dbh->prepare('SELECT name, n FROM t WHERE n = ?');
    my $until = time + $secs;
    my $n     = 0;
    my $sum   = 0;
    while ( time < $until ) {
        $n++;
        $ins->execute( "row-$n", $n );
        $sel->execute($n);
        my $row = $sel->fetchrow_hashref;
        $sum += $row->{n} if $row;
    }
    my ($cnt) = $dbh->selectrow_array('SELECT COUNT(*) FROM t');
    $dbh->disconnect;
    die "dbi lab produced no passes" if $n < 1;
    print "dbi_sqlite_lab_ok passes=$n rows=$cnt sum=$sum secs=$secs dbi=$DBI::VERSION\n";
}

lab_run();
1;
