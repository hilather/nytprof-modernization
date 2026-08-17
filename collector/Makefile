# COL-001..007 + COL-014 + COL-015 collector overlay build (ADR-0004 B0-A).
# Compiles when a C toolchain is present. Not required for legacy-only packaging.
# COL-006 v5 wire writer links zlib (-lz).
# COL-007 v6 writer links zlib + zstd + lz4 for EVENT payload codecs (PR-B07).
# COL-014 dual-sink is test/dev-only (OQ-4) — not product UX.
# COL-015 fork/PID protocol + stress suite (buffered sinks).
#
# Usage (from repo root or collector/):
#   make -C collector
#   make -C collector test
#   make -C collector libnytp_sink_v5.a   # D1-B product archive (-lz only)
#   make -C collector probe-v5            # -lz-only link proof (not in make test)
#   make -C collector xs-bootstrap        # load-only CollectorBootstrap (not -d:NYTProf)
#   make -C collector xs-nytprof          # G03a–G05 D1-B debugger .so (-lz only)
#   make -C collector xs-nytprof-v6       # G05 D1-A debugger .so (-lz -lzstd -llz4)
#   make -C collector clean
#   make -C collector -j$(nproc) test   # parallel-safe via order-only | dirs
#
# Outputs stay under collector/build/ (gitignored). Never writes baseline/6.15/.
# Product D1-B link: libnytp_sink_v5.a + -lz only (no zstd/lz4).
# Full libnytp_sink.a remains the test/dev archive (make test / all).

CC      ?= cc
CFLAGS  ?= -std=c11 -Wall -Wextra -Wpedantic -O2 -g
CPPFLAGS += -Iinclude
LDFLAGS ?=
LDLIBS  ?= -lz -lzstd -llz4
LDLIBS_V5 := -lz
AR      ?= ar
PERL    ?= perl

BUILD   := build
LIB     := $(BUILD)/libnytp_sink.a
LIB_V5  := $(BUILD)/libnytp_sink_v5.a
TESTBINS := \
	$(BUILD)/test_sink_api \
	$(BUILD)/test_lifecycle_seq \
	$(BUILD)/test_fake_clock \
	$(BUILD)/test_batch_fast \
	$(BUILD)/test_v5_wire \
	$(BUILD)/test_v6_abs_wire \
	$(BUILD)/test_v6_codec_chunk_crc \
	$(BUILD)/test_v6_packing_footer \
	$(BUILD)/test_dual_sink \
	$(BUILD)/test_fork_pid

# PR-B09 E3-C fixture generator (not part of unit-test suite)
GEN_E3 := $(BUILD)/gen_e3_c_fixtures
# Default outdir when invoked as `make -C collector gen-e3-fixtures`
# (relative to collector/); override: make -C collector gen-e3-fixtures OUTDIR=...
OUTDIR ?= ../fixtures/v6/from-c

SRC := \
	src/nytp_sink.c \
	src/nytp_sink_counting.c \
	src/nytp_sink_v5.c \
	src/nytp_sink_v6.c \
	src/nytp_sink_dual.c \
	src/nytp_clock.c \
	src/nytp_batch.c \
	src/nytp_fork.c

OBJ := $(patsubst src/%.c,$(BUILD)/%.o,$(SRC))

# PR-G02 D1-B product archive: v5 writer + sink/batch/clock/fork/counting.
# Exclude nytp_sink_v6.o and nytp_sink_dual.o (those need zstd/lz4).
SRC_V5 := \
	src/nytp_sink.c \
	src/nytp_sink_counting.c \
	src/nytp_sink_v5.c \
	src/nytp_clock.c \
	src/nytp_batch.c \
	src/nytp_fork.c

# Separate PIC objects so the product archive/XS .so does not depend on
# distro default-PIE, and so v5 objects do not rebuild on v6/dual header edits.
PIC_DIR := $(BUILD)/pic
OBJ_V5 := $(patsubst src/%.c,$(PIC_DIR)/%.o,$(SRC_V5))

