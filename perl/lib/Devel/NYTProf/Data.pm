package Devel::NYTProf::Data;
# Product Data materializer path (PR-A06 / OQ-2 / toward PERL-005).
#
# Opens a binary v5 profile (or dump JSONL) and exposes the advertised
# JsonlData query surface. Binary open uses native `nytprof-cli dump` →
# JsonlData (thin product bridge). Not full COMPAT-007 bless-array fidelity,
# not in-process pure-XS binary decode, not oracle Devel::NYTProf::Data.
#
# Spec: docs/schemas/perl-xs-data-readstream-mvp-v0.md
# Policy: docs/adrs/0003-r1-full-residual-policy.md (OQ-2: must CLOSE, not waive)

use strict;
use warnings;
use Carp       qw(croak);
use Cwd        qw(abs_path getcwd);
use File::Basename qw(dirname);
use File::Spec;

use Devel::NYTProf::EngineDispatch qw(find_native_cli find_repo_root);
use Devel::NYTProf::JsonlData;

our $VERSION = '0.001';

# Methods delegated to the underlying JsonlData instance.
my @DELEGATE = qw(
  sub_returns
  sub_return_totals
  call_edge_count
  call_edge_totals
  line_totals
  line_calls
  block_line_totals
  block_line_calls
  sub_def
  sub_defs
  file
  files
  file_basename
  source_line
  source_lines
  attribute
  attributes
  option
  options
  pid_start_count
  pid_end_count
  pid_start_events
  pid_end_events
  pid_starts
  pid_ends
  pids
  records_seen
  time_line_events
  time_block_events
  discount_events
  discount_count
  sub_entry_events
  sub_entry_count
  sub_return_events
  new_fid_events
  sub_callers_events
  src_line_events
  sub_info_events
  is_stream_complete
  stream_incompleteness_reasons
);

# ---------------------------------------------------------------------------
# Constructors
# ---------------------------------------------------------------------------

## Oracle-ish constructor.
##
##   my $data = Devel::NYTProf::Data->new({ filename => $profile });
##   my $data = Devel::NYTProf::Data->new({ filename => $profile, quiet => 1 });
##
## Options (hashref or flat %opts after class):
##   filename / profile / file  => binary profile path (preferred product path)
##   jsonl                      => dump JSONL path (bridge; no native CLI)
##   repo_root                  => workspace root for CLI discovery
##   allow_incomplete           => truthy → skip COMPAT-010 completeness croak
##   quiet                      => accepted for oracle API shape (no-op)
##
sub new {
    my $class = shift;
    my %opts;
    if ( @_ == 1 && ref( $_[0] ) eq 'HASH' ) {
        %opts = %{ $_[0] };
    }
    else {
        %opts = @_;
    }

    my $profile =
         $opts{filename}
      // $opts{profile}
      // $opts{file};
    my $jsonl = $opts{jsonl};

    if ( defined $jsonl && length $jsonl ) {
        return $class->from_jsonl( $jsonl, %opts );
    }
    if ( defined $profile && length $profile ) {
        return $class->from_profile( $profile, %opts );
    }
    croak
"Devel::NYTProf::Data->new: require filename/profile (binary) or jsonl => PATH";
}

