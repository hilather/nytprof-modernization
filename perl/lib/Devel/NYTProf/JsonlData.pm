package Devel::NYTProf::JsonlData;
# Pure-Perl Data-from-dump query object.
#
# Builds queryable subroutine return totals, call-edge counts, A4 line
# totals (TIME_LINE + TIME_BLOCK), A4b block_line_totals (TIME_BLOCK
# only), A8 source lines (SRC_LINE), A9 sub defs (SUB_INFO), file
# identity (NEW_FID), profile metadata (ATTRIBUTE / OPTION), process
# lifecycle (PID_START / PID_END), A3 discount event multiplicity
# (DISCOUNT), SUB_ENTRY event multiplicity (calls=2 call-site entries),
# stream event multiplicities for SUB_RETURN / NEW_FID / SUB_CALLERS /
# SRC_LINE / SUB_INFO (JSON-EVENT-COUNTS-MVP), and stream-completeness
# checks aligned with COMPAT-010_INCOMPLETE_STREAM from canonical JSONL
# (oracle golden or native `nytprof-cli dump`). No XS, no FFI, no oracle
# PERL5LIB. Core JSON::PP via JsonlReadStream only.
#
# Spec: docs/schemas/perl-jsonl-data-mvp-v0.md
# Stream: Devel::NYTProf::JsonlReadStream
# Completeness: docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md
# Aggregates: docs/schemas/aggregate-comparison-v0.md
#   (A3 discount_events, A4 lines, A4b block_line, A5 returns, A7 edges,
#    A8 source_lines, A9 sub_defs; files from NEW_FID; attributes/options
#    from header; PID_START / PID_END process events; TIME_LINE/TIME_BLOCK
#    counters; SUB_ENTRY + SUB_RETURN/NEW_FID/SUB_CALLERS/SRC_LINE/SUB_INFO
#    multiplicity. DISCOUNT and SUB_ENTRY are event multiplicity only —
#    not exclusive-time policy freeze / full call-stack arg freeze.)

use strict;
use warnings;
use Carp qw(croak);

use Devel::NYTProf::JsonlReadStream qw(
  for_chunks
  SUB_RETURN_SUBNAME_INDEX
  TIME_LINE_TICKS_INDEX
);

our $VERSION = '0.001';

# Canonical arg indices (ReadStream / dump schema order).
# SUB_RETURN: depth, incl_time, excl_time, subname — name @ SUB_RETURN_SUBNAME_INDEX (3)
# SUB_CALLERS: fid, line, count, incl, excl, reci, rec_depth, called, caller
# Verified against fixtures: caller = args[-1], callee = args[-2], count = args[2]
use constant SUB_CALLERS_COUNT_INDEX  => 2;
use constant SUB_CALLERS_CALLED_INDEX => 7;    # args[-2]
use constant SUB_CALLERS_CALLER_INDEX => 8;    # args[-1]
use constant SUB_CALLERS_MIN_ARGS     => 9;
# TIME_LINE:  ticks, fid, line
# TIME_BLOCK: ticks, fid, line, block_line, sub_line
# Statement (fid, line) layout is shared; both contribute to A4 line_totals.
# A4b uses (fid, block_line) from TIME_BLOCK only (block_line @ args[3]).
use constant TIME_STMT_FID_INDEX        => 1;
use constant TIME_STMT_LINE_INDEX       => 2;
use constant TIME_BLOCK_BLOCK_LINE_INDEX => 3;
use constant TIME_BLOCK_MIN_ARGS         => 4;    # ticks,fid,line,block_line
# SUB_INFO: fid, first_line, last_line, name — A9 last-write-wins
use constant SUB_INFO_FID_INDEX        => 0;
use constant SUB_INFO_FIRST_LINE_INDEX => 1;
use constant SUB_INFO_LAST_LINE_INDEX  => 2;
use constant SUB_INFO_NAME_INDEX       => 3;
use constant SUB_INFO_MIN_ARGS         => 4;
# NEW_FID: fid, eval_fid, eval_line, flags, size, mtime, name — path is last arg
use constant NEW_FID_FID_INDEX => 0;
use constant NEW_FID_MIN_ARGS  => 2;    # at least fid + path
# SRC_LINE: fid, line, text — A8 last-write-wins
use constant SRC_LINE_FID_INDEX  => 0;
use constant SRC_LINE_LINE_INDEX => 1;
use constant SRC_LINE_TEXT_INDEX => 2;
use constant SRC_LINE_MIN_ARGS   => 3;
# ATTRIBUTE / OPTION: key, value — last-write-wins per key
use constant META_KEY_INDEX   => 0;
use constant META_VALUE_INDEX => 1;
use constant META_MIN_ARGS    => 2;
# PID_START: pid, ppid?, start_time?  — dump e.g. [2975381, 2975366, 1786111723.96777]
# PID_END:   pid, end_time?           — dump e.g. [2975381, 1786111723.97052]
use constant PID_PID_INDEX        => 0;
use constant PID_START_PPID_INDEX => 1;
use constant PID_START_TIME_INDEX => 2;
use constant PID_END_TIME_INDEX   => 1;
use constant PID_MIN_ARGS         => 1;    # at least pid

