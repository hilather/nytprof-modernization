# SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
#
# PR-G03a — Product debugger entry for `perl -d:NYTProfM` (load holds
# an in-memory v5 sink; no nytprof.out on trivial -e).
# PR-G03b — DB::enable_sink / emit_time_line / emit_time_block /
# emit_discount / finish_profiler / run_m4_mini_sample call shipped
# nytp_emit_* (single writer).
# PR-G03c — DB::emit_sub_entry / emit_sub_return call nytp_emit_sub_*
# (same held sink).
# PR-G03d — DB::emit_attribute / emit_option / emit_new_fid / emit_src_line /
# emit_sub_info / emit_pid_start / emit_pid_end call nytp_emit_* (same
# held sink).
# PR-G03e — DB::emit_start_deflate / is_deflating call nytp_emit_start_deflate
# / nytp_v5_sink_is_deflating (same held sink). Mid-deflate fork residual.
# PR-G04  — When NYTPROF contains file=<path>, enable the file sink, set
# $^P 0x01 (DB::sub) and 0x02 (DB::DB) so live calls emit SUB_RETURN +
# SUB_CALLERS and statements emit TIME_LINE through shipped nytp_emit_*.
# Default (no file=) stays in-memory — G03a trivial -e writes no nytprof.out.
# PR-G05  — Parse all NYTPROF keys: unknown and format=dual fail-closed;
# format=v6 fail-closed on D1-B (v6_collect message); D1-A enable_sink_v6.
# PR-G06  — addpid=1 installs CORE::GLOBAL::fork → nytp_fork_* + addpid
# child reinit (`<file>.<pid>`). Mid-deflate continue-in-child residual.
# PR-B1 / DI-01 — NYTPROF blocks=1 live TIME_BLOCK + first-seen fid table
# + visit_contexts block/sub lines (not full opcode / not DI-03 / not
# slowops PRINT-MATCH). Default blocks=0 stays TIME_LINE (G04).
# PR-3    — savesrc default 1: PL_perldb SAVESRC|SAVESRC_NOSUBS via XS
# (not $^P |= 0x400). finish_profiler emits SRC_LINE + SUB_INFO.
# PR-7    — Compile-safe start: do not set $DB::single at file= enable.
# INIT sets it so use/BEGIN compile without DB::DB. DB::sub goto-all
# until INIT; after, goto &$raw for Exporter / Getopt / vars / constant
# / overload (not &$raw wrap). Workload subs keep the hash-stack wrap.
# Shape follows 6.15 Devel::NYTProf.pm: package Devel::NYTProfM then DB,
# require Core, init_profiler().

package Devel::NYTProfM;

our $VERSION = '6.15';    # match baseline/6.15 pin; keep in sync with Devel::NYTProfM::Core

package    # hide the package from the PAUSE indexer
    DB;

# Enable specific perl debugger flags (others may be set later).
# Set the flags that influence compilation ASAP so we get full details
# (sub line ranges etc) of modules loaded as a side effect of loading
# Devel::NYTProfM::Core (ie XSLoader, strict, Exporter etc.)
# See "perldoc perlvar" for details of the $^P ($PERLDB) flags.
# 0x01 is sub enter/exit (DB::sub), *not* single-step (that is 0x20) and
# *not* line-by-line (0x02 / DB::DB). G03a–G03e omit 0x01; G04 sets it
# only when NYTPROF file= is present.
$^P = 0x010     # record line range of sub definition
    | 0x100     # informative "file" names for evals
    | 0x200;    # informative names for anonymous subroutines

require Devel::NYTProfM::Core;    # loads XS and provides DB::init_profiler