PROBE_V5 := $(BUILD)/probe_v5_product_link

# Load-only XS bootstrap (not Devel::NYTProf debugger / not product attach).
XS_NAME    := CollectorBootstrap
XS_PKG_DIR := Devel/NYTProf
XS_PM_SRC  := xs/$(XS_PKG_DIR)/$(XS_NAME).pm
XS_XS      := xs/$(XS_NAME).xs
XS_C       := $(BUILD)/$(XS_NAME).c
XS_O       := $(BUILD)/$(XS_NAME).o
XS_BOOT    := $(BUILD)/xs-bootstrap
XS_SO_DIR  := $(XS_BOOT)/auto/$(XS_PKG_DIR)/$(XS_NAME)
XS_SO      := $(XS_SO_DIR)/$(XS_NAME).so
XS_PM_DST  := $(XS_BOOT)/$(XS_PKG_DIR)/$(XS_NAME).pm

# G03a/G03b product debugger (perl -d:NYTProfM). Load holds in-memory sink
# (no nytprof.out). G03b emit APIs write via enable_sink + nytp_emit_*.
# Distinct from xs-bootstrap (CollectorBootstrap). Dest is xs-nytprof/.
NYTP_XS        := xs/NYTProf.xs
NYTP_C         := $(BUILD)/NYTProf.c
NYTP_O         := $(BUILD)/NYTProf.o
# Embed ccopts like NYTProf.o — not the collector src/%.c sink rule.
NYTP_PP_ENTERSUB_C := xs/pp_entersub.c
NYTP_PP_ENTERSUB_O := $(BUILD)/pp_entersub.o
NYTP_PP_LEAVE_C    := xs/pp_leave.c
NYTP_PP_LEAVE_O    := $(BUILD)/pp_leave.o
NYTP_DEST      := $(BUILD)/xs-nytprof
NYTP_SO_DIR    := $(NYTP_DEST)/auto/Devel/NYTProfM
NYTP_SO        := $(NYTP_SO_DIR)/NYTProfM.so
NYTP_PM_SRC    := xs/Devel/NYTProfM.pm
NYTP_CORE_SRC  := xs/Devel/NYTProfM/Core.pm
NYTP_PM_DST    := $(NYTP_DEST)/Devel/NYTProfM.pm
NYTP_CORE_DST  := $(NYTP_DEST)/Devel/NYTProfM/Core.pm

.PHONY: all test clean dirs gen-e3-fixtures \
	libnytp_sink_v5.a probe-v5 xs-bootstrap xs-nytprof xs-nytprof-v6

all: $(LIB) $(LIB_V5) $(TESTBINS) $(GEN_E3)

dirs:
	@mkdir -p $(BUILD)

# Order-only | dirs so -j never races mkdir with cc -o build/*.o
$(BUILD)/%.o: src/%.c include/nytp_sink.h include/nytp_types.h \
		include/nytp_sink_counting.h include/nytp_sink_v5.h \
		include/nytp_sink_v6.h include/nytp_sink_dual.h \
		include/nytprof_v6_ids.h \
		include/nytp_clock.h include/nytp_batch.h include/nytp_event.h \
		include/nytp_fork.h | dirs
	$(CC) $(CPPFLAGS) $(CFLAGS) -c -o $@ $<

$(LIB): $(OBJ) | dirs
	$(AR) rcs $@ $^

$(PIC_DIR)/%.o: src/%.c include/nytp_sink.h include/nytp_types.h \
		include/nytp_sink_counting.h include/nytp_sink_v5.h \
		include/nytp_clock.h include/nytp_batch.h include/nytp_event.h \
		include/nytp_fork.h | dirs
	@mkdir -p $(PIC_DIR)
	$(CC) $(CPPFLAGS) $(CFLAGS) -fPIC -c -o $@ $<