# ---------------------------------------------------------------------------
# Constructors
# ---------------------------------------------------------------------------

## Build from a JSONL file path (golden dump or saved native dump).
##
##   my $data = Devel::NYTProf::JsonlData->from_jsonl($path);
##
sub from_jsonl {
    my ( $class, $path, %opts ) = @_;
    croak "from_jsonl: path required"
      unless defined $path && length $path;
    my $self = $class->_new;
    $self->_ingest( file => $path, %opts );
    return $self;
}

## Build by spawning a CLI that writes JSONL to stdout (typically native dump).
##
##   my $data = Devel::NYTProf::JsonlData->from_cli(
##     [ $nytprof_cli, 'dump', $profile_path ]
##   );
##
sub from_cli {
    my ( $class, $cli, %opts ) = @_;
    croak "from_cli: non-empty arrayref argv required"
      unless defined $cli && ref($cli) eq 'ARRAY' && @$cli;
    my $self = $class->_new;
    $self->_ingest( from_cli => $cli, %opts );
    return $self;
}

## Build from an open readable handle of JSONL lines.
sub from_fh {
    my ( $class, $fh, %opts ) = @_;
    croak "from_fh: fh required" unless defined $fh;
    my $self = $class->_new;
    $self->_ingest( fh => $fh, %opts );
    return $self;
}

# ---------------------------------------------------------------------------
# Queries
# ---------------------------------------------------------------------------

## Return count for one subname (from SUB_RETURN events). Missing → 0.
sub sub_returns {
    my ( $self, $name ) = @_;
    croak "sub_returns: subname required"
      unless defined $name && length $name;
    return $self->{sub_returns}{$name} // 0;
}

## Hashref copy: subname => return count (all names seen in SUB_RETURN).
sub sub_return_totals {
    my ($self) = @_;
    return { %{ $self->{sub_returns} } };
}

## Aggregated call-edge count for (caller, callee) from SUB_CALLERS.
## Sums `count` across all sites for the same pair. Missing → 0.
##
##   $data->call_edge_count('main::mid', 'main::leaf');  # 15 on default-calls1
##
sub call_edge_count {
    my ( $self, $caller, $callee ) = @_;
    croak "call_edge_count: caller required"
      unless defined $caller;
    croak "call_edge_count: callee required"
      unless defined $callee;
    my $key = _edge_key( $caller, $callee );
    return $self->{call_edges}{$key} // 0;
}

## Hashref copy of call edges: "caller\tcallee" => total count.
sub call_edge_totals {
    my ($self) = @_;
    return { %{ $self->{call_edges} } };
}

## A4-ish line totals from TIME_LINE and TIME_BLOCK (statement line field).
## Hashref: "fid:line" => { calls => N, ticks => sum }.
## Empty if neither timing tag was seen.
sub line_totals {
    my ($self) = @_;
    my %out;
    while ( my ( $k, $v ) = each %{ $self->{line_totals} } ) {
        $out{$k} = { calls => $v->[0], ticks => $v->[1] };
    }
    return \%out;
}

## Call count for one (fid, line) from line_totals (TIME_LINE + TIME_BLOCK).
## Missing → 0.
##
##   $data->line_calls(1, 5);  # 780 on blocks-calls1 (TIME_BLOCK)
##
sub line_calls {
    my ( $self, $fid, $line ) = @_;
    croak "line_calls: fid required"  unless defined $fid;
    croak "line_calls: line required" unless defined $line;
    my $v = $self->{line_totals}{"$fid:$line"};
    return $v ? $v->[0] : 0;
}

## A4b block-line totals from TIME_BLOCK only (block start line field).
## Hashref: "fid:block_line" => { calls => N, ticks => sum }.
## Empty when the dump has no TIME_BLOCK events (e.g. default-calls1).
sub block_line_totals {
    my ($self) = @_;
    my %out;
    while ( my ( $k, $v ) = each %{ $self->{block_line_totals} } ) {
        $out{$k} = { calls => $v->[0], ticks => $v->[1] };
    }
    return \%out;
}

## Call count for one (fid, block_line) from block_line_totals (TIME_BLOCK only).
## Missing → 0.
##
##   $data->block_line_calls(1, 4);  # 810 on blocks-calls1 (A4b sample)
##
sub block_line_calls {
    my ( $self, $fid, $block_line ) = @_;
    croak "block_line_calls: fid required"        unless defined $fid;
    croak "block_line_calls: block_line required" unless defined $block_line;
    my $v = $self->{block_line_totals}{"$fid:$block_line"};
    return $v ? $v->[0] : 0;
}

## A9 sub definition range for one name from SUB_INFO (last write wins).
## Returns hashref { fid, first_line, last_line } or undef if missing.
##
##   my $d = $data->sub_def('main::leaf');  # { fid=>1, first_line=>3, last_line=>7 }
##
sub sub_def {
    my ( $self, $name ) = @_;
    croak "sub_def: subname required"
      unless defined $name && length $name;
    my $d = $self->{sub_defs}{$name};
    return undef unless defined $d;
    return { %$d };
}