## Open a binary v5 profile via native dump → JsonlData.
##
##   my $data = Devel::NYTProf::Data->from_profile($path);
##   my $data = Devel::NYTProf::Data->from_profile($path, repo_root => $repo);
##
sub from_profile {
    my ( $class, $path, %opts ) = @_;
    croak "from_profile: path required"
      unless defined $path && length $path;
    croak "from_profile: profile not readable: $path"
      unless -f $path && -r $path;

    my $repo = $opts{repo_root};
    if ( !defined $repo || !length $repo ) {
        $repo = eval { find_repo_root( dirname( abs_path($path) // $path ) ) };
        if ( !defined $repo || !length $repo ) {
            $repo = eval { find_repo_root( getcwd() ) };
        }
    }
    croak "from_profile: cannot locate workspace root for native CLI discovery"
      unless defined $repo && length $repo;

    my $cli = find_native_cli($repo);
    my @argv;
    if ( $cli->{mode} eq 'path' ) {
        @argv = ( $cli->{path}, '--engine=native', 'dump', $path );
    }
    elsif ( $cli->{mode} eq 'cargo' ) {
        @argv = ( @{ $cli->{argv} }, '--engine=native', 'dump', $path );
    }
    else {
        croak "from_profile: unknown native cli mode";
    }

    my $inner = Devel::NYTProf::JsonlData->from_cli( \@argv );
    my $self  = $class->_wrap(
        $inner,
        backend     => 'native-cli-jsonl',
        source_path => abs_path($path) // $path,
        repo_root   => $repo,
    );
    $self->_maybe_require_complete( \%opts );
    return $self;
}

## Build from a committed golden / saved dump JSONL (bridge still available).
sub from_jsonl {
    my ( $class, $path, %opts ) = @_;
    croak "from_jsonl: path required"
      unless defined $path && length $path;
    my $inner = Devel::NYTProf::JsonlData->from_jsonl($path);
    my $self  = $class->_wrap(
        $inner,
        backend     => 'jsonl-file',
        source_path => abs_path($path) // $path,
    );
    $self->_maybe_require_complete( \%opts );
    return $self;
}

## Build by spawning a CLI that writes JSONL to stdout.
sub from_cli {
    my ( $class, $cli, %opts ) = @_;
    my $inner = Devel::NYTProf::JsonlData->from_cli( $cli, %opts );
    my $self  = $class->_wrap(
        $inner,
        backend     => 'jsonl-cli',
        source_path => undef,
    );
    $self->_maybe_require_complete( \%opts );
    return $self;
}

## Wrap an existing JsonlData instance (tests / advanced callers).
sub from_jsonl_data {
    my ( $class, $inner, %opts ) = @_;
    croak "from_jsonl_data: JsonlData object required"
      unless defined $inner && ref($inner) && $inner->isa('Devel::NYTProf::JsonlData');
    my $self = $class->_wrap(
        $inner,
        backend     => $opts{backend} // 'jsonl-wrap',
        source_path => $opts{source_path},
    );
    $self->_maybe_require_complete( \%opts );
    return $self;
}

# ---------------------------------------------------------------------------
# Product metadata
# ---------------------------------------------------------------------------

## Backend kind: native-cli-jsonl | jsonl-file | jsonl-cli | jsonl-wrap
sub backend {
    my ($self) = @_;
    return $self->{backend};
}

## Absolute path of the opened profile or JSONL when known.
sub source_path {
    my ($self) = @_;
    return $self->{source_path};
}

## Always true for this product facade (vs pure JsonlData bridge-only).
sub is_product_path { 1 }

## Materializer implementation tag (residual honesty).
##
##   'thin-native-cli-jsonl' — binary via dump subprocess → JsonlData
##   'jsonl-bridge'          — JSONL file / CLI / wrap only
##
sub materializer {
    my ($self) = @_;
    return $self->{backend} eq 'native-cli-jsonl'
      ? 'thin-native-cli-jsonl'
      : 'jsonl-bridge';
}

## Whether COMPAT-007 full bless-array / oracle AV-HV fidelity is claimed.
## Always false for this MVP (residual honesty).
sub claims_compat007_shapes { 0 }

## Underlying JsonlData object (advanced / dual-path tests).
sub jsonl_data {
    my ($self) = @_;
    return $self->{inner};
}

# ---------------------------------------------------------------------------
# Delegation
# ---------------------------------------------------------------------------

for my $meth (@DELEGATE) {
    no strict 'refs';
    *{$meth} = sub {
        my $self = shift;
        return $self->{inner}->$meth(@_);
    };
}

# ---------------------------------------------------------------------------
# Internals
# ---------------------------------------------------------------------------

sub _wrap {
    my ( $class, $inner, %meta ) = @_;
    return bless {
        inner       => $inner,
        backend     => $meta{backend} // 'jsonl-bridge',
        source_path => $meta{source_path},
        repo_root   => $meta{repo_root},
    }, $class;
}

## Default: croak when stream incomplete (COMPAT-010 verify/report posture).
## Pass allow_incomplete => 1 to load partial dumps (tests / salvage).
sub _maybe_require_complete {
    my ( $self, $opts ) = @_;
    return if $opts->{allow_incomplete};
    return if $self->is_stream_complete;
    my $reasons = $self->stream_incompleteness_reasons;
    my $msg     = join( '; ', @$reasons );
    $msg = 'incomplete stream' unless length $msg;
    croak "Devel::NYTProf::Data: $msg (COMPAT-010; set allow_incomplete to override)";
}

1;

__END__

=head1 NAME

Devel::NYTProf::Data - product Data materializer over binary profiles (PR-A06)

=head1 SYNOPSIS

  use Devel::NYTProf::Data;

  # Product path: binary v5 profile → native dump → queryable aggregates
  my $data = Devel::NYTProf::Data->new({ filename => 'nytprof.out' });
  # or
  my $data = Devel::NYTProf::Data->from_profile('fixtures/v5/default-calls1/nytprof.out');

  printf "leaf returns=%d\n", $data->sub_returns('main::leaf');   # 15
  printf "mid returns=%d\n",  $data->sub_returns('main::mid');    # 3
  printf "mid->leaf=%d\n",
    $data->call_edge_count('main::mid', 'main::leaf');            # 15

  # Bridge still available (no native CLI): golden dump JSONL
  my $from_dump = Devel::NYTProf::Data->from_jsonl(
      'fixtures/v5/default-calls1/readstream.jsonl'
  );

  # Product metadata / residual honesty
  $data->backend;                  # 'native-cli-jsonl'
  $data->materializer;             # 'thin-native-cli-jsonl'
  $data->claims_compat007_shapes;  # 0 (not full bless-array fidelity)

=head1 DESCRIPTION

Product path for residual rows B<PERL-004/005> (OQ-2 B<CLOSE> via PR-A06).
Binary profiles are opened by spawning the native CLI dump into
L<Devel::NYTProf::JsonlData> (thin materializer). Pure-Perl C<JsonlData>
remains the dump-JSONL bridge; this module is the B<product> Data facade
callers should prefer when they have a binary profile.

This is B<not>:

=over 4

=item *

Full oracle C<Devel::NYTProf::Data> (bless-array / COMPAT-007 shapes)

=item *

In-process pure-XS binary wire decode without the native dump bridge

=item *

A waiver of OQ-2 — this implements the close path (MVP / partial)

=back

Default open is fail-closed on incomplete streams (COMPAT-010). Pass
C<allow_incomplete =E<gt> 1> to load partial dumps intentionally.

Never puts C<crates/> on oracle C<PERL5LIB>. Does not load the oracle
C<Devel::NYTProf> install tree.

=head1 SEE ALSO

L<https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-xs-data-readstream-mvp-v0.md>,
L<Devel::NYTProf::ReadStream>,
L<Devel::NYTProf::JsonlData>,
L<https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md>

=cut
