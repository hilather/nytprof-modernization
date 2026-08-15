#!/usr/bin/env perl
# PR-A2 / RPM-03: prove installed perl -d:NYTProfM attach (15/3/15)
# without nytprof-cli. Bounded v5 scanner: SUB_RETURN + SUB_CALLERS only.
use strict;
use warnings;
use File::Basename qw(dirname basename);
use File::Spec;
use File::Temp qw(tempdir);
use Cwd qw(abs_path);

my $here = abs_path( dirname($0) );
do File::Spec->catfile( $here, 'nytprof_v5_tag_table.inc' )
  or die "missing t/nytprof_v5_tag_table.inc: $@\n";

for my $inc (@INC) {
    if ( defined $inc && $inc =~ m{/collector/build(?:/|\z)} ) {
        die "t/installed_attach.t must not use repo collector/build in \@INC ($inc)\n";
    }
}

my $mod = $INC{'Devel/NYTProfM.pm'};
if ( !$mod ) {
    require Devel::NYTProfM;
    $mod = $INC{'Devel/NYTProfM.pm'};
}
die "Devel::NYTProfM not loadable from installed PERL5LIB\n" unless $mod;
if ( $mod =~ m{/collector/build(?:/|\z)} ) {
    die "loaded NYTProfM from repo dest, not installed prefix: $mod\n";
}

my $workload = File::Spec->catfile( $here, 'workload-calls1.pl' );
die "missing $workload\n" unless -f $workload;

my $tmp = tempdir( CLEANUP => 1 );
my $profile = File::Spec->catfile( $tmp, 'nytprof.out' );

{
    local $ENV{NYTPROF} = "file=$profile";
    local $ENV{PERL5OPT};
    my $perl = $^X;
    my @inc  = map { ( '-I', $_ ) } grep { defined && length } @INC;
    my $rc   = system( $perl, @inc, '-d:NYTProfM', $workload );
    die "installed perl -d:NYTProfM workload failed (rc=$rc)\n" if $rc != 0;
}
-f $profile or die "installed attach did not write $profile\n";

my ( $leaf, $mid, $edge ) = scan_profile($profile);
die "leaf SUB_RETURN=$leaf want 15\n" unless $leaf == 15;
die "mid SUB_RETURN=$mid want 3\n"    unless $mid == 3;
die "mid->leaf CALLERS=$edge want 15\n" unless $edge == 15;
print "OK: installed attach leaf=15 mid=3 edge=15\n";

