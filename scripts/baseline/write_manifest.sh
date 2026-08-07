#!/usr/bin/env bash
# Write baseline/6.15/manifest.json provenance (BASE-001).
set -euo pipefail
# shellcheck source=common.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

export MOD_PATH="$(cat "$BASELINE_DIR/oracle-module-path.txt" 2>/dev/null || true)"
export ARCHIVE_SHA="$(cat "$BASELINE_DIR/oracle-archive.sha256" 2>/dev/null || true)"
export COMMIT="$(cat "$BASELINE_DIR/oracle-commit.txt" 2>/dev/null || true)"
export TAG="$(cat "$BASELINE_DIR/oracle-tag.txt" 2>/dev/null || true)"
export PERL5LIB_SAVED="$(cat "$BASELINE_DIR/oracle-perl5lib.txt" 2>/dev/null || true)"

python3 - <<'PY'
import json, os, platform, subprocess, hashlib
from pathlib import Path
from datetime import datetime, timezone

root = Path(os.environ["NYTPROF_MOD_ROOT"])
baseline = Path(os.environ["BASELINE_DIR"])
install = Path(os.environ["INSTALL_DIR"])
src = Path(os.environ["SRC_DIR"])

def run(cmd):
    try:
        return subprocess.check_output(cmd, text=True, stderr=subprocess.STDOUT).strip()
    except Exception as e:
        return f"<error: {e}>"

perl_v = run(["perl", "-V"])
perl_version = run(["perl", "-e", "print $^V"])
gcc_out = run(["gcc", "--version"])
gcc_v = gcc_out.splitlines()[0] if gcc_out and not gcc_out.startswith("<error") else gcc_out
zlib_h = Path("/usr/include/zlib.h")
zlib_note = ""
if zlib_h.exists():
    for line in zlib_h.read_text(errors="replace").splitlines():
        if "ZLIB_VERSION" in line:
            zlib_note = line.strip()
            break

mod_path = os.environ.get("MOD_PATH", "")
archive_sha = os.environ.get("ARCHIVE_SHA", "")
commit = os.environ.get("COMMIT", "")
tag = os.environ.get("TAG", "")
perl5lib = os.environ.get("PERL5LIB_SAVED", "")

artifact_hashes = {}
if install.is_dir():
    for p in install.rglob("*"):
        if not p.is_file():
            continue
        interesting = (
            p.suffix in {".pm", ".so", ".bs"}
            or p.name in {"nytprofhtml", "nytprofcalls", "nytprofmerge", "nytprofcsv", "nytprofcg"}
        )
        if interesting:
            rel = str(p.relative_to(install))
            artifact_hashes[rel] = hashlib.sha256(p.read_bytes()).hexdigest()

candidate_roots = [str(root / "crates"), str(root / "perl")]
contamination = any(c and c in (mod_path or "") for c in candidate_roots)

def rel_or_abs(p: Path) -> str:
    try:
        return str(p.relative_to(root))
    except ValueError:
        return str(p)

manifest = {
    "schema_version": 1,
    "task": "BASE-001",
    "created_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "oracle": {
        "distribution": "Devel-NYTProf",
        "version": "6.15",
        "tag": tag,
        "commit": commit,
        "archive_url_primary": os.environ.get("ORACLE_TARBALL_URL"),
        "archive_url_fallback": os.environ.get("ORACLE_GITHUB_ARCHIVE_URL"),
        "archive_sha256": archive_sha,
        "source_dir": rel_or_abs(src),
        "install_dir": rel_or_abs(install),
    },
    "environment": {
        "os": platform.platform(),
        "uname": run(["uname", "-a"]),
        "perl_version": perl_version,
        "perl_V": perl_v,
        "compiler": gcc_v,
        "zlib_header": zlib_note,
        "cwd_root": str(root),
    },
    "isolation": {
        "perl5lib": perl5lib,
        "module_path": mod_path,
        "loads_from_install_tree": bool(mod_path and str(install) in mod_path),
        "candidate_contamination": contamination,
        "candidate_roots_checked": candidate_roots,
    },
    "artifacts_sha256": artifact_hashes,
    "logs": {
        "build": "baseline/6.15/logs/build_oracle.log",
        "test": "baseline/6.15/logs/test_oracle.log",
    },
    "rebuild": {
        "fetch": "scripts/baseline/fetch_oracle.sh",
        "build": "scripts/baseline/build_oracle.sh",
        "test": "scripts/baseline/test_oracle.sh",
        "manifest": "scripts/baseline/write_manifest.sh",
        "all": "scripts/baseline/run_all.sh",
    },
}

out = baseline / "manifest.json"
out.write_text(json.dumps(manifest, indent=2) + "\n")
print(f"Wrote {out}")
if contamination or not manifest["isolation"]["loads_from_install_tree"]:
    raise SystemExit(
        f"Manifest isolation check failed: contamination={contamination} "
        f"loads_from_install={manifest['isolation']['loads_from_install_tree']} "
        f"module_path={mod_path!r}"
    )
print("Isolation OK: oracle module path is under install tree")
PY
