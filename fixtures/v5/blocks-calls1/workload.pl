use strict;
use warnings;
sub leaf {
    my $x = 0;
    $x++ for 1 .. 50;
    return $x;
}
sub mid {
    my $s = 0;
    $s += leaf() for 1 .. 5;
    return $s;
}
my $total = 0;
$total += mid() for 1 .. 3;
print "total=$total\n";