# Greppable load stamp after successful Core/XS load.
# PRODUCT_XS_ATTACH is 0 until NYTPROF file= enables the live DB::sub hook.
# PRODUCT_STMT_EMIT marks G03b nytp_emit_* wrappers (not opcode TIME_*).
# PRODUCT_SUB_EMIT marks G03c nytp_emit_sub_* wrappers.
# PRODUCT_META_EMIT marks G03d nytp_emit_* meta/finalize wrappers.
# PRODUCT_COMPRESS_EMIT marks G03e nytp_emit_start_deflate.
$Devel::NYTProfM::PRODUCT_XS_LOAD       = 1;
$Devel::NYTProfM::PRODUCT_XS_ATTACH     = 0;
$Devel::NYTProfM::PRODUCT_STMT_EMIT     = 1;
$Devel::NYTProfM::PRODUCT_SUB_EMIT      = 1;
$Devel::NYTProfM::PRODUCT_META_EMIT     = 1;
$Devel::NYTProfM::PRODUCT_COMPRESS_EMIT = 1;
$Devel::NYTProfM::PRODUCT_COMPRESS      = 0;
$Devel::NYTProfM::PRODUCT_V6_COLLECT    = DB::product_v6_collect() ? 1 : 0;
$Devel::NYTProfM::PRODUCT_OPTIONS_PARSE = 1;
$Devel::NYTProfM::PRODUCT_ADDPID        = 0;
$Devel::NYTProfM::PRODUCT_FORK_HOOK     = 0;
# DI-01 stamps (6.15 defaults): blocks=0, calls=1, slowops=2.
$Devel::NYTProfM::PRODUCT_BLOCKS        = 0;
$Devel::NYTProfM::PRODUCT_CALLS         = 1;
$Devel::NYTProfM::PRODUCT_SLOWOPS       = 2;
$Devel::NYTProfM::PRODUCT_STMT_OPS      = 0;
$Devel::NYTProfM::PRODUCT_SLOWOPS_OPS   = 0;
$Devel::NYTProfM::PRODUCT_REQUIRE_REBIND = 0;
$Devel::NYTProfM::PRODUCT_SIGEXIT        = '';
$Devel::NYTProfM::PRODUCT_SIGEXIT_DONE   = 0;
# PR-3: 6.15 NYTP_OPTf_SAVESRC default on. set_savesrc applies PL_perldb.
$Devel::NYTProfM::PRODUCT_SAVESRC        = 1;

our @product_sub_stack;
our $product_in_hook = 0;
# 0 until INIT after file= enable. Compile-time DB::sub must goto so
# caller-sensitive pragmas (vars/constant/overload/Exporter) see the
# real caller. Set together with $DB::single in INIT.
our $product_after_init = 0;

# G04 / DI-01 statement hook (enabled only with NYTPROF file= + $^P 0x02).
# Default (blocks=0): TIME_LINE via shipped nytp_emit_time_line.
# blocks=1: TIME_BLOCK via fid_for_filename + block_and_sub_lines, unless
# a targeted DBSTATE/NEXTSTATE slice (PRODUCT_STMT_OPS) already emits.
# Not full 6.15 opcode / DI-03.
sub DB {
    return unless $Devel::NYTProfM::PRODUCT_XS_ATTACH;
    return if $product_in_hook;
    return if $Devel::NYTProfM::PRODUCT_STMT_OPS;
    my ( undef, $file, $line ) = caller;
    $product_in_hook = 1;
    eval {
        my $fid = DB::fid_for_filename($file);
        if ($Devel::NYTProfM::PRODUCT_BLOCKS) {
            my ( $bl, $sl ) = DB::block_and_sub_lines();
            DB::emit_time_block( 1, $fid, $line || 1, $bl || $line, $sl || $line );
        }
        else {
            DB::emit_attributed_time_line( $fid, $line || 1 );
        }
        1;
    };
    $product_in_hook = 0;
}

# 6.15 options[] + string options + product format=. Unknown keys croak.
my %PRODUCT_NYTPROF_KNOWN = map { $_ => 1 } qw(
  file format start end compress stmts blocks subs calls leave slowops
  usecputime clock trace findcaller forkdepth addpid nameevals nameanonsubs
  evals sigexit posix_exit perldb use_db_sub expand log optimize optimise
  savesrc endatexit libcexit addtimestamp
);

