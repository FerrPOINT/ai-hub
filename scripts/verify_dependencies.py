"""Verify SDK and Namespace/UI cohort separately; never rewrite a pin."""
import argparse
from pathlib import Path
import subprocess


def git(path, *args):
    return subprocess.check_output(["git", "-C", str(path), *args], text=True).strip()


def verify(repo, checkout, pin):
    expected = (repo / pin).read_text(encoding="utf-8").strip()
    if len(expected) != 40 or git(checkout, "rev-parse", "HEAD") != expected:
        raise ValueError(f"Exact {pin} checkout required")
    if git(checkout, "status", "--porcelain", "--untracked-files=all"):
        raise ValueError(f"Dirty {pin} checkout")
    return expected


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--workspace", type=Path, default=Path(__file__).resolve().parents[2])
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[1]
    sdk = args.workspace / ".local/sdk/services-base-875cac2"
    cohort = args.workspace / ".local/sdk/services-base-81decf7"
    print("Base SDK:", verify(repo, sdk, ".base-revision"))
    print("Namespace/UI:", verify(repo, cohort, ".namespace-base-revision"))