## Hashref copy: subname => { fid, first_line, last_line } (all SUB_INFO names).
sub sub_defs {
    my ($self) = @_;
    my %out;
    while ( my ( $k, $v ) = each %{ $self->{sub_defs} } ) {
        $out{$k} = { %$v };
    }
    return \%out;
}

## Full path recorded for fid from NEW_FID (last write wins). Missing → undef.
##
##   $data->file(1);  # ".../workload.pl" on default-calls1
##
sub file {
    my ( $self, $fid ) = @_;
    croak "file: fid required" unless defined $fid;
    return $self->{files}{$fid};
}

## Hashref copy: fid => path (all NEW_FID entries).
sub files {
    my ($self) = @_;
    return { %{ $self->{files} } };
}

## Basename of the path recorded for fid. Missing path → undef.
##
##   $data->file_basename(1);  # "workload.pl" on default-calls1
##
sub file_basename {
    my ( $self, $fid ) = @_;
    croak "file_basename: fid required" unless defined $fid;
    my $path = $self->{files}{$fid};
    return undef unless defined $path && length $path;
    return _path_basename($path);
}

## A8 source text for one (fid, line) from SRC_LINE (last write wins).
## Returns the exact dump text string, or undef if missing.
##
##   $data->source_line(1, 5);  # "    $x++ for 1 .. 50;\n" on default-calls1
##
sub source_line {
    my ( $self, $fid, $line ) = @_;
    croak "source_line: fid required"  unless defined $fid;
    croak "source_line: line required" unless defined $line;
    return $self->{source_lines}{"$fid:$line"};
}

## Hashref copy: "fid:line" => source text (all SRC_LINE entries).
sub source_lines {
    my ($self) = @_;
    return { %{ $self->{source_lines} } };
}

## Profile ATTRIBUTE value for one key (last write wins). Missing → undef.
##
##   $data->attribute('ticks_per_sec');  # e.g. "10000000" on default-calls1
##   $data->attribute('basetime');
##
sub attribute {
    my ( $self, $key ) = @_;
    croak "attribute: key required"
      unless defined $key && length $key;
    return $self->{attributes}{$key};
}

## Hashref copy: attribute key => value (all ATTRIBUTE entries).
sub attributes {
    my ($self) = @_;
    return { %{ $self->{attributes} } };
}

## Profile OPTION value for one key (last write wins). Missing → undef.
##
##   $data->option('calls');  # e.g. "1" on default-calls1
##
sub option {
    my ( $self, $key ) = @_;
    croak "option: key required"
      unless defined $key && length $key;
    return $self->{options}{$key};
}

## Hashref copy: option key => value (all OPTION entries).
sub options {
    my ($self) = @_;
    return { %{ $self->{options} } };
}

## Number of PID_START events seen (same as scalar @{pid_starts()}).
sub pid_start_count {
    my ($self) = @_;
    return $self->{pid_start_events} // 0;
}

## Number of PID_END events seen (same as scalar @{pid_ends()}).
sub pid_end_count {
    my ($self) = @_;
    return $self->{pid_end_events} // 0;
}

## Alias counters (match Rust model naming / incomplete-stream contract).
sub pid_start_events { shift->pid_start_count }
sub pid_end_events   { shift->pid_end_count }

## Arrayref copy of PID_START records: each { pid => N, ppid => ?, start_time => ? }.
## Only defined optional fields are present. Empty arrayref if none.
##
##   my $starts = $data->pid_starts;  # [ { pid => 2975381, ppid => ..., start_time => ... } ]
##
sub pid_starts {
    my ($self) = @_;
    return [ map { {%$_} } @{ $self->{pid_starts} } ];
}

## Arrayref copy of PID_END records: each { pid => N, end_time => ? }.
## Only defined optional fields are present. Empty arrayref if none.
##
##   my $ends = $data->pid_ends;  # [ { pid => 2975381, end_time => ... } ]
##
sub pid_ends {
    my ($self) = @_;
    return [ map { {%$_} } @{ $self->{pid_ends} } ];
}

## Sorted unique list of PIDs seen in PID_START and/or PID_END (arrayref of ints).
## Does not invent PIDs — only values from dump events.
##
##   $data->pids;  # [ 2975381 ] on default-calls1 golden
##
sub pids {
    my ($self) = @_;
    my %seen;
    for my $ev ( @{ $self->{pid_starts} }, @{ $self->{pid_ends} } ) {
        $seen{ $ev->{pid} } = 1 if defined $ev->{pid};
    }
    return [ sort { $a <=> $b } keys %seen ];
}

## Number of JSONL records processed (including synthetic _END if present).
sub records_seen {
    my ($self) = @_;
    return $self->{records_seen} // 0;
}

## Number of TIME_LINE events successfully ingested (statement timing counter).
sub time_line_events {
    my ($self) = @_;
    return $self->{time_line_events} // 0;
}

## Number of TIME_BLOCK events successfully ingested (statement timing counter).
sub time_block_events {
    my ($self) = @_;
    return $self->{time_block_events} // 0;
}