# Parse NYTPROF the way 6.15 Core.pm does: colon-separated, backslash-escapes.
# Empty / absent file= → no product profile file (G03a: no nytprof.out default).
sub _product_parse_nytprof {
    my %opts;
    my $env = $ENV{NYTPROF};
    return \%opts unless defined $env && length $env;
    for my $optval ( $env =~ /((?:[^\\:]+|\\.)+)/g ) {
        $optval =~ s/\\(.)/$1/g;
        my ( $opt, $val ) = split /=/, $optval, 2;
        if ( !defined $opt || $opt eq '' || !defined $val ) {
            die "malformed NYTPROF option: $optval\n";
        }
        if ( !$PRODUCT_NYTPROF_KNOWN{$opt} ) {
            die "unknown NYTPROF option: $opt\n";
        }
        $opts{$opt} = $val;
    }
    return \%opts;
}

sub _product_int_opt {
    my ( $opts, $key, $default ) = @_;
    return $default unless exists $opts->{$key};
    my $val = $opts->{$key};
    if ( !defined $val || $val !~ /^-?\d+$/ ) {
        die "unknown NYTPROF option: $key\n";
    }
    return 0 + $val;
}

sub _product_nytprof_file {
    my $opts = _product_parse_nytprof();
    my $p    = $opts->{file};
    return $p if defined $p && length $p;
    return;
}

sub _product_skip_sub {
    my ($name) = @_;
    return 1 unless defined $name && length $name;
    return 1 if ref $name;
    return 1 if $name =~ /^(?:DB::|Devel::NYTProfM::|CORE::GLOBAL::fork)/;
    return 1 if $name =~ /::__ANON__\b/;
    return 0;
}

# Tail-call / caller-sensitive modules must stay `goto &$raw`.
# Wrapping with `&$raw` then return makes Exporter's `goto &heavy_*`
# become `heavy_(eval)`, and vars/constant/overload import's caller()
# see DB so `use vars qw($VERSION)` / `use constant` fail under strict.
# Compile-time calls also goto (`$product_after_init` is 0 until INIT).
# Never pair this with a pushed stack frame — goto unwinds the pad.
sub _product_needs_goto {
    my ($name) = @_;
    return 0 unless defined $name && length $name;
    return 0 if ref $name;
    return 1
      if $name =~
      /^(?:Exporter(?:::|\z)|Getopt::|vars::|constant::|overload::)/;
    return 0;
}

