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
# PR-G04 / DI-03 E1b — When NYTPROF contains file=<path>, enable the
# file sink. Omit entersub installs OP_ENTERSUB (emit after INIT) and
# leaves $^P 0x01 off; wrap=1 / use_db_sub=1 / entersub=0 set 0x01
# (DB::sub wrap_push). $^P 0x02 stays on for OP_DBSTATE TIME_LINE via
# shipped nytp_emit_*. Default (no file=) stays in-memory — G03a
# trivial -e writes no nytprof.out.
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
# INIT chooses C OP_DBSTATE ($DB::single=0) or Perl DB::DB fallback.
# DB::sub goto-all
# until INIT; after, goto &$raw for Exporter / Getopt / vars / constant
# / overload / any ::import / Moo / Moose / Class:: / Rex:: / DateTime::
# / Memoize:: (not &$raw wrap). Workload wrap must not `eval { &$raw }`:
# that eval frame is visible to caller() (loggers report NYTProfM.pm).
# Perl already skips package-DB sub frames. DESTROY still emits on die.
# PR-10   — Do not wrap CORE::require (no CORE::GLOBAL::require).
# Preload B::Hooks::EndOfScope / Variable::Magic / namespace::* and
# CvNODEBUG their CVs before $^P 0x01. DB::sub during on_scope_end
# (even goto) breaks %^H so DateTime::Duration dies: Can't use string
# ("#pod\n") as an ARRAY ref at B/Hooks/EndOfScope/XS.pm line 39.
# Do not defer 0x01 to INIT (subs compiled without PERLDBf_SUB never
# call DB::sub).
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
# 0x01 is sub enter/exit (DB::sub wrap escape), *not* single-step
# (0x20) and *not* line-by-line (0x02 / DB::DB). G03a–G03e omit 0x01.
# E1b default file= leaves 0x01 off (opcode ENTERSUB). wrap=1 /
# use_db_sub=1 / entersub=0 set 0x01.
$^P = 0x010     # record line range of sub definition
    | 0x100     # informative "file" names for evals
    | 0x200;    # informative names for anonymous subroutines

require Devel::NYTProfM::Core;    # loads XS and provides DB::init_profiler

# Greppable load stamp after successful Core/XS load.
# PRODUCT_XS_ATTACH is 0 until NYTPROF file= enables live attach.
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
# Omitted NYTPROF compress ⇒ zlib level 6 (6.15). Stamp before parse is
# overwritten in _product_apply_options. compress=0 remains the opt-out.
$Devel::NYTProfM::PRODUCT_COMPRESS        = 1;
$Devel::NYTProfM::PRODUCT_COMPRESS_LEVEL  = 6;
$Devel::NYTProfM::PRODUCT_DURABLE         = 0;
$Devel::NYTProfM::PRODUCT_V6_COLLECT    = DB::product_v6_collect() ? 1 : 0;
$Devel::NYTProfM::PRODUCT_OPTIONS_PARSE = 1;
$Devel::NYTProfM::PRODUCT_ADDPID        = 0;
$Devel::NYTProfM::PRODUCT_FORK_HOOK     = 0;
# DI-01 stamps (6.15 defaults): blocks=0, calls=1, slowops=2.
$Devel::NYTProfM::PRODUCT_BLOCKS        = 0;
$Devel::NYTProfM::PRODUCT_CALLS         = 1;
$Devel::NYTProfM::PRODUCT_SLOWOPS       = 2;
$Devel::NYTProfM::PRODUCT_STMTS         = 1;
$Devel::NYTProfM::PRODUCT_STMT_OPS      = 0;
$Devel::NYTProfM::PRODUCT_DBSTATE_LINE  = 0;
$Devel::NYTProfM::PRODUCT_SLOWOPS_OPS   = 0;
# DI-03 E1b: omit entersub ⇒ opcode ON. wrap=1 / use_db_sub=1 is the
# wrap escape (KD-E11; not 6.15 stmt DB::DB). entersub=0 forces wrap.
# wrap=1 wins over entersub=1.
$Devel::NYTProfM::PRODUCT_USE_DB_SUB    = 0;
$Devel::NYTProfM::PRODUCT_WRAP          = 0;
$Devel::NYTProfM::PRODUCT_ENTERSUB      = 0;
# 1 only after file= actually installed OP_ENTERSUB (not wrap=1 win).
$Devel::NYTProfM::PRODUCT_ENTERSUB_OPS  = 0;
# Bench control only (not an NYTPROF colon option). 1 = old Perl
# caller(0)+fid XSUB wrap so g16 can measure the C site crossing.
$Devel::NYTProfM::PRODUCT_WRAP_SLOW =
  (  defined $ENV{NYTPROF_WRAP_SLOW}
  && $ENV{NYTPROF_WRAP_SLOW} ne ''
  && $ENV{NYTPROF_WRAP_SLOW} ne '0' ) ? 1 : 0;
