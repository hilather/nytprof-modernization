use strict;
use warnings;
sub leaf {
    my $x = 0;
    $x++ for 1 .. 50;
    return $x;
}
print "one_leaf=", leaf(), "\n";