# Smallest G04 hook: Perl DB::sub + $^P 0x01. Emits SUB_RETURN (A5) and
# SUB_CALLERS (A7) through shipped DB::emit_* → nytp_emit_*. Not 6.15
# entersub / opcode TIME_* / XSUB / goto.
# $DB::sub is a CV ref for BEGIN (and some evals); resolve to
# Package::BEGIN@line like 6.15.
sub sub {
    my $raw = $DB::sub;
    my $called = $raw;
    if ( ref $raw ) {
        $called = DB::name_cv($raw);
        if ( defined $called && $called =~ /::BEGIN$/ && $called !~ /@/ ) {
            my ( undef, undef, $bline ) = caller(0);
            $called .= '@' . ( $bline || 0 );
        }
    }
    if (  !$Devel::NYTProfM::PRODUCT_XS_ATTACH
        || $product_in_hook
        || !$product_after_init
        || _product_skip_sub($called)
        || _product_needs_goto($called) )
    {
        goto &$raw;
    }

    my $caller =
      @product_sub_stack
      ? $product_sub_stack[-1]{name}
      : 'main::RUNTIME';
    my ( undef, $cfile, $cline ) = caller(0);
    $cline ||= 1;
    my $cfid = 1;
    if ( defined $cfile && length $cfile ) {
        $product_in_hook = 1;
        eval { $cfid = DB::fid_for_filename($cfile) || 1; 1 };
        $product_in_hook = 0;
    }

    # Absorb MATCH/PRINT exclusive that ran in the parent since last child.
    if (@product_sub_stack) {
        $product_sub_stack[-1]{child_excl} += DB::take_pending_child_excl();
    }
    else {
        DB::take_pending_child_excl();
    }

    push @product_sub_stack,
      {
        name       => $called,
        t0         => DB::clock_now_ticks(),
        child_excl => 0,
        fid        => $cfid,
        line       => $cline,
      };

    if ( $Devel::NYTProfM::PRODUCT_CALLS >= 2 ) {
        $product_in_hook = 1;
        eval {
            DB::emit_sub_entry( $cfid, $cline );
            1;
        };
        $product_in_hook = 0;
    }

    my $wa = wantarray;
    my ( @ret, $scalar, $ok );
    $ok = eval {
        if ($wa) {
            @ret = &$raw;
        }
        elsif ( defined $wa ) {
            $scalar = &$raw;
        }
        else {
            &$raw;
        }
        1;
    };
    my $err = $@;
    my $frame = pop @product_sub_stack;
    my $depth = @product_sub_stack + 1;
    my $incl  = 0;
    my $excl  = 0;
    my $site_fid  = 1;
    my $site_line = 1;
    if ($frame) {
        $incl = DB::clock_now_ticks() - $frame->{t0};
        $incl = 0 if $incl < 0;
        $frame->{child_excl} += DB::take_pending_child_excl();
        $excl = $incl - ( $frame->{child_excl} || 0 );
        $excl = 0 if $excl < 0;
        if (@product_sub_stack) {
            $product_sub_stack[-1]{child_excl} += $excl;
        }
        $site_fid  = $frame->{fid}  || 1;
        $site_line = $frame->{line} || 1;
    }

    $product_in_hook = 1;
    eval {
        DB::emit_sub_return( $depth, $incl, $excl, $called );
        DB::emit_sub_callers( $site_fid, $site_line, 1, $incl, $excl, 0.0, 0,
            $called, $caller );
        1;
    };
    $product_in_hook = 0;

    die $err if !$ok;
    return @ret    if $wa;
    return $scalar if defined $wa;
    return;
}

# G06: smallest live fork hook — CORE::GLOBAL::fork around shipped
# nytp_fork_prepare / resume_parent / resume_child + addpid reinit.
sub _product_fork {
    unless (  $Devel::NYTProfM::PRODUCT_XS_ATTACH
        && $Devel::NYTProfM::PRODUCT_ADDPID )
    {
        return CORE::fork();
    }
    local $product_in_hook = 1;
    my $st = DB::fork_prepare();
    die "DB::fork_prepare status=$st\n" if $st != 0;
    my $pid = CORE::fork();
    if ( !defined $pid ) {
        DB::fork_resume_parent();
        return undef;
    }
    if ($pid) {
        $st = DB::fork_resume_parent();
        die "DB::fork_resume_parent status=$st\n" if $st != 0;
        return $pid;
    }
    $st = DB::fork_resume_child($$);
    die "DB::fork_resume_child status=$st\n" if $st != 0;
    return 0;
}

sub _product_install_require_rebind {
    return if $Devel::NYTProfM::PRODUCT_REQUIRE_REBIND;
    $Devel::NYTProfM::PRODUCT_REQUIRE_REBIND = 1;
    # After warnings.pm compiles, rebind MATCH/PRINT op_ppaddr (ck_match
    # may not keep PL_ppaddr[OP_MATCH]). Then import() sees the hook.
    *CORE::GLOBAL::require = sub {
        my ($f) = @_;
        my $ok = CORE::require($f);
        if (  $Devel::NYTProfM::PRODUCT_SLOWOPS_OPS
            && defined $f
            && $f =~ /(?:^|[\/\\])warnings\.pm\z/ )
        {
            DB::rebind_stash_slowops('warnings');
        }
        return $ok;
    };
}

sub _product_sigexit_signals {
    my ($opts) = @_;
    my $v = $opts->{sigexit};
    return () unless defined $v && length $v && $v ne '0';
    if ( $v eq '1' ) {
        return qw(INT TERM HUP PIPE);
    }
    return map { uc $_ } grep { length } split /,/, $v;
}