## Number of DISCOUNT events successfully ingested (A3 multiplicity only).
##
## DISCOUNT tags have empty args in the dump schema; each tag increments by 1.
## This is event multiplicity accounting only — not exclusive-time policy
## freeze / fake-clock discount semantics (BASE-003 / TEST-003).
##
##   $data->discount_events;  # 818 on default-calls1 golden
##
sub discount_events {
    my ($self) = @_;
    return $self->{discount_events} // 0;
}

## Alias for discount_events (A3 aggregate name / count wording).
sub discount_count { shift->discount_events }

## Number of SUB_ENTRY events successfully ingested (multiplicity only).
##
## SUB_ENTRY is emitted when NYTPROF calls>=2; args are caller_fid,
## caller_line. Each tag increments by 1. This is event multiplicity
## accounting only — not full call-stack / arg freeze.
##
##   $data->sub_entry_events;  # 0 on default-calls1; 27 on calls2-default
##
sub sub_entry_events {
    my ($self) = @_;
    return $self->{sub_entry_events} // 0;
}

## Alias for sub_entry_events (count wording).
sub sub_entry_count { shift->sub_entry_events }

## Number of SUB_RETURN events successfully ingested (multiplicity only).
##
## One increment per matching tag whose args yield a usable subname.
## Not the same as sum of sub_return_totals when names collide (still 1/tag).
##
##   $data->sub_return_events;  # 27 on default-calls1 golden
##
sub sub_return_events {
    my ($self) = @_;
    return $self->{sub_return_events} // 0;
}

## Number of NEW_FID events successfully ingested (multiplicity only).
##
##   $data->new_fid_events;  # 3 on default-calls1 golden
##
sub new_fid_events {
    my ($self) = @_;
    return $self->{new_fid_events} // 0;
}

## Number of SUB_CALLERS events successfully ingested (multiplicity only).
##
## One increment per matching tag successfully parsed (not sum of edge counts).
##
##   $data->sub_callers_events;  # 13 on default-calls1 golden
##
sub sub_callers_events {
    my ($self) = @_;
    return $self->{sub_callers_events} // 0;
}

## Number of SRC_LINE events successfully ingested (A8 stream multiplicity).
##
##   $data->src_line_events;  # 632 on default-calls1 golden
##
sub src_line_events {
    my ($self) = @_;
    return $self->{src_line_events} // 0;
}

## Number of SUB_INFO events successfully ingested (A9 stream multiplicity).
##
##   $data->sub_info_events;  # 31 on default-calls1 golden
##
sub sub_info_events {
    my ($self) = @_;
    return $self->{sub_info_events} // 0;
}

## Whether the ingested stream is complete enough for default verify/report.
##
## Aligned with ProfileModel / docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md:
## true iff stream_incompleteness_reasons() is empty.
##
##   $data->is_stream_complete;  # 1 on default-calls1 golden
##
sub is_stream_complete {
    my ($self) = @_;
    return @{ $self->stream_incompleteness_reasons } == 0 ? 1 : 0;
}

## Human-readable reasons the stream is incomplete (empty arrayref if complete).
##
## Rules (COMPAT-010_INCOMPLETE_STREAM):
## 1. PID balance: if pid_start_events > 0, require pid_end_events >= pid_start_events
## 2. Statement timing: time_line_events + time_block_events > 0
##    (equivalently: line_totals non-empty when counters are used)
##
## Returns a new arrayref each call (caller may mutate).
##
##   my $r = $data->stream_incompleteness_reasons;  # [] when complete
##
sub stream_incompleteness_reasons {
    my ($self) = @_;
    my @reasons;
    my $ps = $self->{pid_start_events}  // 0;
    my $pe = $self->{pid_end_events}    // 0;
    my $tl = $self->{time_line_events}  // 0;
    my $tb = $self->{time_block_events} // 0;
    if ( $ps > 0 && $pe < $ps ) {
        push @reasons, 'missing PID_END after PID_START';
    }
    if ( ( $tl + $tb ) == 0 ) {
        push @reasons, 'no statement timing events (TIME_LINE/TIME_BLOCK)';
    }
    return \@reasons;
}

# ---------------------------------------------------------------------------
# Internals
# ---------------------------------------------------------------------------

sub _new {
    my ($class) = @_;
    return bless {
        sub_returns       => {},    # name => count
        call_edges        => {},    # "caller\tcallee" => count
        line_totals       => {},    # "fid:line" => [calls, ticks]
        block_line_totals => {},    # "fid:block_line" => [calls, ticks] (TIME_BLOCK)
        sub_defs          => {},    # name => { fid, first_line, last_line }
        files             => {},    # fid => path (full; last write wins)
        source_lines      => {},    # "fid:line" => text (last write wins)
        attributes        => {},    # key => value (ATTRIBUTE; last write wins)
        options           => {},    # key => value (OPTION; last write wins)
        pid_starts        => [],    # [ { pid, ppid?, start_time? }, ... ]
        pid_ends          => [],    # [ { pid, end_time? }, ... ]
        pid_start_events  => 0,
        pid_end_events    => 0,
        time_line_events  => 0,     # TIME_LINE count (stream completeness)
        time_block_events => 0,     # TIME_BLOCK count (stream completeness)
        discount_events   => 0,     # DISCOUNT count (A3 multiplicity only)
        sub_entry_events  => 0,     # SUB_ENTRY count (multiplicity only)
        sub_return_events => 0,     # SUB_RETURN count (multiplicity; JSON-EVENT-COUNTS-MVP)
        new_fid_events    => 0,     # NEW_FID count (multiplicity)
        sub_callers_events => 0,    # SUB_CALLERS tag count (not edge sum)
        src_line_events   => 0,     # SRC_LINE count (A8 stream)
        sub_info_events   => 0,     # SUB_INFO count (A9 stream)
        records_seen      => 0,
    }, $class;
}

