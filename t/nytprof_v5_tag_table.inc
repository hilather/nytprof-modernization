# Bounded v5 tag skip table for t/installed_attach.t (PR-A2 / RPM-03).
# Copied from collector/src/nytp_sink_v5.c emit layout (COMPAT-001 tags).
# Parse only SUB_RETURN and SUB_CALLERS; skip others by documented shape.
# Fail closed on unknown tag or oversize length (no large alloc).
package NYTProfM::V5TagTable;
use strict;
use warnings;

our $MAX_STR = 1_048_576;    # 1 MiB fail-closed

# tag byte => skip/parse name (documentation for the scanner)
our %TAG = (
    ':' => 'ATTRIBUTE_TEXT',
    '!' => 'OPTION_TEXT',
    '#' => 'COMMENT_TEXT',
    '+' => 'TIME_LINE',
    '*' => 'TIME_BLOCK',
    '-' => 'DISCOUNT',
    '@' => 'NEW_FID',
    'S' => 'SRC_LINE',
    's' => 'SUB_INFO',
    'c' => 'SUB_CALLERS',
    'P' => 'PID_START',
    'p' => 'PID_END',
    '>' => 'SUB_ENTRY',
    '<' => 'SUB_RETURN',
    'z' => 'START_DEFLATE',
    "'" => 'STRING',
    '"' => 'STRING_UTF8',
);

1;