sub _product_sigexit_handler {
    return if $Devel::NYTProfM::PRODUCT_SIGEXIT_DONE;
    $Devel::NYTProfM::PRODUCT_SIGEXIT_DONE = 1;
    eval { finish_profiler(); 1 };
    exit 1;
}

sub _product_install_sigexit {
    my (@sigs) = @_;
    return unless @sigs;
    # POSIX::_exit / raw SYS_exit residual: END does not run; no flush.
    for my $s (@sigs) {
        $SIG{$s} = \&_product_sigexit_handler;
    }
    $Devel::NYTProfM::PRODUCT_SIGEXIT = join ',', @sigs;
}

sub _product_install_fork_hook {
    return if $Devel::NYTProfM::PRODUCT_FORK_HOOK;
    # Do not `no warnings` here: compiling that loads warnings.pm before
    # XS BOOT can redirect OP_MATCH (needed for warnings::CORE:match).
    local $^W = 0;
    *CORE::GLOBAL::fork = \&_product_fork;
    $Devel::NYTProfM::PRODUCT_FORK_HOOK = 1;
}

# G05: parse NYTPROF before opening a file so unknown / dual / D1-B
# format=v6 croak without writing a profile.
{
    my $opts = _product_parse_nytprof();
    my $fmt  = exists $opts->{format} ? lc( $opts->{format} ) : 'v5';
    if ( $fmt eq 'dual' ) {
        die "format=dual is rejected\n";
    }
    if ( $fmt eq 'v6' && !$Devel::NYTProfM::PRODUCT_V6_COLLECT ) {
        die "format=v6 requires v6-enabled build "
          . "(install v6_collect package or rebuild with --with v6_collect)\n";
    }
    if ( $fmt ne 'v5' && $fmt ne 'v6' ) {
        die "unknown NYTPROF option: format\n"
          unless exists $opts->{format};
        die "unknown format=$opts->{format} (want v5|v6)\n";
    }
    $Devel::NYTProfM::PRODUCT_FORMAT = $fmt;
    my $addpid = $opts->{addpid};
    $Devel::NYTProfM::PRODUCT_ADDPID =
      ( defined $addpid && $addpid ne '' && $addpid ne '0' ) ? 1 : 0;

    # DI-01: stamp blocks/calls/slowops. Honor blocks=1 for TIME_BLOCK.
    # slowops=2 is the 6.15 default — do not fail-closed. slowops=1 is
    # residual until PR-B2 / full opcode. PRINT/MATCH install is PR-B2.
    my $blocks = _product_int_opt( $opts, 'blocks', 0 );
    $Devel::NYTProfM::PRODUCT_BLOCKS = $blocks ? 1 : 0;
    my $calls = _product_int_opt( $opts, 'calls', 1 );
    if ( $calls < 0 || $calls > 2 ) {
        die "unknown NYTPROF option: calls\n";
    }
    $Devel::NYTProfM::PRODUCT_CALLS = $calls;
    my $slowops = _product_int_opt( $opts, 'slowops', 2 );
    if ( $slowops == 1 ) {
        die "slowops=1 (collapsed CORE:: package) is residual until full "
          . "opcode attach; use default/slowops=2 (PRINT/MATCH subset) or "
          . "slowops=0\n";
    }
    if ( $slowops != 0 && $slowops != 2 ) {
        die "unknown NYTPROF option: slowops\n";
    }
    $Devel::NYTProfM::PRODUCT_SLOWOPS = $slowops;
    my $compress = _product_int_opt( $opts, 'compress', 0 );
    $Devel::NYTProfM::PRODUCT_COMPRESS = $compress ? 1 : 0;
    my $usecpu = $opts->{usecputime};
    if ( defined $usecpu && $usecpu ne '' && $usecpu ne '0' ) {
        warn
"The NYTProf usecputime option has been removed (try using clock=N if possible)\n";
    }
    my $savesrc = _product_int_opt( $opts, 'savesrc', 1 );
    $Devel::NYTProfM::PRODUCT_SAVESRC = $savesrc ? 1 : 0;
}

