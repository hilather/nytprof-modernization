package Devel::NYTProf::CollectorBootstrap;

use strict;
use warnings;

our $VERSION = '0.001';

# Load-only scaffold (PR-G02). Not Devel::NYTProf, not -d:NYTProf.
# Do not set $Devel::NYTProf::PRODUCT_XS_ATTACH — attach is not ready.
require XSLoader;
XSLoader::load( __PACKAGE__, $VERSION );

1;
