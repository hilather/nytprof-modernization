package Devel::NYTProf::ReadStream;
# Product ReadStream path (PR-A06 / OQ-2 / toward PERL-004).
#
# Streams tags from a binary v5 profile (or dump JSONL) with the same
# callback shape as JsonlReadStream / oracle ReadStream for_chunks.
# Binary open uses native `nytprof-cli dump` → JsonlReadStream (thin
# product bridge). Not pure-XS binary wire decode; not full scalar-flag
# fidelity package.
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
use Devel::NYTProf::JsonlReadStream ();

our $VERSION = '0.001';

use Exporter qw(import);
our @EXPORT_OK = qw(
  for_chunks
  process_profile
  process_jsonl
  count_sub_returns
  SUB_RETURN_SUBNAME_INDEX
  TIME_LINE_TICKS_INDEX
  is_product_path
  materializer_kind
);

use constant SUB_RETURN_SUBNAME_INDEX =>
  Devel::NYTProf::JsonlReadStream::SUB_RETURN_SUBNAME_INDEX;
use constant TIME_LINE_TICKS_INDEX =>
  Devel::NYTProf::JsonlReadStream::TIME_LINE_TICKS_INDEX;

# ---------------------------------------------------------------------------
# Product metadata
# ---------------------------------------------------------------------------

## Always true for this product facade.
sub is_product_path { 1 }

## Residual honesty: thin native-cli-jsonl bridge (not pure-XS wire decode).
sub materializer_kind { 'thin-native-cli-jsonl' }

# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

## Walk tags with a single callback (oracle-ish + product options).
##
##   for_chunks(sub { my ($tag, $args, $seq) = @_; ... },
##              filename => $profile);          # binary product path
##   for_chunks($cb, profile => $profile);
##   for_chunks($cb, file => $jsonl_path);      # dump JSONL bridge
##   for_chunks($cb, jsonl => $jsonl_path);
##   for_chunks($cb, fh => $fh);
##   for_chunks($cb, from_cli => [ $cli, 'dump', $profile ]);
##
## Exactly one input source is required. Binary C<filename>/C<profile>
## requires a discoverable native CLI (see L<Devel::NYTProf::EngineDispatch>).
##
## Options:
##   repo_root => workspace root for CLI discovery (optional)
##   on_error  => 'croak' (default) | 'skip' — bad JSONL lines
##
## Returns number of records delivered to the callback.
sub for_chunks {
    my ( $callback, %opts ) = @_;
    croak "for_chunks: CODE callback required"
      unless defined $callback && ref($callback) eq 'CODE';

    # Product binary path: filename / profile
    my $binary = $opts{filename} // $opts{profile};
    if ( defined $binary && length $binary ) {
        croak "for_chunks: cannot combine filename/profile with file/fh/from_cli/jsonl"
          if defined $opts{file}
          || defined $opts{fh}
          || defined $opts{from_cli}
          || defined $opts{jsonl};
        my @argv = _dump_argv_for_profile( $binary, $opts{repo_root} );
        return Devel::NYTProf::JsonlReadStream::for_chunks(
            $callback,
            from_cli => \@argv,
            ( exists $opts{on_error} ? ( on_error => $opts{on_error} ) : () ),
        );
    }

    # jsonl alias → file
    if ( defined $opts{jsonl} && length $opts{jsonl} ) {
        croak "for_chunks: cannot combine jsonl with file/fh/from_cli"
          if defined $opts{file} || defined $opts{fh} || defined $opts{from_cli};
        return Devel::NYTProf::JsonlReadStream::for_chunks(
            $callback,
            file => $opts{jsonl},
            ( exists $opts{on_error} ? ( on_error => $opts{on_error} ) : () ),
        );
    }

    # Pass-through to JsonlReadStream for file / fh / from_cli
    if (   defined $opts{file}
        || defined $opts{fh}
        || defined $opts{from_cli} )
    {
        my %pass = map { $_ => $opts{$_} }
          grep { exists $opts{$_} } qw(file fh from_cli on_error);
        return Devel::NYTProf::JsonlReadStream::for_chunks( $callback, %pass );
    }

    croak
"for_chunks: require filename/profile (binary), jsonl/file, fh, or from_cli";
}