$Devel::NYTProfM::PRODUCT_SIGEXIT        = '';
$Devel::NYTProfM::PRODUCT_SIGEXIT_DONE   = 0;
# PR-3: 6.15 NYTP_OPTf_SAVESRC default on. set_savesrc applies PL_perldb.
$Devel::NYTProfM::PRODUCT_SAVESRC        = 1;

our @product_sub_stack;
our $product_in_hook = 0;
# 0 until INIT after file= enable. Compile-time DB::sub must goto so
# caller-sensitive pragmas (vars/constant/overload/Exporter) see the
# real caller. Set in INIT with the statement-hook choice ($DB::single
# or C OP_DBSTATE TIME_LINE).
our $product_after_init = 0;

# G04 / DI-01 statement hook (enabled only with NYTPROF file= + $^P 0x02).
# Default (blocks=0, PR-15): C OP_DBSTATE emits TIME_LINE and INIT leaves
# $DB::single=0, so this Perl DB::DB is not entered. Fallback if the C
# hook is not installed: TIME_LINE via shipped nytp_emit_time_line.
# blocks=1: TIME_BLOCK via fid_for_filename + block_and_sub_lines, unless
# a targeted DBSTATE/NEXTSTATE slice (PRODUCT_STMT_OPS) already emits.
# Not full 6.15 opcode / DI-03.
sub DB {
    return unless $Devel::NYTProfM::PRODUCT_XS_ATTACH;
    return unless $Devel::NYTProfM::PRODUCT_STMTS;
    return if $product_in_hook;
    return if $Devel::NYTProfM::PRODUCT_STMT_OPS;
    return if $Devel::NYTProfM::PRODUCT_DBSTATE_LINE;
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
  durable aggregate
  usecputime clock trace findcaller forkdepth addpid nameevals nameanonsubs
  evals sigexit posix_exit perldb use_db_sub expand log optimize optimise
  savesrc endatexit libcexit addtimestamp
  wrap entersub
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
    return 1
      if $name =~
      /^(?:DB::|Devel::NYTProfM::|CORE::GLOBAL::(?:fork|require))/;
    return 1 if $name =~ /::__ANON__\b/;
    return 0;
}

# Tail-call / caller-sensitive modules must stay `goto &$raw`.
# Wrapping with `&$raw` then return makes Exporter's `goto &heavy_*`
# become `heavy_(eval)`, and vars/constant/overload import's caller()
# see DB so `use vars qw($VERSION)` / `use constant` fail under strict.
# Compile-time calls also goto (`$product_after_init` is 0 until INIT).
# Never pair this with a pushed stack frame — goto unwinds the pad.
our %product_sub_disp;    # name => 0 wrap / 1 skip / 2 goto

sub _product_sub_disp {
    my ($name) = @_;
    return 1 unless defined $name && length $name;
    return 1 if ref $name;
    my $hit = $product_sub_disp{$name};
    return $hit if defined $hit;
    my $d = 0;
    if ( _product_skip_sub($name) ) {
        $d = 1;
    }
    elsif ( _product_needs_goto($name) ) {
        $d = 2;
    }
    $product_sub_disp{$name} = $d;
    return $d;
}

sub _product_needs_goto {
    my ($name) = @_;
    return 0 unless defined $name && length $name;
    return 0 if ref $name;
    # Inherited Exporter::import is named Child::import in $DB::sub
    # (Rex::Shared::Var::import). Wrapping it exports into DB, so
    # `share qw(@SUMMARY)` in Rex::TaskList::Base is a syntax error
    # and ->new is missing. Same for any use/import that uses caller.
    return 1 if $name =~ /::(?:import|unimport)\z/;
    return 1
      if $name =~
      /^(?:Exporter(?:::|\z)|Getopt::|vars::|constant::|overload::)/;
    # XSLoader::load() with no args uses caller() as the module name.
    # Wrapping it looks for DB.so: "Can't locate loadable object for
    # module DB" (Rex::Shared::Var::Common `use Fcntl` / Storable).
    return 1
      if $name =~
      /^(?:XSLoader(?:::|\z)|DynaLoader(?:::|\z))/;
    return 1
      if $name =~
      /^(?:Moo(?:::|\z)|Moose(?:::|\z)|Class::|Rex(?:::|\z)|DateTime(?:::|\z))/;
    # Memoize::memoize does `my $uppack = caller` then looks up
    # $uppack::$fn. Wrap ⇒ caller is DB ⇒
    # "Cannot operate on nonexistent function `foo'" for a sub that
    # exists in the real package (works without -d:NYTProfM).
    return 1 if $name =~ /^(?:Memoize(?:::|\z))/;
    return 0;
}