sub _edge_key {
    my ( $caller, $callee ) = @_;
    return "$caller\t$callee";
}

## Basename of a path-like string (/, \ separators). Bare names unchanged.
sub _path_basename {
    my ($path) = @_;
    return $path unless defined $path && length $path;
    if ( $path =~ m{([^/\\]+)\z} ) {
        my $base = $1;
        return length($base) ? $base : $path;
    }
    return $path;
}

sub _ingest {
    my ( $self, %src ) = @_;

    my $sub_returns       = $self->{sub_returns};
    my $call_edges        = $self->{call_edges};
    my $line_totals       = $self->{line_totals};
    my $block_line_totals = $self->{block_line_totals};
    my $sub_defs          = $self->{sub_defs};
    my $files             = $self->{files};
    my $source_lines      = $self->{source_lines};
    my $attributes        = $self->{attributes};
    my $options           = $self->{options};
    my $pid_starts        = $self->{pid_starts};
    my $pid_ends          = $self->{pid_ends};

    my $n = for_chunks(
        sub {
            my ( $tag, $args, $seq ) = @_;
            if ( $tag eq 'SUB_RETURN' ) {
                return
                  unless defined $args
                  && @$args > SUB_RETURN_SUBNAME_INDEX;
                my $name = $args->[SUB_RETURN_SUBNAME_INDEX];
                return unless defined $name && length $name;
                $sub_returns->{$name}++;
                # JSON-EVENT-COUNTS-MVP: one per successfully ingested tag.
                $self->{sub_return_events}++;
            }
            elsif ( $tag eq 'SUB_CALLERS' ) {
                return
                  unless defined $args && @$args >= SUB_CALLERS_MIN_ARGS;
                # Prefer fixed schema indices; also consistent with args[-2]/[-1].
                my $count  = $args->[SUB_CALLERS_COUNT_INDEX];
                my $callee = $args->[SUB_CALLERS_CALLED_INDEX];
                my $caller = $args->[SUB_CALLERS_CALLER_INDEX];
                return
                  unless defined $callee
                  && defined $caller
                  && !ref($callee)
                  && !ref($caller);
                # count may be JSON number; coerce carefully
                $count = 0 unless defined $count && !ref($count);
                $count = int($count);
                my $key = _edge_key( $caller, $callee );
                $call_edges->{$key} += $count;
                # JSON-EVENT-COUNTS-MVP: tag multiplicity (not sum of counts).
                $self->{sub_callers_events}++;
            }
            elsif ( $tag eq 'TIME_LINE' || $tag eq 'TIME_BLOCK' ) {
                # A4: both tags attribute to statement (fid, line).
                # TIME_LINE args: ticks, fid, line
                # TIME_BLOCK args: ticks, fid, line, block_line, sub_line
                # Also count events for COMPAT-010 stream completeness.
                return unless defined $args && @$args > TIME_STMT_LINE_INDEX;
                my $ticks = $args->[TIME_LINE_TICKS_INDEX] // 0;
                my $fid   = $args->[TIME_STMT_FID_INDEX];
                my $line  = $args->[TIME_STMT_LINE_INDEX];
                return unless defined $fid && defined $line;
                $ticks = 0 if ref($ticks);
                $ticks = 0 + $ticks;
                my $key = "$fid:$line";
                if ( !exists $line_totals->{$key} ) {
                    $line_totals->{$key} = [ 0, 0 ];
                }
                $line_totals->{$key}[0]++;
                $line_totals->{$key}[1] += $ticks;

                if ( $tag eq 'TIME_LINE' ) {
                    $self->{time_line_events}++;
                }
                else {
                    # TIME_BLOCK: statement timing count + optional A4b
                    $self->{time_block_events}++;
                    # A4b: block start line from TIME_BLOCK only (block_line @ args[3]).
                    if ( @$args >= TIME_BLOCK_MIN_ARGS ) {
                        my $block_line = $args->[TIME_BLOCK_BLOCK_LINE_INDEX];
                        if ( defined $block_line && !ref($block_line) ) {
                            my $bkey = "$fid:$block_line";
                            if ( !exists $block_line_totals->{$bkey} ) {
                                $block_line_totals->{$bkey} = [ 0, 0 ];
                            }
                            $block_line_totals->{$bkey}[0]++;
                            $block_line_totals->{$bkey}[1] += $ticks;
                        }
                    }
                }
            }
            elsif ( $tag eq 'SUB_INFO' ) {
                # A9: fid, first_line, last_line, name — last write wins per name.
                return
                  unless defined $args && @$args >= SUB_INFO_MIN_ARGS;
                my $fid   = $args->[SUB_INFO_FID_INDEX];
                my $first = $args->[SUB_INFO_FIRST_LINE_INDEX];
                my $last  = $args->[SUB_INFO_LAST_LINE_INDEX];
                my $name  = $args->[SUB_INFO_NAME_INDEX];
                return
                  unless defined $name
                  && length $name
                  && !ref($name)
                  && defined $fid
                  && defined $first
                  && defined $last
                  && !ref($fid)
                  && !ref($first)
                  && !ref($last);
                $sub_defs->{$name} = {
                    fid        => int($fid),
                    first_line => int($first),
                    last_line  => int($last),
                };
                # JSON-EVENT-COUNTS-MVP: A9 stream multiplicity.
                $self->{sub_info_events}++;
            }
            elsif ( $tag eq 'NEW_FID' ) {
                # fid @ 0; path is last arg (schema: name). Store full path.
                return
                  unless defined $args && @$args >= NEW_FID_MIN_ARGS;
                my $fid  = $args->[NEW_FID_FID_INDEX];
                my $path = $args->[-1];
                return
                  unless defined $fid
                  && !ref($fid)
                  && defined $path
                  && !ref($path)
                  && length $path;
                $files->{ int($fid) } = $path;
                # JSON-EVENT-COUNTS-MVP: one per successfully ingested NEW_FID.
                $self->{new_fid_events}++;
            }
            elsif ( $tag eq 'SRC_LINE' ) {
                # A8: fid, line, text — last write wins per (fid, line).
                return
                  unless defined $args && @$args >= SRC_LINE_MIN_ARGS;
                my $fid  = $args->[SRC_LINE_FID_INDEX];
                my $line = $args->[SRC_LINE_LINE_INDEX];
                my $text = $args->[SRC_LINE_TEXT_INDEX];
                return
                  unless defined $fid
                  && !ref($fid)
                  && defined $line
                  && !ref($line)
                  && defined $text
                  && !ref($text);
                $source_lines->{"$fid:$line"} = $text;
                # JSON-EVENT-COUNTS-MVP: A8 stream multiplicity.
                $self->{src_line_events}++;
            }
            elsif ( $tag eq 'ATTRIBUTE' ) {
                # key, value — last write wins per key; store dump values as-is.
                return unless defined $args && @$args >= META_MIN_ARGS;
                my $key = $args->[META_KEY_INDEX];
                my $val = $args->[META_VALUE_INDEX];
                return
                  unless defined $key
                  && length $key
                  && !ref($key)
                  && defined $val
                  && !ref($val);
                $attributes->{$key} = $val;
            }
            elsif ( $tag eq 'OPTION' ) {
                # key, value — last write wins per key; store dump values as-is.
                return unless defined $args && @$args >= META_MIN_ARGS;
                my $key = $args->[META_KEY_INDEX];
                my $val = $args->[META_VALUE_INDEX];
                return
                  unless defined $key
                  && length $key
                  && !ref($key)
                  && defined $val
                  && !ref($val);
                $options->{$key} = $val;
            }
            elsif ( $tag eq 'PID_START' ) {
                # pid [, ppid [, start_time ]] — store dump-derived values only.
                return unless defined $args && @$args >= PID_MIN_ARGS;
                my $pid = $args->[PID_PID_INDEX];
                return unless defined $pid && !ref($pid);
                my $ev = { pid => int($pid) };
                if (   @$args > PID_START_PPID_INDEX
                    && defined $args->[PID_START_PPID_INDEX]
                    && !ref( $args->[PID_START_PPID_INDEX] ) )
                {
                    $ev->{ppid} = int( $args->[PID_START_PPID_INDEX] );
                }
                if (   @$args > PID_START_TIME_INDEX
                    && defined $args->[PID_START_TIME_INDEX]
                    && !ref( $args->[PID_START_TIME_INDEX] ) )
                {
                    $ev->{start_time} = 0 + $args->[PID_START_TIME_INDEX];
                }
                push @$pid_starts, $ev;
                $self->{pid_start_events}++;
            }
            elsif ( $tag eq 'PID_END' ) {
                # pid [, end_time ] — store dump-derived values only.
                return unless defined $args && @$args >= PID_MIN_ARGS;
                my $pid = $args->[PID_PID_INDEX];
                return unless defined $pid && !ref($pid);
                my $ev = { pid => int($pid) };
                if (   @$args > PID_END_TIME_INDEX
                    && defined $args->[PID_END_TIME_INDEX]
                    && !ref( $args->[PID_END_TIME_INDEX] ) )
                {
                    $ev->{end_time} = 0 + $args->[PID_END_TIME_INDEX];
                }
                push @$pid_ends, $ev;
                $self->{pid_end_events}++;
            }
            elsif ( $tag eq 'DISCOUNT' ) {
                # A3: empty-args marker; event multiplicity only (not time policy).
                $self->{discount_events}++;
            }
            elsif ( $tag eq 'SUB_ENTRY' ) {
                # calls>=2 call-site entry; multiplicity only (not arg freeze).
                # Schema: caller_fid, caller_line — count every SUB_ENTRY tag.
                $self->{sub_entry_events}++;
            }
        },
        %src
    );

    $self->{records_seen} = $n // 0;
    return $self;
}