$(LIB_V5): $(OBJ_V5) | dirs
	rm -f $@
	$(AR) rcs $@ $^

# PHONY alias: make -C collector libnytp_sink_v5.a
libnytp_sink_v5.a: $(LIB_V5)

# -lz-only consumer of the v5 product archive (not part of make test).
$(PROBE_V5): t/probe_v5_product_link.c $(LIB_V5) | dirs
	$(CC) $(CPPFLAGS) $(CFLAGS) -o $@ t/probe_v5_product_link.c $(LIB_V5) $(LDFLAGS) $(LDLIBS_V5)

probe-v5: $(PROBE_V5)
	./$(PROBE_V5)

$(XS_C): $(XS_XS) | dirs
	$(PERL) -MExtUtils::ParseXS=process_file -e \
		'process_file(filename => "$(XS_XS)", output => "$(XS_C)", prototypes => 0)'

$(XS_O): $(XS_C) include/nytp_sink.h include/nytp_sink_v5.h
	$(CC) $(CPPFLAGS) -fPIC -O2 -g \
		`$(PERL) -MExtUtils::Embed -e ccopts` \
		-c -o $@ $(XS_C)

$(XS_PM_DST): $(XS_PM_SRC) | dirs
	@mkdir -p $(dir $@)
	cp -f $(XS_PM_SRC) $@

# XS .so uses Config lddlflags + -lz only. Embed ldopts pulls -lperl, which
# fails on Debian without a libperl.so symlink; the module is loaded by perl.
$(XS_SO): $(XS_O) $(LIB_V5) | dirs
	@mkdir -p $(XS_SO_DIR)
	$(CC) `$(PERL) -MConfig -e 'print $$Config{lddlflags}'` \
		-o $@ $(XS_O) $(LIB_V5) $(LDFLAGS) $(LDLIBS_V5)

xs-bootstrap: $(XS_SO) $(XS_PM_DST)

$(NYTP_C): $(NYTP_XS) | dirs
	$(PERL) -MExtUtils::ParseXS=process_file -e \
		'process_file(filename => "$(NYTP_XS)", output => "$(NYTP_C)", prototypes => 0)'

$(NYTP_O): $(NYTP_C) include/nytp_sink.h include/nytp_sink_v5.h include/nytp_clock.h \
		include/nytp_fork.h xs/nytprof_pp.h xs/slowops.h
	$(CC) $(CPPFLAGS) -Ixs -fPIC -O2 -g \
		`$(PERL) -MExtUtils::Embed -e ccopts` \
		-c -o $@ $(NYTP_C)

$(NYTP_PP_ENTERSUB_O): $(NYTP_PP_ENTERSUB_C) xs/nytprof_pp.h \
		include/nytp_sink.h include/nytp_clock.h include/nytp_types.h | dirs
	$(CC) $(CPPFLAGS) -Ixs -fPIC -O2 -g \
		`$(PERL) -MExtUtils::Embed -e ccopts` \
		-c -o $@ $(NYTP_PP_ENTERSUB_C)

$(NYTP_PP_LEAVE_O): $(NYTP_PP_LEAVE_C) xs/nytprof_pp.h \
		include/nytp_sink.h include/nytp_clock.h include/nytp_types.h | dirs
	$(CC) $(CPPFLAGS) -Ixs -fPIC -O2 -g \
		`$(PERL) -MExtUtils::Embed -e ccopts` \
		-c -o $@ $(NYTP_PP_LEAVE_C)

$(NYTP_PM_DST): $(NYTP_PM_SRC) | dirs
	@mkdir -p $(dir $@)
	cp -f $(NYTP_PM_SRC) $@

$(NYTP_CORE_DST): $(NYTP_CORE_SRC) | dirs
	@mkdir -p $(dir $@)
	cp -f $(NYTP_CORE_SRC) $@