# Wrap escape (wrap=1 / use_db_sub=1 / entersub=0): C wrap_push/wrap_pop
# → nytp_emit_* (PR-16). Default opcode stubs this so wrap and
# OP_ENTERSUB never emit the same call.
# $DB::sub is a CV ref for BEGIN (and some evals); resolve to
# Package::BEGIN@line like 6.15.
sub sub {
    my $raw = $DB::sub;
    if ($Devel::NYTProfM::PRODUCT_ENTERSUB_OPS) {
        die "NYTProfM: \$DB::sub is missing; cannot tail-call\n"
          unless defined $raw && ( ref($raw) || ( !ref($raw) && length $raw ) );
        goto &$raw;
    }
    my $called = $raw;
    if ( ref $raw ) {
        $called = DB::name_cv($raw);
        if ( defined $called && $called =~ /::BEGIN$/ && $called !~ /@/ ) {
            my ( undef, undef, $bline ) = caller(0);
            $called .= '@' . ( $bline || 0 );
        }
    }
    elsif ( defined $raw && !ref $raw ) {
        # Imported alias: Rexfile `task` may be "task" or "main::task"
        # while the CV lives in Rex::Commands. Resolve defining name
        # so _product_needs_goto sees Rex:: (not main::).
        my $cand = $raw =~ /::/ ? $raw : "main::$raw";
        if ( defined &$cand ) {
            my $real = eval { DB::name_cv( \&$cand ) };
            $called = $real if defined $real && length $real && $real =~ /::/;
        }
    }
    if (  !$Devel::NYTProfM::PRODUCT_XS_ATTACH
        || $product_in_hook
        || !$product_after_init
        || _product_sub_disp($called) )
    {
        die "NYTProfM: \$DB::sub is missing; cannot tail-call\n"
          unless defined $raw && ( ref($raw) || ( !ref($raw) && length $raw ) );
        goto &$raw;
    }

    if ($Devel::NYTProfM::PRODUCT_WRAP_SLOW) {
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
        if (@product_sub_stack) {
            $product_sub_stack[-1]{child_excl} +=
              DB::take_pending_child_excl();
        }
        else {
            DB::take_pending_child_excl();
        }
        push @product_sub_stack,
          {
            name       => $called,
            caller     => $caller,
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
    }
    else {
        DB::wrap_push($called);
    }

    die "NYTProfM: \$DB::sub is missing; cannot wrap\n"
      unless defined $raw && ( ref($raw) || ( !ref($raw) && length $raw ) );

    # Do not wrap the callee in eval. caller() skips package-DB sub
    # frames but not CXt_EVAL — loggers then report NYTProfM.pm.
    # DESTROY still emits SUB_RETURN if the callee dies.
    my $guard = bless { armed => 1 }, 'DB::ProductWrapGuard';
    my $wa    = wantarray;
    my ( @ret, $scalar );
    if ($wa) {
        @ret = &$raw;
    }
    elsif ( defined $wa ) {
        $scalar = &$raw;
    }
    else {
        &$raw;
    }
    $guard->{armed} = 0;
    _product_finish_current_frame();
    return @ret    if $wa;
    return $scalar if defined $wa;
    return;
}

sub _product_finish_current_frame {
    if ( !$Devel::NYTProfM::PRODUCT_WRAP_SLOW ) {
        DB::wrap_pop();
        return;
    }
    my $frame     = pop @product_sub_stack;
    my $depth     = @product_sub_stack + 1;
    my $incl      = 0;
    my $excl      = 0;
    my $site_fid  = 1;
    my $site_line = 1;
    my $called    = 'main::RUNTIME';
    my $caller    = 'main::RUNTIME';
    if ($frame) {
        $called = $frame->{name}   || $called;
        $caller = $frame->{caller} || $caller;
        $incl   = DB::clock_now_ticks() - $frame->{t0};
        $incl   = 0 if $incl < 0;
        $frame->{child_excl} += DB::take_pending_child_excl();
        $excl = $incl - ( $frame->{child_excl} || 0 );
        $excl = 0 if $excl < 0;
        # Exclusive = incl − Σ child *inclusive*. Crediting child excl
        # leaked grandchildren into the parent (lab_run excl ≈ YAML).
        if (@product_sub_stack) {
            $product_sub_stack[-1]{child_excl} += $incl;
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
}

{
    package    # hide from PAUSE
        DB::ProductWrapGuard;
    sub DESTROY {
        my ($g) = @_;
        return unless $g && $g->{armed};
        $g->{armed} = 0;
        eval { DB::_product_finish_current_frame(); 1 };
    }
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

# Preload compile-time %^H helpers and disable DB::sub on their CVs.
# Must run before $^P 0x01. Missing modules are skipped (host may not
# have DateTime / namespace::autoclean).
sub _product_nodebug_hint_magic {
    my @pkgs = qw(
      Variable::Magic
      B::Hooks::EndOfScope
      B::Hooks::EndOfScope::XS
      B::Hooks::EndOfScope::PP
      namespace::clean
      namespace::autoclean
      Package::Stash
      Package::Stash::XS
      Sub::Exporter::Progressive
    );
    for my $pkg (@pkgs) {
        eval "require $pkg; 1" or next;
        eval { DB::nodebug_stash($pkg); 1 };
    }
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
    my $stmts = _product_int_opt( $opts, 'stmts', 1 );
    $Devel::NYTProfM::PRODUCT_STMTS = $stmts ? 1 : 0;
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
    if ( exists $opts->{compress} ) {
        my $compress = _product_int_opt( $opts, 'compress', 0 );
        if ( $compress < 0 || $compress > 9 ) {
            die "unknown NYTPROF option: compress\n";
        }
        $Devel::NYTProfM::PRODUCT_COMPRESS_LEVEL = $compress;
        $Devel::NYTProfM::PRODUCT_COMPRESS       = $compress > 0 ? 1 : 0;
    }
    else {
        $Devel::NYTProfM::PRODUCT_COMPRESS_LEVEL = 6;
        $Devel::NYTProfM::PRODUCT_COMPRESS       = 1;
    }
    $Devel::NYTProfM::PRODUCT_DURABLE =
      _product_int_opt( $opts, 'durable', 0 ) ? 1 : 0;
    if ( _product_int_opt( $opts, 'aggregate', 0 ) ) {
        die "aggregate=1 is residual until ADR-0013 is accepted; "
          . "in-memory coalesced checkpoints are not implemented\n";
    }
    my $usecpu = $opts->{usecputime};
    if ( defined $usecpu && $usecpu ne '' && $usecpu ne '0' ) {
        warn
"The NYTProf usecputime option has been removed (try using clock=N if possible)\n";
    }
    my $savesrc = _product_int_opt( $opts, 'savesrc', 1 );
    $Devel::NYTProfM::PRODUCT_SAVESRC = $savesrc ? 1 : 0;
    # DI-03 E1b: omit entersub ⇒ opcode. wrap=1 / use_db_sub=1 stamp
    # wrap; entersub=0 forces wrap. wrap wins over entersub=1.
    my $use_db_sub = _product_int_opt( $opts, 'use_db_sub', 0 );
    if ( $use_db_sub != 0 && $use_db_sub != 1 ) {
        die "unknown NYTPROF option: use_db_sub\n";
    }
    my $wrap = _product_int_opt( $opts, 'wrap', 0 );
    if ( $wrap != 0 && $wrap != 1 ) {
        die "unknown NYTPROF option: wrap\n";
    }
    my $entersub = _product_int_opt( $opts, 'entersub', 1 );
    if ( $entersub != 0 && $entersub != 1 ) {
        die "unknown NYTPROF option: entersub\n";
    }
    $Devel::NYTProfM::PRODUCT_USE_DB_SUB = $use_db_sub ? 1 : 0;
    $Devel::NYTProfM::PRODUCT_ENTERSUB   = $entersub ? 1 : 0;
    $Devel::NYTProfM::PRODUCT_WRAP =
      ( $wrap || $use_db_sub || !$entersub ) ? 1 : 0;
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
            my $lvl = $Devel::NYTProfM::PRODUCT_COMPRESS
              ? $Devel::NYTProfM::PRODUCT_COMPRESS_LEVEL
              : 0;
            my $dur = $Devel::NYTProfM::PRODUCT_DURABLE ? 1 : 0;
            $st = enable_sink( $path, $lvl, $dur );
            if ( $st != 0 ) {
                die "DB::enable_sink($path) status=$st\n";
            }
            # durable=1 delays z until seal (D2). Until then, live z at enable.
            if ( $Devel::NYTProfM::PRODUCT_COMPRESS && !$dur ) {
                $st = emit_start_deflate();
                if ( $st != 0 ) {
                    die "DB::emit_start_deflate status=$st\n";
                }
            }
        }
        $Devel::NYTProfM::PRODUCT_XS_ATTACH = 1;
        # PR-10: preload %^H-magic modules and CvNODEBUG their CVs
        # *before* $^P 0x01. DB::sub (even `goto &$raw`) during
        # on_scope_end breaks Variable::Magic::getdata(%^H) so
        # DateTime::Duration dies: Can't use string ("#pod\n") as an
        # ARRAY ref at B/Hooks/EndOfScope/XS.pm line 39. Do not defer
        # 0x01 to INIT — subs compiled without PERLDBf_SUB never call
        # DB::sub (g04 15/3/15 goes to 0).
        _product_nodebug_hint_magic();
        # wrap=1 / use_db_sub=1 / entersub=0 wins (escape).
        # Omit entersub ⇒ opcode; $^P 0x01 stays off; DB::sub is stub.
        if ( $Devel::NYTProfM::PRODUCT_WRAP ) {
            $^P |= 0x01;    # sub enter/exit → DB::sub
        }
        elsif ( $Devel::NYTProfM::PRODUCT_ENTERSUB ) {
            my $st_es = DB::install_product_entersub();
            if ( $st_es != 0 ) {
                die "DB::install_product_entersub status=$st_es\n";
            }
            $Devel::NYTProfM::PRODUCT_ENTERSUB_OPS = 1;
            if ( $^P & 0x01 ) {
                die "NYTProfM: opcode entersub and wrap would both emit\n";
            }
        }
        else {
            $^P |= 0x01;    # defensive: treat unset as wrap
        }
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
        elsif ( $Devel::NYTProfM::PRODUCT_STMTS ) {
            # Install before the user script compiles so OP_DBSTATE
            # copies our ppaddr. Stay inactive until INIT.
            my $st_line = DB::install_product_dbstate_timeline();
            if ( $st_line == 0 ) {
                $Devel::NYTProfM::PRODUCT_DBSTATE_LINE = 1;
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
            # Do **not** install CORE::GLOBAL::require: wrapping
            # CORE::require in a Perl sub breaks compile-time %^H
            # magic (B::Hooks::EndOfScope::XS / namespace::autoclean /
            # DateTime::Duration — "Can't use string ("#pod\n") as an
            # ARRAY ref at .../EndOfScope/XS.pm line 39").
            require warnings;
            DB::rebind_stash_slowops('warnings');
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

# PR-7 / PR-15: statement emit after compile. $DB::single at file=
# enable would run DB::DB during use/BEGIN (Getopt::Long $VERSION).
# C OP_DBSTATE is installed at enable (so compile copies op_ppaddr)
# but stays inactive until INIT. Then $DB::single stays 0.
INIT {
    if ($Devel::NYTProfM::PRODUCT_XS_ATTACH) {
        $product_after_init = 1;
        if ( $Devel::NYTProfM::PRODUCT_ENTERSUB_OPS ) {
            if ( $^P & 0x01 ) {
                die "NYTProfM: opcode entersub and wrap would both emit\n";
            }
            DB::entersub_set_emit_enabled(1);
        }
        if ( $Devel::NYTProfM::PRODUCT_DBSTATE_LINE ) {
            my $st_on = DB::activate_product_dbstate_timeline();
            if ( $st_on != 0 ) {
                $Devel::NYTProfM::PRODUCT_DBSTATE_LINE = 0;
                $DB::single = 1;
            }
            else {
                $DB::single = 0;
            }
        }
        elsif ( $Devel::NYTProfM::PRODUCT_STMT_OPS ) {
            $DB::single = 0;
        }
        elsif ( $Devel::NYTProfM::PRODUCT_STMTS ) {
            $DB::single = 1;
        }
        else {
            $DB::single = 0;
        }
        if ($Devel::NYTProfM::PRODUCT_SLOWOPS_OPS) {
            DB::rebind_stash_slowops('warnings');
        }
    }
}

END {
    # Close the held sink (file or in-memory). Second call is a no-op.
    finish_profiler();
}

1;
