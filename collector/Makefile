# COL-001 collector overlay build (ADR-0004 B0-A).
# Compiles when a C toolchain is present. Not required for legacy-only packaging.
#
# Usage (from repo root or collector/):
#   make -C collector
#   make -C collector test
#   make -C collector clean
#
# Outputs stay under collector/build/ (gitignored). Never writes baseline/6.15/.

CC      ?= cc
CFLAGS  ?= -std=c11 -Wall -Wextra -Wpedantic -O2 -g
CPPFLAGS += -Iinclude
LDFLAGS ?=
AR      ?= ar

BUILD   := build
LIB     := $(BUILD)/libnytp_sink.a
TESTBIN := $(BUILD)/test_sink_api

SRC := \
	src/nytp_sink.c \
	src/nytp_sink_counting.c \
	src/nytp_sink_v5.c

OBJ := $(patsubst src/%.c,$(BUILD)/%.o,$(SRC))

.PHONY: all test clean dirs

all: dirs $(LIB) $(TESTBIN)

dirs:
	@mkdir -p $(BUILD)

$(BUILD)/%.o: src/%.c include/nytp_sink.h include/nytp_types.h \
		include/nytp_sink_counting.h include/nytp_sink_v5.h
	$(CC) $(CPPFLAGS) $(CFLAGS) -c -o $@ $<

$(LIB): $(OBJ)
	$(AR) rcs $@ $^

$(TESTBIN): t/test_sink_api.c $(LIB)
	$(CC) $(CPPFLAGS) $(CFLAGS) -o $@ t/test_sink_api.c $(LIB) $(LDFLAGS)

test: all
	./$(TESTBIN)

clean:
	rm -rf $(BUILD)