1;

__END__

=head1 NAME

Devel::NYTProf::JsonlData - pure-Perl Data-from-JSONL dump query object

=head1 SYNOPSIS

  use Devel::NYTProf::JsonlData;

  my $data = Devel::NYTProf::JsonlData->from_jsonl(
      'fixtures/v5/default-calls1/readstream.jsonl'
  );

  # Subroutine return counts (from real SUB_RETURN events)
  $data->sub_returns('main::leaf');     # 15
  $data->sub_returns('main::mid');      # 3
  my $all = $data->sub_return_totals;   # { 'main::leaf' => 15, ... }

  # Call-edge counts (from real SUB_CALLERS events)
  $data->call_edge_count('main::mid', 'main::leaf');  # 15

  # A4 line totals from TIME_LINE and/or TIME_BLOCK
  my $lines = $data->line_totals;  # { "1:5" => { calls => N, ticks => T }, ... }
  $data->line_calls(1, 5);         # 780 on blocks-calls1 (TIME_BLOCK)

  # A4b block_line totals from TIME_BLOCK only (block start line)
  my $blocks = $data->block_line_totals;  # { "1:4" => { calls => 810, ... }, ... }
  $data->block_line_calls(1, 4);          # 810 on blocks-calls1 (A4b sample)

  # A9 sub definitions from SUB_INFO (last write wins)
  my $leaf = $data->sub_def('main::leaf');
  # { fid => 1, first_line => 3, last_line => 7 } on default-calls1
  my $mid = $data->sub_def('main::mid');
  # { fid => 1, first_line => 8, last_line => 12 }

  # File identity from NEW_FID (full path stored; basename helper)
  $data->file(1);             # ".../workload.pl"
  $data->file_basename(1);    # "workload.pl"

  # A8 source text from SRC_LINE (last write wins; exact dump string)
  $data->source_line(1, 5);   # "    $x++ for 1 .. 50;\n" on default-calls1
  my $src = $data->source_lines;  # { "1:5" => "...", ... }

  # Profile metadata from ATTRIBUTE / OPTION (last write wins; dump values)
  $data->attribute('ticks_per_sec');  # e.g. "10000000"
  $data->attribute('basetime');
  $data->option('calls');             # e.g. "1" on default-calls1
  my $attrs = $data->attributes;      # { key => value, ... }
  my $opts  = $data->options;

  # Process lifecycle from PID_START / PID_END (dump-derived; do not invent PIDs)
  $data->pid_start_count;             # >= 1 on default-calls1
  $data->pid_end_count;               # >= 1 on default-calls1
  my $starts = $data->pid_starts;     # [ { pid => 2975381, ppid => ..., start_time => ... } ]
  my $ends   = $data->pid_ends;       # [ { pid => 2975381, end_time => ... } ]
  $data->pids;                        # unique pids, e.g. [ 2975381 ]

  # Stream completeness (COMPAT-010_INCOMPLETE_STREAM; same rules as ProfileModel)
  $data->time_line_events;            # TIME_LINE count
  $data->time_block_events;           # TIME_BLOCK count
  $data->is_stream_complete;          # true on default-calls1 golden
  my $reasons = $data->stream_incompleteness_reasons;  # [] when complete

  # A3 DISCOUNT event multiplicity (not exclusive-time policy freeze)
  $data->discount_events;             # 818 on default-calls1 golden
  $data->discount_count;              # alias for discount_events

  # SUB_ENTRY event multiplicity (calls>=2; not full call-stack arg freeze)
  $data->sub_entry_events;            # 0 on default-calls1; 27 on calls2-default
  $data->sub_entry_count;             # alias for sub_entry_events

  # Stream event multiplicities (JSON-EVENT-COUNTS-MVP; match ProfileModel)
  $data->sub_return_events;           # 27 on default-calls1
  $data->new_fid_events;              # 3
  $data->sub_callers_events;          # 13
  $data->src_line_events;             # 632
  $data->sub_info_events;             # 31

  # Native dump subprocess (no oracle PERL5LIB)
  my $live = Devel::NYTProf::JsonlData->from_cli(
      [ 'prefix/bin/nytprof-cli', 'dump', 'fixtures/v5/default-calls1/nytprof.out' ]
  );

