#!/usr/bin/env python3
import argparse
import re
import sys
from pathlib import Path


VERSION_RE = re.compile(r"^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$")


def replace_exact(path: Path, pattern: str, replacement: str, expected: int = 1) -> bool:
    original = path.read_text(encoding="utf-8")
    updated, count = re.subn(pattern, replacement, original, flags=re.MULTILINE)
    if count != expected:
        raise ValueError(
            f"{path}: expected {expected} replacement(s) for pattern {pattern!r}, found {count}"
        )
    if updated != original:
        path.write_text(updated, encoding="utf-8")
        return True
    return False


def check_exact(path: Path, pattern: str, expected_text: str) -> bool:
    text = path.read_text(encoding="utf-8")
    match = re.search(pattern, text, flags=re.MULTILINE)
    if not match:
        raise ValueError(f"{path}: pattern {pattern!r} not found")
    return match.group(0) == expected_text


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Sync all version-bearing files from the root VERSION file."
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Do not write files; fail if any managed file is out of sync",
    )
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parent.parent
    version_file = repo_root / "VERSION"
    version = version_file.read_text(encoding="utf-8").strip()

    if not VERSION_RE.fullmatch(version):
        print(f"invalid version in {version_file}: {version!r}")
        return 1

    managed = [
        (
            repo_root / "Cargo.toml",
            r'^version = ".*"$',
            f'version = "{version}"',
        ),
        (
            repo_root / "crates/poolsim-cli/Cargo.toml",
            r'^poolsim-core = \{ path = "\.\./poolsim-core", version = ".*" \}$',
            f'poolsim-core = {{ path = "../poolsim-core", version = "{version}" }}',
        ),
        (
            repo_root / "crates/poolsim-web/Cargo.toml",
            r'^poolsim-core = \{ path = "\.\./poolsim-core", version = ".*" \}$',
            f'poolsim-core = {{ path = "../poolsim-core", version = "{version}" }}',
        ),
        (
            repo_root / "crates/poolsim-core/README.md",
            r'^poolsim-core = ".*"$',
            f'poolsim-core = "{version}"',
        ),
        (
            repo_root / "crates/poolsim-core/src/lib.rs",
            r'^#!\[doc\(html_root_url = "https://docs\.rs/poolsim-core/.*"\)\]$',
            f'#![doc(html_root_url = "https://docs.rs/poolsim-core/{version}")]',
        ),
        (
            repo_root / "crates/poolsim-cli/src/main.rs",
            r'^#!\[doc\(html_root_url = "https://docs\.rs/poolsim-cli/.*"\)\]$',
            f'#![doc(html_root_url = "https://docs.rs/poolsim-cli/{version}")]',
        ),
        (
            repo_root / "crates/poolsim-web/src/lib.rs",
            r'^#!\[doc\(html_root_url = "https://docs\.rs/poolsim-web/.*"\)\]$',
            f'#![doc(html_root_url = "https://docs.rs/poolsim-web/{version}")]',
        ),
        (
            repo_root / "docs/web-api.md",
            r'^  "version": ".*"$',
            f'  "version": "{version}"',
        ),
        (
            repo_root / "docs/web-api.md",
            r'^    version: ".*",$',
            f'    version: "{version}",',
            2,
        ),
        (
            repo_root / "docs/web-api.md",
            r'^assert_eq!\(state\.version, ".*"\);$',
            f'assert_eq!(state.version, "{version}");',
        ),
    ]

    try:
        if args.check:
            out_of_sync = []
            for item in managed:
                path, pattern, expected_text, *rest = item
                if not check_exact(path, pattern, expected_text):
                    out_of_sync.append(path)
            if out_of_sync:
                print("version sync check failed:")
                for path in out_of_sync:
                    print(f"  - {path.relative_to(repo_root)}")
                print("run: python3 tools/sync_version.py")
                return 1

            print(f"version sync check passed for {len(managed)} managed patterns at {version}")
            return 0

        changed_files = set()
        for item in managed:
            path, pattern, replacement, *rest = item
            expected = rest[0] if rest else 1
            if replace_exact(path, pattern, replacement, expected):
                changed_files.add(path.relative_to(repo_root).as_posix())

        print(f"synced version {version} from VERSION")
        if changed_files:
            for path in sorted(changed_files):
                print(f"  updated {path}")
        else:
            print("  no files needed changes")
        return 0
    except ValueError as error:
        print(f"version sync failed: {error}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
