# COL-001..003 collector overlay build (ADR-0004 B0-A).
# Compiles when a C toolchain is present. Not required for legacy-only packaging.
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
AR      ?= ar

BUILD   := build
LIB     := $(BUILD)/libnytp_sink.a
TESTBINS := \
	$(BUILD)/test_sink_api \
	$(BUILD)/test_lifecycle_seq \
	$(BUILD)/test_fake_clock

SRC := \
	src/nytp_sink.c \
	src/nytp_sink_counting.c \
	src/nytp_sink_v5.c \
	src/nytp_clock.c

OBJ := $(patsubst src/%.c,$(BUILD)/%.o,$(SRC))

.PHONY: all test clean dirs

all: $(LIB) $(TESTBINS)

dirs:
	@mkdir -p $(BUILD)

# Order-only | dirs so -j never races mkdir with cc -o build/*.o
$(BUILD)/%.o: src/%.c include/nytp_sink.h include/nytp_types.h \
		include/nytp_sink_counting.h include/nytp_sink_v5.h \
		include/nytp_clock.h | dirs
	$(CC) $(CPPFLAGS) $(CFLAGS) -c -o $@ $<

$(LIB): $(OBJ) | dirs
	$(AR) rcs $@ $^

$(BUILD)/test_sink_api: t/test_sink_api.c $(LIB) | dirs
	$(CC) $(CPPFLAGS) $(CFLAGS) -o $@ t/test_sink_api.c $(LIB) $(LDFLAGS)

$(BUILD)/test_lifecycle_seq: t/test_lifecycle_seq.c $(LIB) | dirs
	$(CC) $(CPPFLAGS) $(CFLAGS) -o $@ t/test_lifecycle_seq.c $(LIB) $(LDFLAGS)

$(BUILD)/test_fake_clock: t/test_fake_clock.c $(LIB) | dirs
	$(CC) $(CPPFLAGS) $(CFLAGS) -o $@ t/test_fake_clock.c $(LIB) $(LDFLAGS)

test: all
	./$(BUILD)/test_sink_api
	./$(BUILD)/test_lifecycle_seq
	./$(BUILD)/test_fake_clock

clean:
	rm -rf $(BUILD)
