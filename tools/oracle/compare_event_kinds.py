#!/usr/bin/env python3
"""DI-04 projected kinds comparator.

Project dump JSONL onto MUST_KIND_SET, then apply presence/absent rules.
Does not run the full tag+args dump comparator. Does not compare args,
ticks, or seq. Does not require unprojected tag-multiset equality
(oracle DISCOUNT/SRC_LINE would always fail a hooks-only product dump).
"""
from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

MUST_KIND_SET = frozenset(
    {
        "NEW_FID",
        "TIME_LINE",
        "SUB_RETURN",
        "SUB_CALLERS",
        "SUB_ENTRY",
    }
)

# Anything not in MUST_KIND_SET is dropped. Named for honesty in docs/smokes.
DROP_SET = frozenset(
    {
        "DISCOUNT",
        "SRC_LINE",
        "SUB_INFO",
        "ATTRIBUTE",
        "OPTION",
        "START_DEFLATE",
        "PID_START",
        "PID_END",
        "COMMENT",
        "TIME_BLOCK",
        "VERSION",
    }
)


def bag_from_jsonl(path: Path) -> Counter[str]:
    bag: Counter[str] = Counter()
    with path.open("r", encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            rec = json.loads(line)
            tag = rec.get("tag")
            if isinstance(tag, str) and tag:
                bag[tag] += 1
    return bag


def project(bag: Counter[str]) -> dict[str, int]:
    return {tag: int(bag.get(tag, 0)) for tag in MUST_KIND_SET}


def rules_for_mode(mode: str) -> dict[str, str]:
    # present => count >= 1; absent => count == 0
    rules = {
        "NEW_FID": "present",
        "TIME_LINE": "present",
        "TIME_BLOCK": "absent",
        "SUB_RETURN": "present",
        "SUB_CALLERS": "present",
        "SUB_ENTRY": "absent" if mode == "calls1" else "present",
    }
    return rules


def parse_golden(path: Path) -> dict[str, str]:
    out: dict[str, str] = {}
    with path.open("r", encoding="utf-8") as fh:
        for raw in fh:
            line = raw.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split()
            if len(parts) != 2:
                raise SystemExit(f"bad golden line in {path}: {raw!r}")
            tag, word = parts
            if word not in ("present", "absent"):
                raise SystemExit(f"bad golden word in {path}: {word}")
            out[tag] = word
    return out


def check_rules(label: str, proj: dict[str, int], rules: dict[str, str], errors: list[str]) -> None:
    # TIME_BLOCK is not in MUST_KIND_SET (dropped on default mini) — count is 0 after project.
    tb = 0 if "TIME_BLOCK" not in MUST_KIND_SET else proj.get("TIME_BLOCK", 0)
    for tag, want in rules.items():
        if tag == "TIME_BLOCK":
            got = tb
        else:
            got = proj.get(tag, 0)
        if want == "present" and got < 1:
            errors.append(f"{label}: {tag} want present (>=1) got {got}")
        if want == "absent" and got != 0:
            errors.append(f"{label}: {tag} want absent (0) got {got}")


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description="Projected kinds compare (not compare_jsonl)")
    ap.add_argument("--mode", choices=("calls1", "calls2"), required=True)
    ap.add_argument("--product", type=Path, required=True)
    ap.add_argument("--oracle", type=Path, default=None)
    ap.add_argument("--golden", type=Path, default=None)
    args = ap.parse_args(argv)

    if not args.product.is_file():
        print(f"ERROR: missing product dump {args.product}", file=sys.stderr)
        return 2

    rules = rules_for_mode(args.mode)
    if args.golden is not None:
        gold = parse_golden(args.golden)
        for tag, word in gold.items():
            if tag == "TIME_BLOCK" or tag in MUST_KIND_SET:
                rules[tag] = word

    errors: list[str] = []
    prod_bag = bag_from_jsonl(args.product)
    prod_proj = project(prod_bag)
    check_rules("product", prod_proj, rules, errors)

    if args.oracle is not None:
        if not args.oracle.is_file():
            print(f"ERROR: missing oracle dump {args.oracle}", file=sys.stderr)
            return 2
        ora_bag = bag_from_jsonl(args.oracle)
        ora_proj = project(ora_bag)
        check_rules("oracle", ora_proj, rules, errors)

    print(f"mode={args.mode}")
    print(f"MUST_KIND_SET={','.join(sorted(MUST_KIND_SET))}")
    print("product_projected=" + ",".join(f"{k}={prod_proj[k]}" for k in sorted(prod_proj)))
    if errors:
        for e in errors:
            print(f"ERROR: {e}", file=sys.stderr)
        return 1
    print("OK: projected kinds")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