init_profiler();    # G03a: hold in-memory v5 sink — never writes nytprof.out

# G04: explicit NYTPROF file= switches the held sink to a real file and
# enables DB::sub collection. Absent file= keeps G03a fileless default.
# G05: format=v6 + D1-A uses enable_sink_v6 (NYTPROF6); format=v5 stays v5.
{
    my $path = _product_nytprof_file();
    if ( defined $path && length $path ) {
        my $fmt = $Devel::NYTProfM::PRODUCT_FORMAT || 'v5';
        my $st;
        if ( $fmt eq 'v6' ) {
            $st = enable_sink_v6($path);
            if ( $st != 0 ) {
                die "DB::enable_sink_v6($path) status=$st\n";
            }
        }
        else {
            $st = enable_sink($path);
            if ( $st != 0 ) {
                die "DB::enable_sink($path) status=$st\n";
            }
            if ( $Devel::NYTProfM::PRODUCT_COMPRESS ) {
                $st = emit_start_deflate();
                if ( $st != 0 ) {
                    die "DB::emit_start_deflate status=$st\n";
                }
            }
        }
        $Devel::NYTProfM::PRODUCT_XS_ATTACH = 1;
        $^P |= 0x01;    # sub enter/exit → DB::sub
        $^P |= 0x02;    # line-by-line (dbstate already compiled when $^P != 0)
        $^P |= 0x20;    # start with single-step on
        # PR-7: do not set $DB::single here. pp_dbstate would run DB::DB
        # while compiling use/BEGIN (Getopt::Long $VERSION under strict).
        # INIT below turns statement hooks on after compile.
        # 6.15 ~3177–3179: PL_perldb |= PERLDBf_SAVESRC | PERLDBf_SAVESRC_NOSUBS
        # via XS macros — not $^P |= 0x400 (eval-source / SAVESRC_INVALID).
        DB::set_savesrc( $Devel::NYTProfM::PRODUCT_SAVESRC );
        if ( $Devel::NYTProfM::PRODUCT_BLOCKS ) {
            # DBSTATE/NEXTSTATE/UNSTACK TIME_BLOCK slice — not DI-03 opcode.
            my $st_ops = DB::install_product_stmt_ops();
            if ( $st_ops == 0 ) {
                $Devel::NYTProfM::PRODUCT_STMT_OPS = 1;
            }
        }
        if (  $Devel::NYTProfM::PRODUCT_SLOWOPS == 2
            && $Devel::NYTProfM::PRODUCT_CALLS >= 1 )
        {
            # Thin PRINT/MATCH only (KD-35). Not full slowops.h / DI-03.
            my $st_so = DB::install_product_slowops();
            if ( $st_so == 0 ) {
                $Devel::NYTProfM::PRODUCT_SLOWOPS_OPS = 1;
            }
            # Compile warnings.pm after PL_ppaddr redirect, then rebind
            # any MATCH ops that kept a specialized op_ppaddr.
            require warnings;
            DB::rebind_stash_slowops('warnings');
            _product_install_require_rebind();
        }
        if ( $Devel::NYTProfM::PRODUCT_ADDPID ) {
            _product_install_fork_hook();
        }
        my @sigexit = _product_sigexit_signals( _product_parse_nytprof() );
        if (@sigexit) {
            _product_install_sigexit(@sigexit);
        }
    }
}

# PR-7: statement hooks after compile. $DB::single at file= enable
# would run DB::DB during use/BEGIN (Getopt::Long $VERSION / Exporter).
# $^P 0x01|0x02|0x20 stay on from enable; only this bit is deferred.
INIT {
    if ($Devel::NYTProfM::PRODUCT_XS_ATTACH) {
        $DB::single           = 1;
        $product_after_init   = 1;
    }
}

END {
    # Close the held sink (file or in-memory). Second call is a no-op.
    finish_profiler();
}

1;