# XS .so uses Config lddlflags + -lz only (same D1-B / G02 pattern).
$(NYTP_SO): $(NYTP_O) $(NYTP_PP_ENTERSUB_O) $(NYTP_PP_LEAVE_O) $(LIB_V5) | dirs
	@mkdir -p $(NYTP_SO_DIR)
	$(CC) `$(PERL) -MConfig -e 'print $$Config{lddlflags}'` \
		-o $@ $(NYTP_O) $(NYTP_PP_ENTERSUB_O) $(NYTP_PP_LEAVE_O) $(LIB_V5) $(LDFLAGS) $(LDLIBS_V5)

xs-nytprof: $(NYTP_SO) $(NYTP_PM_DST) $(NYTP_CORE_DST)

# G05 D1-A: v5 PIC objects + PIC nytp_sink_v6.o; -lz -lzstd -llz4.
# Separate dest so D1-B xs-nytprof is not contaminated.
LDLIBS_V6C     := -lz -lzstd -llz4
LIB_V6C        := $(BUILD)/libnytp_sink_v6collect.a
PIC_V6_O       := $(PIC_DIR)/nytp_sink_v6.o
NYTP_V6_DEST   := $(BUILD)/xs-nytprof-v6
NYTP_V6_SO_DIR := $(NYTP_V6_DEST)/auto/Devel/NYTProfM
NYTP_V6_SO     := $(NYTP_V6_SO_DIR)/NYTProfM.so
NYTP_O_V6      := $(BUILD)/NYTProf_v6.o
NYTP_V6_PM     := $(NYTP_V6_DEST)/Devel/NYTProfM.pm
NYTP_V6_CORE   := $(NYTP_V6_DEST)/Devel/NYTProfM/Core.pm

$(PIC_V6_O): src/nytp_sink_v6.c include/nytp_sink.h include/nytp_types.h \
		include/nytp_sink_v6.h include/nytprof_v6_ids.h \
		include/nytp_sink_counting.h | dirs
	@mkdir -p $(PIC_DIR)
	$(CC) $(CPPFLAGS) $(CFLAGS) -fPIC -c -o $@ src/nytp_sink_v6.c

$(LIB_V6C): $(OBJ_V5) $(PIC_V6_O) | dirs
	rm -f $@
	$(AR) rcs $@ $^

$(NYTP_O_V6): $(NYTP_C) include/nytp_sink.h include/nytp_sink_v5.h \
		include/nytp_sink_v6.h include/nytp_clock.h xs/nytprof_pp.h \
		xs/slowops.h
	$(CC) $(CPPFLAGS) -Ixs -fPIC -O2 -g -DNYTPROF_V6_COLLECT \
		`$(PERL) -MExtUtils::Embed -e ccopts` \
		-c -o $@ $(NYTP_C)

$(NYTP_V6_PM): $(NYTP_PM_SRC) | dirs
	@mkdir -p $(dir $@)
	cp -f $(NYTP_PM_SRC) $@

$(NYTP_V6_CORE): $(NYTP_CORE_SRC) | dirs
	@mkdir -p $(dir $@)
	cp -f $(NYTP_CORE_SRC) $@

$(NYTP_V6_SO): $(NYTP_O_V6) $(NYTP_PP_ENTERSUB_O) $(NYTP_PP_LEAVE_O) $(LIB_V6C) | dirs
	@mkdir -p $(NYTP_V6_SO_DIR)
	$(CC) `$(PERL) -MConfig -e 'print $$Config{lddlflags}'` \
		-o $@ $(NYTP_O_V6) $(NYTP_PP_ENTERSUB_O) $(NYTP_PP_LEAVE_O) $(LIB_V6C) $(LDFLAGS) $(LDLIBS_V6C)

xs-nytprof-v6: $(NYTP_V6_SO) $(NYTP_V6_PM) $(NYTP_V6_CORE)

