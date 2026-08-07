#!/usr/bin/env python3
"""Build aggregate-comparison-v0 totals from a ReadStream JSONL dump.

Schema: docs/schemas/aggregate-comparison-v0.md

Usage:
  python3 tools/oracle/aggregate_from_jsonl.py path/to/readstream.jsonl
  python3 tools/oracle/aggregate_from_jsonl.py path/to/readstream.jsonl -o aggregates.oracle.json

Reads stdin if no file is given. Writes a single JSON object to stdout (or -o).

Python 3 stdlib only.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
from collections import OrderedDict
from typing import Any, Dict, List, Optional, TextIO, Tuple


SCHEMA = "aggregate-comparison-v0"


def _as_number(x: Any) -> float:
    if isinstance(x, bool):
        raise TypeError(f"expected number, got bool: {x!r}")
    if isinstance(x, (int, float)):
        return float(x)
    raise TypeError(f"expected number, got {type(x).__name__}: {x!r}")


def _as_int(x: Any) -> int:
    if isinstance(x, bool):
        raise TypeError(f"expected int, got bool: {x!r}")
    if isinstance(x, int):
        return x
    if isinstance(x, float) and x.is_integer():
        return int(x)
    raise TypeError(f"expected int, got {type(x).__name__}: {x!r}")


def _line_key(fid: int, line: int) -> str:
    return f"{fid}:{line}"


def _is_workload_sub(name: str) -> bool:
    """A6: leaf/mid workload subs present in fixtures."""
    if name.endswith("::leaf") or name.endswith("::mid"):
        return True
    if "main::leaf" in name or "main::mid" in name:
        return True
    return name in ("main::leaf", "main::mid")


def _source_label(path: Optional[str]) -> str:
    """Prefer repo-relative fixtures/... path when obvious."""
    if not path or path == "-":
        return "<stdin>"
    # Normalize separators
    p = path.replace("\\", "/")
    # If already relative under fixtures/, keep it
    if p.startswith("fixtures/"):
        return p
    # Walk for a fixtures/ segment
    parts = p.split("/")
    if "fixtures" in parts:
        i = parts.index("fixtures")
        return "/".join(parts[i:])
    return p


def aggregate_stream(inp: TextIO, source: str) -> Dict[str, Any]:
    time_line_events = 0
    time_block_events = 0
    discount_events = 0

    # line_key -> [calls, ticks]
    line_totals: Dict[str, List[int]] = {}
    # subname -> [returns, incl, excl]
    sub_totals: Dict[str, List[Any]] = {}

    line_no = 0
    for raw in inp:
        line_no += 1
        text = raw.strip()
        if not text:
            continue
        try:
            obj = json.loads(text)
        except json.JSONDecodeError as e:
            raise SystemExit(f"JSON decode error at line {line_no}: {e}") from e
        if not isinstance(obj, dict):
            raise SystemExit(
                f"line {line_no}: expected object, got {type(obj).__name__}"
            )

        tag = obj.get("tag")
        args = obj.get("args")
        if args is None:
            args = []
        if not isinstance(args, list):
            raise SystemExit(f"line {line_no}: args must be array for tag {tag!r}")

        if tag == "TIME_LINE":
            # args: ticks, fid, line
            if len(args) < 3:
                raise SystemExit(
                    f"line {line_no}: TIME_LINE needs ticks,fid,line; got {args!r}"
                )
            ticks = _as_int(args[0])
            fid = _as_int(args[1])
            ln = _as_int(args[2])
            time_line_events += 1
            key = _line_key(fid, ln)
            if key not in line_totals:
                line_totals[key] = [0, 0]
            line_totals[key][0] += 1
            line_totals[key][1] += ticks

        elif tag == "TIME_BLOCK":
            time_block_events += 1
            # MVP: do not fold into line_totals (TIME_LINE only)

        elif tag == "DISCOUNT":
            discount_events += 1

        elif tag == "SUB_RETURN":
            # args: depth, incl_time, excl_time, subname
            if len(args) < 4:
                raise SystemExit(
                    f"line {line_no}: SUB_RETURN needs depth,incl,excl,subname; "
                    f"got {args!r}"
                )
            incl = _as_number(args[1])
            excl = _as_number(args[2])
            subname = args[3]
            if not isinstance(subname, str):
                raise SystemExit(
                    f"line {line_no}: SUB_RETURN subname must be string, "
                    f"got {type(subname).__name__}"
                )
            if subname not in sub_totals:
                sub_totals[subname] = [0, 0.0, 0.0]
            sub_totals[subname][0] += 1
            sub_totals[subname][1] += incl
            sub_totals[subname][2] += excl

    # Deterministic sorted maps
    line_out: Dict[str, Dict[str, int]] = OrderedDict()
    for key in sorted(line_totals.keys(), key=_line_key_sort):
        calls, ticks = line_totals[key]
        line_out[key] = {"calls": calls, "ticks": ticks}

    sub_out: Dict[str, Dict[str, Any]] = OrderedDict()
    for name in sorted(sub_totals.keys()):
        returns, incl, excl = sub_totals[name]
        # Emit incl/excl as int when whole numbers for stable JSON
        sub_out[name] = {
            "returns": int(returns),
            "incl": _json_number(incl),
            "excl": _json_number(excl),
        }

    workload = sorted(n for n in sub_out if _is_workload_sub(n))

    # Top-level key order matches schema examples / task contract
    return OrderedDict(
        [
            ("schema", SCHEMA),
            ("source", source),
            ("time_line_events", time_line_events),
            ("time_block_events", time_block_events),
            ("discount_events", discount_events),
            ("sub_return_totals", sub_out),
            ("line_totals", line_out),
            ("workload_subs", workload),
        ]
    )


def _json_number(x: float) -> Any:
    """Prefer JSON integers when the sum is integral (stable dumps)."""
    if isinstance(x, float) and x.is_integer() and abs(x) <= 2**53:
        return int(x)
    if isinstance(x, int):
        return x
    return float(x)


def _line_key_sort(key: str) -> Tuple[int, int, str]:
    """Sort 'fid:line' numerically; fall back to string for odd keys."""
    try:
        a, b = key.split(":", 1)
        return (int(a), int(b), key)
    except (ValueError, TypeError):
        return (10**18, 10**18, key)


def dump_aggregate(obj: Dict[str, Any], out: TextIO) -> None:
    # sort_keys=True for nested determinism; top-level OrderedDict keys also sorted
    # but we want fixed top-level order — so encode without global sort_keys and
    # ensure nested dicts already use OrderedDict with sorted keys.
    json.dump(obj, out, ensure_ascii=False, indent=2, sort_keys=False)
    out.write("\n")


def main(argv: Optional[List[str]] = None) -> int:
    p = argparse.ArgumentParser(
        description="Aggregate TIME_LINE / SUB_RETURN totals from ReadStream JSONL"
    )
    p.add_argument(
        "input",
        nargs="?",
        help="input JSONL file (default: stdin)",
    )
    p.add_argument(
        "-o",
        "--output",
        help="output file (default: stdout)",
    )
    p.add_argument(
        "--source",
        help="override source label embedded in the JSON (default: path-derived)",
    )
    args = p.parse_args(argv)

    if args.input:
        fin: TextIO = open(args.input, "r", encoding="utf-8")
        source = args.source if args.source is not None else _source_label(args.input)
    else:
        fin = sys.stdin
        source = args.source if args.source is not None else _source_label(None)

    if args.output:
        fout: TextIO = open(args.output, "w", encoding="utf-8")
    else:
        fout = sys.stdout

    try:
        # If path is absolute under repo, try to make source relative to cwd
        if args.input and args.source is None:
            try:
                rel = os.path.relpath(args.input, os.getcwd())
                if not rel.startswith(".."):
                    source = _source_label(rel)
            except ValueError:
                pass
        agg = aggregate_stream(fin, source)
        dump_aggregate(agg, fout)
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
        try:
            sys.stdout.close()
        except Exception:
            pass
        sys.exit(0)
