# SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
#
# PR-G03a–G03e / G04 — XSLoader bootstrap for product Devel::NYTProfM.
# Loads auto/Devel/NYTProfM/NYTProfM.so. Not CollectorBootstrap.

package Devel::NYTProfM::Core;

use strict;
use warnings;

use XSLoader;

our $VERSION = '6.15';    # keep in sync with Devel::NYTProfM + XSLoader

XSLoader::load( 'Devel::NYTProfM', $VERSION );

1;
