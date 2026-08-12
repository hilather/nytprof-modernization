# COL-001..007 + COL-014 collector overlay build (ADR-0004 B0-A).
# Compiles when a C toolchain is present. Not required for legacy-only packaging.
# COL-006 v5 wire writer links zlib (-lz).
# COL-007 v6 writer links zlib + zstd + lz4 for EVENT payload codecs (PR-B07).
# COL-014 dual-sink is test/dev-only (OQ-4) — not product UX.
#
# Usage (from repo root or collector/):
#   make -C collector
#   make -C collector test
#   make -C collector clean
#   make -C collector -j$(nproc) test   # parallel-safe via order-only | dirs
#
# Outputs stay under collector/build/ (gitignored). Never writes baseline/6.15/.

CC      ?= cc
CFLAGS  ?= -std=c11 -Wall -Wextra -Wpedantic -O2 -g
CPPFLAGS += -Iinclude
LDFLAGS ?=
LDLIBS  ?= -lz -lzstd -llz4
AR      ?= ar

BUILD   := build
LIB     := $(BUILD)/libnytp_sink.a
TESTBINS := \
	$(BUILD)/test_sink_api \
	$(BUILD)/test_lifecycle_seq \
	$(BUILD)/test_fake_clock \
	$(BUILD)/test_batch_fast \
	$(BUILD)/test_v5_wire \
	$(BUILD)/test_v6_abs_wire \
	$(BUILD)/test_v6_codec_chunk_crc \
	$(BUILD)/test_v6_packing_footer \
	$(BUILD)/test_dual_sink

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
	src/nytp_batch.c

OBJ := $(patsubst src/%.c,$(BUILD)/%.o,$(SRC))

.PHONY: all test clean dirs gen-e3-fixtures

all: $(LIB) $(TESTBINS) $(GEN_E3)

dirs:
	@mkdir -p $(BUILD)

# Order-only | dirs so -j never races mkdir with cc -o build/*.o
$(BUILD)/%.o: src/%.c include/nytp_sink.h include/nytp_types.h \
		include/nytp_sink_counting.h include/nytp_sink_v5.h \
		include/nytp_sink_v6.h include/nytp_sink_dual.h \
		include/nytprof_v6_ids.h \
		include/nytp_clock.h include/nytp_batch.h include/nytp_event.h | dirs
	$(CC) $(CPPFLAGS) $(CFLAGS) -c -o $@ $<

$(LIB): $(OBJ) | dirs
	$(AR) rcs $@ $^

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

clean:
	rm -rf $(BUILD)