=head1 DESCRIPTION

MVP pure-Perl aggregator that turns a canonical event dump (JSONL) into
queryable subroutine, call-edge, line totals, block-line totals (A4b),
source lines, sub definition ranges, file identity, profile
ATTRIBUTE/OPTION metadata, process lifecycle (C<PID_START> /
C<PID_END>), A3 C<DISCOUNT> event multiplicity (C<discount_events> /
C<discount_count>), C<SUB_ENTRY> event multiplicity (C<sub_entry_events> /
C<sub_entry_count>), stream event multiplicities for C<SUB_RETURN> /
C<NEW_FID> / C<SUB_CALLERS> / C<SRC_LINE> / C<SUB_INFO>
(C<sub_return_events>, C<new_fid_events>, C<sub_callers_events>,
C<src_line_events>, C<sub_info_events>; JSON-EVENT-COUNTS-MVP), and
stream-completeness checks aligned with C<COMPAT-010_INCOMPLETE_STREAM>.
Intended as a lightweight Data-like surface before full XS
C<Devel::NYTProf::Data> (PERL-*).

Uses L<Devel::NYTProf::JsonlReadStream> for JSONL parsing (core C<JSON::PP>
only). Does B<not> load oracle C<Devel::NYTProf> and never places C<crates/>
on C<PERL5LIB>. C<DISCOUNT> and C<SUB_ENTRY> accounting is event
multiplicity only — not exclusive-time policy freeze or full call-stack
arg freeze.

