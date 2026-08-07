#!/usr/bin/env python3
"""Normalize ReadStream JSONL dumps for structural golden compare.

Canonical event dump schema: docs/schemas/canonical-event-dump-v0.md

Modes:
  structural (default)
    - COMMENT args -> ["<COMMENT>"]
    - ATTRIBUTE basetime value -> "<BASETIME>"
    - ATTRIBUTE application value -> basename (or "<APP>" if empty)
    - NEW_FID name (last arg): basename when path-like
    - Floating NVs: re-encoded with stable %.17g representation via JSON
    - Keep tag order; keep or drop _END consistently (keep by default)
    - Renumber seq from 0 (default) for determinism

Usage:
  normalize_jsonl.py [--mode structural] [--preserve-seq] [--drop-end] [input.jsonl]
  Reads stdin if no file; writes normalized JSONL to stdout.
"""
from __future__ import annotations

import argparse
import json
import math
import re
import sys
from typing import Any, List, Optional, TextIO

# Path-like: has a directory separator, or absolute Unix/Windows path prefix.
_PATH_SEP_RE = re.compile(r"[/\\]")
_ABS_WIN_RE = re.compile(r"^[A-Za-z]:[/\\]")


def looks_like_path(s: str) -> bool:
    if not s:
        return False
    if s.startswith("/") or s.startswith("\\"):
        return True
    if _ABS_WIN_RE.match(s):
        return True
    # Relative paths with a separator (not bare filenames / package names)
    if _PATH_SEP_RE.search(s):
        return True
    return False


def basename_path(s: str) -> str:
    """POSIX/Windows-ish basename without importing pathlib (pure logic)."""
    if not s:
        return s
    # Normalize to forward for split; keep last component
    s2 = s.replace("\\", "/")
    # Strip trailing slashes (except root)
    while len(s2) > 1 and s2.endswith("/"):
        s2 = s2[:-1]
    if "/" in s2:
        return s2.rsplit("/", 1)[-1]
    return s2


def is_float_like(x: Any) -> bool:
    """True for non-integer real numbers (JSON floats that are not whole ints)."""
    if isinstance(x, bool):
        return False
    if isinstance(x, float):
        if math.isnan(x) or math.isinf(x):
            return True
        return not x.is_integer()
    return False


def normalize_number(x: Any) -> Any:
    """Leave ints alone; re-box floats so dumps are deterministic enough."""
    if isinstance(x, bool):
        return x
    if isinstance(x, int):
        return x
    if isinstance(x, float):
        if math.isnan(x) or math.isinf(x):
            # JSON has no NaN/Inf by default; stringify sentinel
            if math.isnan(x):
                return "<NAN>"
            return "<INF>" if x > 0 else "<-INF>"
        if x.is_integer() and abs(x) <= 2**53:
            return int(x)
        # Round-trip via fixed precision string then back to float so
        # json.dumps emits a stable form for equal values after load.
        return float(f"{x:.17g}")
    return x


def normalize_args(tag: str, args: List[Any], mode: str) -> List[Any]:
    if mode != "structural":
        raise ValueError(f"unsupported mode: {mode}")

    out: List[Any] = list(args)

    if tag == "COMMENT":
        return ["<COMMENT>"]

    if tag == "ATTRIBUTE" and len(out) >= 2:
        key = out[0]
        val = out[1]
        if key == "basetime":
            out[1] = "<BASETIME>"
        elif key == "application":
            if val is None or val == "":
                out[1] = "<APP>"
            elif isinstance(val, str):
                out[1] = basename_path(val) if looks_like_path(val) else (
                    val if val else "<APP>"
                )
                if out[1] == "":
                    out[1] = "<APP>"
            else:
                out[1] = val
        # normalize other attribute values if numeric float
        for i in range(len(out)):
            out[i] = normalize_number(out[i]) if not isinstance(out[i], str) else out[i]
        return out

    if tag == "NEW_FID" and len(out) >= 1:
        # name is last arg
        name = out[-1]
        if isinstance(name, str) and looks_like_path(name):
            out[-1] = basename_path(name)
        for i in range(len(out) - 1):
            out[i] = normalize_number(out[i])
        return out

    # Generic: normalize numbers in all args
    return [normalize_number(a) if not isinstance(a, str) else a for a in out]


def normalize_record(
    obj: dict,
    seq: int,
    mode: str,
    preserve_seq: bool,
) -> Optional[dict]:
    tag = obj.get("tag")
    if tag is None:
        raise ValueError(f"record missing tag: {obj!r}")
    args = obj.get("args")
    if args is None:
        args = []
    if not isinstance(args, list):
        raise ValueError(f"args must be array for tag {tag}")

    new_args = normalize_args(str(tag), args, mode)
    new_seq = obj.get("seq", seq) if preserve_seq else seq
    return {"seq": new_seq, "tag": tag, "args": new_args}


def process_stream(
    inp: TextIO,
    out: TextIO,
    mode: str = "structural",
    preserve_seq: bool = False,
    drop_end: bool = False,
) -> int:
    """Normalize JSONL from inp to out. Returns record count written."""
    # Deterministic encoder: sorted keys, compact separators, no ASCII escape forced
    encoder = json.JSONEncoder(
        ensure_ascii=False,
        separators=(",", ":"),
        allow_nan=False,
    )
    # For floats, json uses repr-like; we pre-normalized numbers.

    n_out = 0
    line_no = 0
    for raw in inp:
        line_no += 1
        line = raw.strip()
        if not line:
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError as e:
            raise SystemExit(f"JSON decode error at line {line_no}: {e}") from e
        if not isinstance(obj, dict):
            raise SystemExit(f"line {line_no}: expected object, got {type(obj).__name__}")

        tag = obj.get("tag")
        if drop_end and tag == "_END":
            continue

        rec = normalize_record(obj, n_out, mode, preserve_seq)
        if rec is None:
            continue
        # Canonical key order: seq, tag, args (matches schema examples)
        ordered = {"seq": rec["seq"], "tag": rec["tag"], "args": rec["args"]}
        out.write(encoder.encode(ordered))
        out.write("\n")
        n_out += 1
    return n_out


def main(argv: Optional[List[str]] = None) -> int:
    p = argparse.ArgumentParser(
        description="Normalize ReadStream JSONL for structural golden compare"
    )
    p.add_argument(
        "input",
        nargs="?",
        help="input JSONL file (default: stdin)",
    )
    p.add_argument(
        "--mode",
        default="structural",
        choices=("structural",),
        help="normalization mode (default: structural)",
    )
    p.add_argument(
        "--preserve-seq",
        action="store_true",
        help="keep original seq values instead of renumbering from 0",
    )
    p.add_argument(
        "--drop-end",
        action="store_true",
        help="drop synthetic _END records",
    )
    p.add_argument(
        "-o",
        "--output",
        help="output file (default: stdout)",
    )
    args = p.parse_args(argv)

    if args.input:
        fin: TextIO = open(args.input, "r", encoding="utf-8")
    else:
        fin = sys.stdin

    if args.output:
        fout: TextIO = open(args.output, "w", encoding="utf-8")
    else:
        fout = sys.stdout

    try:
        process_stream(
            fin,
            fout,
            mode=args.mode,
            preserve_seq=args.preserve_seq,
            drop_end=args.drop_end,
        )
    finally:
        if fin is not sys.stdin:
            fin.close()
        if fout is not sys.stdout:
            fout.close()
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except BrokenPipeError:
        # e.g. `normalize_jsonl.py … | head` — exit quietly
        try:
            sys.stdout.close()
        except Exception:
            pass
        sys.exit(0)