{
    open my $fh, '<:raw', $profile or die "open $profile: $!\n";
    local $/;
    my $bytes = <$fh>;
    close $fh;
    die "omitted compress must write START_DEFLATE z\n"
      unless defined $bytes && $bytes =~ /^NYTProf 5 0\n/ && index( $bytes, 'z' ) >= 0;
    my $zi = index( $bytes, 'z' );
    my $cmf = ord( substr( $bytes, $zi + 1, 1 ) );
    die "omitted compress zlib CMF=$cmf want 0x78\n" unless $cmf == 0x78;
    my $plain = substr( $bytes, 0, $zi ) . inflate_v5_zlib( substr( $bytes, $zi + 1 ) );
    $plain =~ /HASH\(/
      and die "profile must not contain HASH( caller names\n";
}

# PR-S1/S2: omitted compress is zlib-6. Also assert explicit compress=1
# (level 1) still yields 15/3/15 through the same inflater.
{
    my $zprofile = File::Spec->catfile( $tmp, 'nytprof.z.out' );
    local $ENV{NYTPROF} = "file=$zprofile:compress=1";
    local $ENV{PERL5OPT};
    my $perl = $^X;
    my @inc  = map { ( '-I', $_ ) } grep { defined && length } @INC;
    my $rc   = system( $perl, @inc, '-d:NYTProfM', $workload );
    die "installed perl -d:NYTProfM compress=1 failed (rc=$rc)\n" if $rc != 0;
    -f $zprofile or die "compress=1 attach did not write $zprofile\n";
    my ( $zl, $zm, $ze ) = scan_profile($zprofile);
    die "compress=1 leaf SUB_RETURN=$zl want 15\n" unless $zl == 15;
    die "compress=1 mid SUB_RETURN=$zm want 3\n"   unless $zm == 3;
    die "compress=1 mid->leaf CALLERS=$ze want 15\n" unless $ze == 15;
    print "OK: installed attach compress=1 leaf=15 mid=3 edge=15\n";
}

# aggregate=1 is residual (ADR-0013 proposed); must not silently no-op.
{
    my $agg = File::Spec->catfile( $tmp, 'nytprof.agg' );
    local $ENV{NYTPROF} = "file=$agg:aggregate=1";
    local $ENV{PERL5OPT};
    my $perl = $^X;
    my @inc  = map { ( '-I', $_ ) } grep { defined && length } @INC;
    my $out  = `$perl @inc -d:NYTProfM -e 1 2>&1`;
    my $rc   = $? >> 8;
    die "aggregate=1 must fail (rc=$rc)\n" if $rc == 0;
    $out =~ /ADR-0013/
      or die "aggregate=1 error missing ADR-0013 text: $out\n";
    die "aggregate=1 must not write $agg\n" if -e $agg;
}
print "OK: installed aggregate=1 fail-closed\n";

# format=v6 must fail-closed on D1-B (no NYTPROF6 file).
{
    my $v6 = File::Spec->catfile( $tmp, 'nytprof.v6' );
    local $ENV{NYTPROF} = "file=$v6:format=v6";
    local $ENV{PERL5OPT};
    my $perl = $^X;
    my @inc  = map { ( '-I', $_ ) } grep { defined && length } @INC;
    my $out  = `$perl @inc -d:NYTProfM -e 1 2>&1`;
    my $rc   = $? >> 8;
    die "format=v6 must fail (rc=$rc)\n" if $rc == 0;
    $out =~ /v6_collect/ or die "format=v6 error missing v6_collect text\n";
    die "format=v6 must not write $v6\n" if -e $v6;
}
print "OK: installed format=v6 fail-closed\n";
exit 0;

# Fail-closed inflate of a v5 START_DEFLATE member (windowBits=15 zlib).
# Cap before allocating the inflated SV (SEC-004 spirit). Nested `z` is
# rejected by scan_tags(..., forbid_z => 1).
sub inflate_v5_zlib {
    my ($src) = @_;
    my $max = 64 * 1024 * 1024;
    die "deflate member oversize\n" if length($src) > $max;
    eval { require Compress::Raw::Zlib; 1 }
      or die "Compress::Raw::Zlib required to parse compress=1 profiles: $@\n";
    my ( $inf, $ist ) = Compress::Raw::Zlib::Inflate->new(
        -WindowBits  => 15,
        -Bufsize     => 65536,
        -LimitOutput => 1,
    );
    die "inflate init failed ($ist)\n" unless $inf;
    my $out   = '';
    my $input = $src;
    while ( length $input ) {
        my $chunk = '';
        my $st    = $inf->inflate( $input, $chunk );
        $out .= $chunk if defined $chunk;
        die "inflated profile exceeds 64 MiB\n" if length($out) > $max;
        return $out if $st == Compress::Raw::Zlib::Z_STREAM_END();
        die "inflate failed status=$st\n"
          unless $st == Compress::Raw::Zlib::Z_OK();
        last unless defined $chunk && length $chunk;
    }
    die "inflate incomplete (no Z_STREAM_END)\n";
}

sub scan_profile {
    my ($path) = @_;
    open my $fh, '<:raw', $path or die "open $path: $!\n";
    my $hdr = <$fh>;
    die "bad magic (not NYTProf 5)\n" unless defined $hdr && $hdr =~ /^NYTProf 5/;
    return scan_tags( $fh, 0 );
}

sub scan_tags {
    my ( $fh, $forbid_z ) = @_;
    my $leaf = 0;
    my $mid  = 0;
    my $edge = 0;
    while (1) {
        my $tag;
        last unless read( $fh, $tag, 1 );
        last if $tag eq '';
        if ( $tag eq ':' || $tag eq '!' || $tag eq '#' ) {
            my $rest = <$fh>;
            die "truncated text tag\n" unless defined $rest;
            next;
        }
        if ( $tag eq 'z' ) {
            die "nested START_DEFLATE (fail closed)\n" if $forbid_z;
            local $/;
            my $rest = <$fh>;
            $rest = '' unless defined $rest;
            my $body = inflate_v5_zlib($rest);
            open my $inf, '<:raw', \$body or die "inflate fh: $!\n";
            my ( $l2, $m2, $e2 ) = scan_tags( $inf, 1 );
            $leaf += $l2;
            $mid  += $m2;
            $edge += $e2;
            last;
        }
        if ( $tag eq '<' ) {
            read_u32($fh);    # depth (already consumed tag)
            skip_nv($fh);
            skip_nv($fh);
            my $name = read_str($fh);
            $leaf++ if $name eq 'main::leaf';
            $mid++  if $name eq 'main::mid';
            next;
        }
        if ( $tag eq 'c' ) {
            read_u32($fh);    # fid
            skip_u32($fh);    # line
            my $caller = read_str($fh);
            skip_u32($fh);    # count
            skip_nv($fh);
            skip_nv($fh);
            skip_nv($fh);
            skip_u32($fh);    # rec_depth
            my $called = read_str($fh);
            $edge += 1 if $caller eq 'main::mid' && $called eq 'main::leaf';
            next;
        }
        if ( $tag eq '+' ) {
            read_u32($fh);
            skip_u32($fh);
            skip_u32($fh);
            next;
        }
        if ( $tag eq '*' ) {
            read_u32($fh);
            skip_u32($fh);
            skip_u32($fh);
            skip_u32($fh);
            skip_u32($fh);
            next;
        }
        if ( $tag eq '-' ) {
            next;
        }
        if ( $tag eq '>' ) {
            read_u32($fh);
            skip_u32($fh);
            next;
        }
        if ( $tag eq '@' ) {
            read_u32($fh);
            skip_u32($fh);
            skip_u32($fh);
            skip_u32($fh);
            skip_u32($fh);
            skip_u32($fh);
            skip_str($fh);
            next;
        }
        if ( $tag eq 'S' ) {
            read_u32($fh);
            skip_u32($fh);
            skip_str($fh);
            next;
        }
        if ( $tag eq 's' ) {
            read_u32($fh);
            skip_str($fh);
            skip_u32($fh);
            skip_u32($fh);
            next;
        }
        if ( $tag eq 'P' ) {
            read_u32($fh);
            skip_u32($fh);
            skip_nv($fh);
            next;
        }
        if ( $tag eq 'p' ) {
            read_u32($fh);
            skip_nv($fh);
            next;
        }
        die sprintf( "unknown v5 tag 0x%02x (fail closed)\n", ord($tag) );
    }
    return ( $leaf, $mid, $edge );
}

sub read_u32 {
    my ($fh) = @_;
    my $b = read_byte($fh);
    if ( $b < 0x80 ) {
        return $b;
    }
    if ( $b < 0xC0 ) {
        return ( ( $b & 0x3F ) << 8 ) | read_byte($fh);
    }
    if ( $b < 0xE0 ) {
        return ( ( $b & 0x1F ) << 16 ) | ( read_byte($fh) << 8 ) | read_byte($fh);
    }
    if ( $b < 0xFF ) {
        return ( ( $b & 0x0F ) << 24 )
          | ( read_byte($fh) << 16 )
          | ( read_byte($fh) << 8 )
          | read_byte($fh);
    }
    return ( read_byte($fh) << 24 )
      | ( read_byte($fh) << 16 )
      | ( read_byte($fh) << 8 )
      | read_byte($fh);
}

sub skip_u32 { read_u32(@_) }

sub read_byte {
    my ($fh) = @_;
    my $buf;
    my $n = read( $fh, $buf, 1 );
    die "truncated u32/tag payload\n" unless $n && $n == 1;
    return ord($buf);
}

sub skip_nv {
    my ($fh) = @_;
    my $buf;
    my $n = read( $fh, $buf, 8 );
    die "truncated NV\n" unless $n && $n == 8;
}

sub read_str {
    my ($fh) = @_;
    my $stag = read_byte($fh);
    die "expected string tag\n" unless $stag == ord("'") || $stag == ord('"');
    my $len = read_u32($fh);
    die "oversize string $len\n" if $len > $NYTProfM::V5TagTable::MAX_STR + 0;
    return '' if $len == 0;
    my $buf;
    my $n = read( $fh, $buf, $len );
    die "truncated string\n" unless $n && $n == $len;
    return $buf;
}

sub skip_str { read_str(@_) }
