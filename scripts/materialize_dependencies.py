"""Materialize clean exact dependency sources without touching sibling dev HEAD."""
from pathlib import Path
import argparse
import subprocess
import sys

from verify_dependencies import verify


def run(*args):
    subprocess.run(["git", *map(str, args)], check=True)


def materialize(workspace):
    repo = Path(__file__).resolve().parents[1]
    source = workspace / "services-base"
    if not (source / ".git").exists():
        raise ValueError("Materialized services-base sibling required; no implicit remote credential setup")
    dependencies = [
        ("services-base-875cac2", ".base-revision"),
        ("services-base-81decf7", ".namespace-base-revision"),
    ]
    sdk_root = workspace / ".local" / "sdk"
    sdk_root.mkdir(parents=True, exist_ok=True)
    for name, pin in dependencies:
        checkout = sdk_root / name
        expected = (repo / pin).read_text(encoding="utf-8").strip()
        if not checkout.exists():
            # Check local source object first; remote refs are refreshed deliberately by owner.
            run("-C", source, "cat-file", "-e", f"{expected}^{{commit}}")
            run("-C", source, "worktree", "add", "--detach", checkout, expected)
        print(pin, verify(repo, checkout, pin))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--workspace", type=Path, default=Path(__file__).resolve().parents[2])
    args = parser.parse_args()
    try:
        materialize(args.workspace.resolve())
    except (ValueError, subprocess.CalledProcessError) as error:
        print(f"Dependency materialization refused: {error}", file=sys.stderr)
        raise SystemExit(1)