=head2 Stream completeness

Aligned with Rust C<ProfileModel::is_stream_complete> /
C<stream_incompleteness_reasons> and
F<docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md>:

  1. If pid_start_events > 0, require pid_end_events >= pid_start_events
  2. Require time_line_events + time_block_events > 0

C<is_stream_complete> is true iff C<stream_incompleteness_reasons> is empty.
Model load / C<from_jsonl> may still succeed on incomplete streams; callers
that need verify/report semantics should check completeness explicitly.

=head2 Argument shapes

Aligned with dump schema / aggregate-comparison-v0:

  SUB_RETURN  => depth, incl_time, excl_time, subname   # name @ index 3
  SUB_CALLERS => fid, line, count, incl, excl, reci, rec_depth, called, caller
                # count @ 2, callee @ 7 (args[-2]), caller @ 8 (args[-1])
  TIME_LINE   => ticks, fid, line
  TIME_BLOCK  => ticks, fid, line, block_line, sub_line
                # A4 uses statement (fid, line) — same indices as TIME_LINE
                # A4b uses (fid, block_line) @ args[1], args[3] (TIME_BLOCK only)
  SRC_LINE    => fid, line, text
                # A8 source_lines: "fid:line" => text; last write wins
  SUB_INFO    => fid, first_line, last_line, name
                # A9 sub_defs: name => { fid, first_line, last_line }; last write wins
  NEW_FID     => fid, eval_fid, eval_line, flags, size, mtime, name
                # files: fid => full path (last arg); use file_basename($fid)
  ATTRIBUTE   => key, value
                # attributes: key => value; last write wins (do not invent values)
  OPTION      => key, value
                # options: key => value; last write wins (do not invent values)
  PID_START   => pid, ppid?, start_time?
                # pid_starts: list of { pid, ppid?, start_time? }; counts via pid_start_count
  PID_END     => pid, end_time?
                # pid_ends: list of { pid, end_time? }; counts via pid_end_count
  DISCOUNT    => (empty args)
                # discount_events / discount_count: one increment per DISCOUNT tag (A3)
  SUB_ENTRY   => caller_fid, caller_line
                # sub_entry_events / sub_entry_count: one increment per SUB_ENTRY tag
  SUB_RETURN  => ... (also sub_return_events: one per successfully ingested tag)
  SUB_CALLERS => ... (also sub_callers_events: one per successfully ingested tag)
  NEW_FID     => ... (also new_fid_events: one per successfully ingested tag)
  SRC_LINE    => ... (also src_line_events: one per successfully ingested tag)
  SUB_INFO    => ... (also sub_info_events: one per successfully ingested tag)

Call-edge keys sum C<count> across all C<SUB_CALLERS> sites for the same
C<(caller, callee)> pair. Line totals count one call per timing event and sum
ticks for each C<(fid, line)> from both C<TIME_LINE> and C<TIME_BLOCK>.
Block-line totals (A4b) count one call per C<TIME_BLOCK> and sum ticks for
each C<(fid, block_line)>; empty when no C<TIME_BLOCK> is present.
Source text is stored exactly as the dump emits it (including trailing newline).
ATTRIBUTE and OPTION values are stored exactly as emitted (typically JSON
strings such as C<"10000000"> or C<"1">).
PID values are dump-derived only — never invent process IDs.
C<DISCOUNT> is counted as event multiplicity only (default-calls1 golden
observes B<818>); this is not exclusive-time policy freeze.
C<SUB_ENTRY> is counted as event multiplicity only (default-calls1
B<0> with C<calls=1>; calls2-default B<27> with C<calls=2>); this is not
full call-stack / arg freeze.
JSON-EVENT-COUNTS-MVP stream multiplicities on default-calls1 golden:
C<sub_return_events> B<27>, C<new_fid_events> B<3>, C<sub_callers_events>
B<13>, C<src_line_events> B<632>, C<sub_info_events> B<31> (match independent
stream re-count of those tags).

=head1 SEE ALSO

F<docs/schemas/perl-jsonl-data-mvp-v0.md>,
F<docs/schemas/perl-jsonl-readstream-mvp-v0.md>,
F<docs/schemas/aggregate-comparison-v0.md>,
F<docs/contracts/COMPAT-010_INCOMPLETE_STREAM.md>,
L<Devel::NYTProf::JsonlReadStream>

=cut
