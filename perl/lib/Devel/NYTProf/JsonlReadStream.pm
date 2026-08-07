package Devel::NYTProf::JsonlReadStream;
# Pure-Perl ReadStream-from-dump bridge.
#
# Consumes canonical JSONL (oracle golden or native `nytprof-cli dump`) and
# invokes callback-style handlers for tags. No XS, no FFI, no oracle PERL5LIB.
#
# Spec: docs/schemas/perl-jsonl-readstream-mvp-v0.md
# Record shape: docs/schemas/canonical-event-dump-v0.md

use strict;
use warnings;
use Carp     qw(croak);
use JSON::PP ();

our $VERSION = '0.001';

use Exporter qw(import);
our @EXPORT_OK = qw(
  process_jsonl
  process_fh
  process_fh_handlers
  for_chunks
  count_sub_returns
  SUB_RETURN_SUBNAME_INDEX
  TIME_LINE_TICKS_INDEX
);

# Canonical arg indices (ReadStream / dump schema order).
# SUB_RETURN: depth, incl_time, excl_time, subname
use constant SUB_RETURN_SUBNAME_INDEX => 3;
# TIME_LINE: ticks, fid, line
use constant TIME_LINE_TICKS_INDEX => 0;

# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

## Process a JSONL file path with a single callback.
##
##   for_chunks(sub { my ($tag, $args, $seq) = @_; ... }, file => $path);
##   for_chunks($cb, fh => $fh);
##   for_chunks($cb, from_cli => [ $cli, 'dump', $profile ]);
##
## Options (exactly one input source required unless both file and fh omitted
## and from_cli is set):
##   file      => path to JSONL
##   fh        => open filehandle (read)
##   from_cli  => arrayref argv; runs as subprocess, reads its stdout as JSONL
##   on_error  => 'croak' (default) | 'skip'  — how to handle bad lines
##
## Returns number of records successfully delivered to the callback
## (including synthetic _END when present).
sub for_chunks {
    my ( $callback, %opts ) = @_;
    croak "for_chunks: CODE callback required"
      unless defined $callback && ref($callback) eq 'CODE';

    my $on_error = $opts{on_error} // 'croak';
    $on_error = lc $on_error;
    croak "for_chunks: on_error must be 'croak' or 'skip'"
      unless $on_error eq 'croak' || $on_error eq 'skip';

    my $fh;
    my $close_fh = 0;
    my $pid;

    if ( my $cli = $opts{from_cli} ) {
        croak "for_chunks: from_cli must be a non-empty arrayref"
          unless ref($cli) eq 'ARRAY' && @$cli;
        croak "for_chunks: cannot combine from_cli with file/fh"
          if defined $opts{file} || defined $opts{fh};

        my @cmd = @$cli;
        $pid = open( my $pipe, '-|', @cmd );
        if ( !defined $pid ) {
            croak "for_chunks: failed to spawn from_cli ($cmd[0]): $!";
        }
        $fh      = $pipe;
        $close_fh = 1;
    }
    elsif ( defined $opts{fh} ) {
        $fh = $opts{fh};
        croak "for_chunks: fh is not a valid handle"
          unless defined $fh && ( ref($fh) || fileno($fh) );
    }
    elsif ( defined $opts{file} ) {
        my $path = $opts{file};
        croak "for_chunks: file path required" unless length $path;
        open( my $in, '<:encoding(UTF-8)', $path )
          or croak "for_chunks: cannot open $path: $!";
        $fh      = $in;
        $close_fh = 1;
    }
    else {
        croak "for_chunks: require file, fh, or from_cli";
    }

    my $n = eval {
        process_fh( $fh, $callback, on_error => $on_error );
    };
    my $err = $@;

    if ($close_fh) {
        close $fh;
        if ( defined $pid ) {
            my $status = $?;
            if ( !$err && $status != 0 ) {
                my $exit = $status >> 8;
                my $sig  = $status & 127;
                $err =
                  $sig
                  ? "for_chunks: from_cli killed by signal $sig"
                  : "for_chunks: from_cli exited $exit";
            }
        }
    }

    croak $err if $err;
    return $n;
}

## Process a JSONL path with per-tag handler hash.
##
##   process_jsonl($path, {
##     SUB_RETURN => sub { my ($args, $seq) = @_; ... },
##     TIME_LINE  => sub { my ($args, $seq) = @_; ... },
##     '*'        => sub { my ($tag, $args, $seq) = @_; ... },  # optional default
##   });
##
## Returns number of JSONL records parsed (same as process_fh).
## Tags without a matching handler (and no '*') are not delivered to a
## handler, but still count toward the parse total.
sub process_jsonl {
    my ( $path, $handlers, %opts ) = @_;
    croak "process_jsonl: path required"
      unless defined $path && length $path;
    croak "process_jsonl: handlers hashref required"
      unless defined $handlers && ref($handlers) eq 'HASH';

    open( my $fh, '<:encoding(UTF-8)', $path )
      or croak "process_jsonl: cannot open $path: $!";
    my $n = process_fh_handlers( $fh, $handlers, %opts );
    close $fh;
    return $n;
}