## Process a binary profile with per-tag handlers.
##
##   process_profile($profile, {
##     SUB_RETURN => sub { my ($args, $seq) = @_; ... },
##     '*'        => sub { my ($tag, $args, $seq) = @_; ... },
##   });
##
sub process_profile {
    my ( $path, $handlers, %opts ) = @_;
    croak "process_profile: path required"
      unless defined $path && length $path;
    croak "process_profile: handlers hashref required"
      unless defined $handlers && ref($handlers) eq 'HASH';

    my $wildcard = $handlers->{'*'};
    return for_chunks(
        sub {
            my ( $tag, $args, $seq ) = @_;
            my $h = $handlers->{$tag};
            if ( defined $h && ref($h) eq 'CODE' ) {
                $h->( $args, $seq );
                return;
            }
            if ( defined $wildcard && ref($wildcard) eq 'CODE' ) {
                $wildcard->( $tag, $args, $seq );
            }
        },
        filename => $path,
        %opts,
    );
}

## Dump-JSONL bridge (re-export convenience).
sub process_jsonl {
    return Devel::NYTProf::JsonlReadStream::process_jsonl(@_);
}

## Count SUB_RETURN by subname from a JSONL path (bridge convenience).
sub count_sub_returns {
    return Devel::NYTProf::JsonlReadStream::count_sub_returns(@_);
}

# ---------------------------------------------------------------------------
# Internals
# ---------------------------------------------------------------------------

sub _dump_argv_for_profile {
    my ( $path, $repo_root ) = @_;
    croak "binary profile not readable: $path"
      unless -f $path && -r $path;

    my $repo = $repo_root;
    if ( !defined $repo || !length $repo ) {
        $repo = eval { find_repo_root( dirname( abs_path($path) // $path ) ) };
        if ( !defined $repo || !length $repo ) {
            $repo = eval { find_repo_root( getcwd() ) };
        }
    }
    croak "cannot locate workspace root for native CLI discovery"
      unless defined $repo && length $repo;

    my $cli = find_native_cli($repo);
    if ( $cli->{mode} eq 'path' ) {
        return ( $cli->{path}, '--engine=native', 'dump', $path );
    }
    if ( $cli->{mode} eq 'cargo' ) {
        return ( @{ $cli->{argv} }, '--engine=native', 'dump', $path );
    }
    croak "unknown native cli mode";
}

1;

__END__

=head1 NAME

Devel::NYTProf::ReadStream - product ReadStream over binary profiles (PR-A06)

=head1 SYNOPSIS

  use Devel::NYTProf::ReadStream qw(for_chunks process_profile);

  my %returns;
  for_chunks(
      sub {
          my ($tag, $args, $seq) = @_;
          if ($tag eq 'SUB_RETURN' && @$args > 3) {
              $returns{ $args->[3] }++;
          }
      },
      filename => 'fixtures/v5/default-calls1/nytprof.out',  # binary
  );

  # Dump JSONL bridge still works
  for_chunks(
      sub { my ($tag, $args, $seq) = @_; ... },
      jsonl => 'fixtures/v5/default-calls1/readstream.jsonl',
  );

=head1 DESCRIPTION

Product path for residual row B<PERL-004> (OQ-2 B<CLOSE> via PR-A06).
Binary profiles are streamed by spawning the native CLI dump into
L<Devel::NYTProf::JsonlReadStream> (thin materializer). Argument order
matches the canonical dump schema.

This is B<not> full pure-XS binary wire decode, not oracle scalar-flag
fidelity, and B<not> a waiver of OQ-2 — this implements the close path
(MVP / partial). C<JsonlReadStream> remains available as the dump-JSONL
bridge.

Never puts C<crates/> on oracle C<PERL5LIB>.

=head1 SEE ALSO

L<https://github.com/hilather/nytprof-modernization/blob/main/docs/schemas/perl-xs-data-readstream-mvp-v0.md>,
L<Devel::NYTProf::Data>,
L<Devel::NYTProf::JsonlReadStream>,
L<https://github.com/hilather/nytprof-modernization/blob/main/docs/adrs/0003-r1-full-residual-policy.md>

=cut
