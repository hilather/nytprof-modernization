#!/usr/bin/env perl
# RPM-03 / PR-A2: prove installed perl -d:NYTProfM attach (15/3/15) without nytprof-cli.
# PR-A1 ships this file in NYTProfM-6.15.tar.gz. Bounded tag parser lands in PR-A2
# (t/nytprof_v5_tag_table.inc). SKIP is intentional: mock %check must not
# invoke this file until the parser lands (PR-A2).
use strict;
use warnings;

print "SKIP: t/installed_attach.t parser lands in PR-A2 (RPM-03)\n";
exit 0;