## Like process_jsonl but from an open handle.
sub process_fh_handlers {
    my ( $fh, $handlers, %opts ) = @_;
    croak "process_fh_handlers: handlers hashref required"
      unless defined $handlers && ref($handlers) eq 'HASH';

    my $wildcard = $handlers->{'*'};
    return process_fh(
        $fh,
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
        %opts
    );
}

## Low-level: iterate JSONL lines from $fh, invoke $callback->($tag, $args, $seq).
##
## Blank lines skipped. Synthetic _END is delivered like any other tag.
## Returns count of successfully parsed records passed to the callback.
sub process_fh {
    my ( $fh, $callback, %opts ) = @_;
    croak "process_fh: CODE callback required"
      unless defined $callback && ref($callback) eq 'CODE';
    croak "process_fh: fh required" unless defined $fh;

    my $on_error = lc( $opts{on_error} // 'croak' );
    my $json     = JSON::PP->new->utf8(0);
    my $count    = 0;
    my $line_no  = 0;

    while ( defined( my $raw = <$fh> ) ) {
        $line_no++;
        # Strip BOM on first line if present; trim trailing newline.
        if ( $line_no == 1 ) {
            $raw =~ s/\A\x{FEFF}//;
            $raw =~ s/\A\xEF\xBB\xBF//;
        }
        $raw =~ s/\r?\n\z//;
        next if $raw =~ /\A\s*\z/;

        my $obj = eval { $json->decode($raw) };
        if ($@) {
            my $msg = "JSON decode error at line $line_no: $@";
            $msg =~ s/\s+at\s+\S+\s+line\s+\d+\.?\s*\z//;
            if ( $on_error eq 'skip' ) {
                warn "JsonlReadStream: $msg\n";
                next;
            }
            croak $msg;
        }
        if ( ref($obj) ne 'HASH' ) {
            my $msg = "line $line_no: expected JSON object, got "
              . ( defined $obj ? ref($obj) || 'scalar' : 'undef' );
            if ( $on_error eq 'skip' ) {
                warn "JsonlReadStream: $msg\n";
                next;
            }
            croak $msg;
        }

        my $tag = $obj->{tag};
        if ( !defined $tag || !length $tag ) {
            my $msg = "line $line_no: missing tag";
            if ( $on_error eq 'skip' ) {
                warn "JsonlReadStream: $msg\n";
                next;
            }
            croak $msg;
        }

        my $args = $obj->{args};
        if ( !defined $args ) {
            $args = [];
        }
        elsif ( ref($args) ne 'ARRAY' ) {
            my $msg = "line $line_no: args must be array for tag $tag";
            if ( $on_error eq 'skip' ) {
                warn "JsonlReadStream: $msg\n";
                next;
            }
            croak $msg;
        }

        my $seq = $obj->{seq};
        $callback->( $tag, $args, $seq );
        $count++;
    }

    return $count;
}

## Convenience: count SUB_RETURN events by subname from a JSONL path.
##
## Returns hashref { subname => count }. Uses canonical arg index 3.
sub count_sub_returns {
    my ($path) = @_;
    my %counts;
    process_jsonl(
        $path,
        {
            SUB_RETURN => sub {
                my ($args) = @_;
                return unless defined $args && @$args > SUB_RETURN_SUBNAME_INDEX;
                my $name = $args->[SUB_RETURN_SUBNAME_INDEX];
                return unless defined $name && length $name;
                $counts{$name}++;
            },
        }
    );
    return \%counts;
}

1;

__END__

=head1 NAME

Devel::NYTProf::JsonlReadStream - pure-Perl ReadStream-from-JSONL dump bridge

=head1 SYNOPSIS

  use Devel::NYTProf::JsonlReadStream qw(for_chunks process_jsonl count_sub_returns);

  # Tag-keyed handlers (file path)
  my %returns;
  process_jsonl(
      'fixtures/v5/default-calls1/readstream.jsonl',
      {
          SUB_RETURN => sub {
              my ($args, $seq) = @_;
              # args: depth, incl_time, excl_time, subname
              my $name = $args->[3];
              $returns{$name}++ if defined $name;
          },
          TIME_LINE => sub {
              my ($args, $seq) = @_;
              # args: ticks, fid, line
          },
      }
  );

  # Single callback over all tags
  for_chunks(
      sub {
          my ($tag, $args, $seq) = @_;
          ...
      },
      file => 'fixtures/v5/default-calls1/readstream.jsonl',
  );

  # Subprocess native dump (no oracle PERL5LIB)
  for_chunks(
      sub { my ($tag, $args, $seq) = @_; ... },
      from_cli => [ 'prefix/bin/nytprof-cli', 'dump', 'path/to/nytprof.out' ],
  );

=head1 DESCRIPTION

MVP bridge that turns a canonical event dump (JSONL) into ReadStream-style
callbacks. Intended for pure-Perl tooling before full XS C<ReadStream>
(PERL-004). Uses core C<JSON::PP> only.

Argument order matches L<docs/schemas/canonical-event-dump-v0.md>:

  SUB_RETURN  => depth, incl_time, excl_time, subname
  TIME_LINE   => ticks, fid, line

Does B<not> load oracle C<Devel::NYTProf> and never places C<crates/> on
C<PERL5LIB>.

=head1 SEE ALSO

F<docs/schemas/perl-jsonl-readstream-mvp-v0.md>,
F<docs/schemas/canonical-event-dump-v0.md>

=cut
