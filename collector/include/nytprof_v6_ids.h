/* SPDX-License-Identifier: Artistic-1.0-Perl OR GPL-1.0-or-later
 *
 * nytprof_v6_ids.h — Format v6 numeric ID lockfile (C mirror)
 *
 * Status: FROZEN for SUPPORTED_MAJOR=6 by ADR-0006 (PR-B11).
 *         Values must not change without a new major / superseding ADR.
 *
 * Normative docs:
 *   docs/adrs/0006-v6-wire-freeze.md
 *   docs/schemas/v6-wire-ids-frozen-v1.md
 *   docs/contracts/V6_PROVISIONAL_ID_LOCKFILE_v0.md  (historical path; status frozen)
 *   docs/adrs/0001-v6-event-body-packing-candidate.md  (accepted packing intent)
 *   docs/adrs/0002-v6-string-pool-candidate.md         (accepted FOOTER-local dict)
 *
 * Must stay aligned with crates/nytprof-format-v6 (MAGIC, SUPPORTED_MAJOR,
 * chunk::kind/codec, event_body::opcode/flags, tlv::type_id, …).
 *
 * Golden vectors: fixtures/v6/vectors/
 */
#ifndef NYTPROF_V6_IDS_H
#define NYTPROF_V6_IDS_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ---- Fixed header ------------------------------------------------------- */

/** ASCII "NYTPROF6" — 8 bytes, same as nytprof_format_v6::MAGIC. */
#define NYTPROF_V6_MAGIC_0 'N'
#define NYTPROF_V6_MAGIC_1 'Y'
#define NYTPROF_V6_MAGIC_2 'T'
#define NYTPROF_V6_MAGIC_3 'P'
#define NYTPROF_V6_MAGIC_4 'R'
#define NYTPROF_V6_MAGIC_5 'O'
#define NYTPROF_V6_MAGIC_6 'F'
#define NYTPROF_V6_MAGIC_7 '6'

#define NYTPROF_V6_MAGIC_LEN 8
#define NYTPROF_V6_SUPPORTED_MAJOR 6u
#define NYTPROF_V6_HEADER_LEN_MIN 16u
#define NYTPROF_V6_HEADER_LEN_FULL 36u
#define NYTPROF_V6_MAX_HEADER_LEN (1024u * 1024u)

/* ---- Chunk frame -------------------------------------------------------- */

/** ASCII "NYT6" as little-endian u32 (0x3654594E). */
#define NYTPROF_V6_CHUNK_SYNC 0x3654594Eu
#define NYTPROF_V6_CHUNK_HEADER_LEN 40u
#define NYTPROF_V6_MAX_CHUNK_PAYLOAD (64u * 1024u * 1024u)
#define NYTPROF_V6_FLAG_KIND_REQUIRED 0x0001u

/* chunk::kind */
#define NYTPROF_V6_KIND_RESERVED 0u
#define NYTPROF_V6_KIND_EVENT 1u
#define NYTPROF_V6_KIND_SOURCE 2u
#define NYTPROF_V6_KIND_INDEX 3u
#define NYTPROF_V6_KIND_SUMMARY 4u
#define NYTPROF_V6_KIND_FOOTER 5u

/* chunk::codec */
#define NYTPROF_V6_CODEC_NONE 0u
#define NYTPROF_V6_CODEC_ZLIB 1u
#define NYTPROF_V6_CODEC_ZSTD 2u
#define NYTPROF_V6_CODEC_LZ4 3u

/* ---- Event-body opcodes (event_body::opcode) ---------------------------- */

#define NYTPROF_V6_OP_RESERVED 0u
#define NYTPROF_V6_OP_MARK 1u
#define NYTPROF_V6_OP_TIME_LINE 2u
#define NYTPROF_V6_OP_TIME_BLOCK 3u
#define NYTPROF_V6_OP_SUB_ENTRY 4u
#define NYTPROF_V6_OP_SUB_RETURN 5u
#define NYTPROF_V6_OP_SUB_INFO 6u
#define NYTPROF_V6_OP_SRC_LINE 7u
#define NYTPROF_V6_OP_NEW_FID 8u
#define NYTPROF_V6_OP_PID_START 9u
#define NYTPROF_V6_OP_PID_END 10u
#define NYTPROF_V6_OP_SUB_CALLERS 11u
#define NYTPROF_V6_OP_DISCOUNT 12u
#define NYTPROF_V6_OP_ATTRIBUTE 13u
#define NYTPROF_V6_OP_OPTION 14u
#define NYTPROF_V6_OP_COMMENT 15u
#define NYTPROF_V6_OP_START_DEFLATE 16u
#define NYTPROF_V6_OP_VERSION 17u
/** ADR-0001 packing forms (frozen opcode numbers; ADR-0006). */
#define NYTPROF_V6_OP_TIME_LINE_RUN 18u
#define NYTPROF_V6_OP_TIME_BLOCK_RUN 19u

/* Event-body flags (u8 after opcode) — frozen bit assignments (ADR-0006) */
#define NYTPROF_V6_FLAG_OPCODE_REQUIRED 0x01u
#define NYTPROF_V6_FLAG_BODY_LENGTH 0x02u
#define NYTPROF_V6_FLAG_SITE_DELTA 0x04u /* ADR-0001 */
#define NYTPROF_V6_FLAG_HAS_SEQ 0x08u    /* OQ-5 permanent optional seq (ADR-0006) */

/* ---- String-blob -------------------------------------------------------- */

#define NYTPROF_V6_FLAG_UTF8 0x01u
#define NYTPROF_V6_MAX_STRING_BYTES (16ull * 1024ull * 1024ull)

/* ---- Header TLV type_id ------------------------------------------------- */

#define NYTPROF_V6_TLV_RESERVED 0u
#define NYTPROF_V6_TLV_PRODUCER 1u
#define NYTPROF_V6_TLV_TICKS_PER_SEC 2u
#define NYTPROF_V6_TLV_END 0x7eu
#define NYTPROF_V6_FLAG_TYPE_REQUIRED 0x01u

/* ---- Fail-closed caps --------------------------------------------------- */

#define NYTPROF_V6_MAX_EVENT_BODY_BYTES (64ull * 1024ull * 1024ull)
#define NYTPROF_V6_MAX_TLV_VALUE_BYTES (16ull * 1024ull * 1024ull)
#define NYTPROF_V6_MAX_TLV_REGION_BYTES (64ull * 1024ull * 1024ull)
#define NYTPROF_V6_MAX_TIME_RUN_LEN 1048576u

/* ---- FOOTER string-dictionary (ADR-0002; IDs frozen ADR-0006) ------------ */

#define NYTPROF_V6_MAX_DICT_ENTRIES 1048576u
#define NYTPROF_V6_MAX_DICT_TOTAL_BYTES (64ull * 1024ull * 1024ull)

#ifdef __cplusplus
}
#endif

#endif /* NYTPROF_V6_IDS_H */