$(BUILD)/test_sink_api: t/test_sink_api.c $(LIB) | dirs
	$(CC) $(CPPFLAGS) $(CFLAGS) -o $@ t/test_sink_api.c $(LIB) $(LDFLAGS) $(LDLIBS)

$(BUILD)/test_lifecycle_seq: t/test_lifecycle_seq.c $(LIB) | dirs
	$(CC) $(CPPFLAGS) $(CFLAGS) -o $@ t/test_lifecycle_seq.c $(LIB) $(LDFLAGS) $(LDLIBS)

$(BUILD)/test_fake_clock: t/test_fake_clock.c $(LIB) | dirs
	$(CC) $(CPPFLAGS) $(CFLAGS) -o $@ t/test_fake_clock.c $(LIB) $(LDFLAGS) $(LDLIBS)

$(BUILD)/test_batch_fast: t/test_batch_fast.c $(LIB) | dirs
	$(CC) $(CPPFLAGS) $(CFLAGS) -o $@ t/test_batch_fast.c $(LIB) $(LDFLAGS) $(LDLIBS)

$(BUILD)/test_v5_wire: t/test_v5_wire.c $(LIB) | dirs
	$(CC) $(CPPFLAGS) $(CFLAGS) -o $@ t/test_v5_wire.c $(LIB) $(LDFLAGS) $(LDLIBS)

$(BUILD)/test_v6_abs_wire: t/test_v6_abs_wire.c $(LIB) | dirs
	$(CC) $(CPPFLAGS) $(CFLAGS) -o $@ t/test_v6_abs_wire.c $(LIB) $(LDFLAGS) $(LDLIBS)

$(BUILD)/test_v6_codec_chunk_crc: t/test_v6_codec_chunk_crc.c $(LIB) | dirs
	$(CC) $(CPPFLAGS) $(CFLAGS) -o $@ t/test_v6_codec_chunk_crc.c $(LIB) $(LDFLAGS) $(LDLIBS)

$(BUILD)/test_v6_packing_footer: t/test_v6_packing_footer.c $(LIB) | dirs
	$(CC) $(CPPFLAGS) $(CFLAGS) -o $@ t/test_v6_packing_footer.c $(LIB) $(LDFLAGS) $(LDLIBS)

$(BUILD)/test_dual_sink: t/test_dual_sink.c $(LIB) | dirs
	$(CC) $(CPPFLAGS) $(CFLAGS) -o $@ t/test_dual_sink.c $(LIB) $(LDFLAGS) $(LDLIBS)

$(BUILD)/test_fork_pid: t/test_fork_pid.c $(LIB) | dirs
	$(CC) $(CPPFLAGS) $(CFLAGS) -o $@ t/test_fork_pid.c $(LIB) $(LDFLAGS) $(LDLIBS)

$(GEN_E3): t/gen_e3_c_fixtures.c $(LIB) | dirs
	$(CC) $(CPPFLAGS) $(CFLAGS) -o $@ t/gen_e3_c_fixtures.c $(LIB) $(LDFLAGS) $(LDLIBS)

# Produce committed product E3 C fixtures (PR-B09). Run from repo root preferred:
#   make -C collector gen-e3-fixtures OUTDIR=$$(pwd)/fixtures/v6/from-c
gen-e3-fixtures: $(GEN_E3)
	@mkdir -p "$(OUTDIR)"
	./$(GEN_E3) "$(OUTDIR)"

test: all
	./$(BUILD)/test_sink_api
	./$(BUILD)/test_lifecycle_seq
	./$(BUILD)/test_fake_clock
	./$(BUILD)/test_batch_fast
	./$(BUILD)/test_v5_wire
	./$(BUILD)/test_v6_abs_wire
	./$(BUILD)/test_v6_codec_chunk_crc
	./$(BUILD)/test_v6_packing_footer
	./$(BUILD)/test_dual_sink
	./$(BUILD)/test_fork_pid

clean:
	rm -rf $(BUILD)
