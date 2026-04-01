#!/usr/bin/env python3
import argparse
import re
import sys
from pathlib import Path


LINK_RE = re.compile(r"\[[^\]]+\]\(([^)]+)\)")


def iter_markdown_links(text: str) -> list[str]:
    return [match.group(1).strip() for match in LINK_RE.finditer(text)]


def is_external_link(target: str) -> bool:
    return target.startswith(("http://", "https://", "mailto:", "#"))


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate docs/ markdown structure and local links.")
    parser.add_argument("--docs-dir", default="docs", help="Path to the docs directory")
    args = parser.parse_args()

    docs_dir = Path(args.docs_dir)
    if not docs_dir.exists() or not docs_dir.is_dir():
        print(f"docs directory not found: {docs_dir}")
        return 1

    markdown_files = sorted(docs_dir.rglob("*.md"))
    if not markdown_files:
        print(f"no markdown files found in docs directory: {docs_dir}")
        return 1

    readme = docs_dir / "README.md"
    if readme not in markdown_files:
        print("docs validation failed:")
        print("  - missing docs/README.md")
        return 1

    failures: list[str] = []
    readme_text = readme.read_text(encoding="utf-8")

    for md_file in markdown_files:
        text = md_file.read_text(encoding="utf-8")
        if not text.strip():
            failures.append(f"{md_file} is empty")
            continue

        first_non_empty = next((line.strip() for line in text.splitlines() if line.strip()), "")
        if not first_non_empty.startswith("# "):
            failures.append(f"{md_file} must start with a level-1 markdown heading")

        rel = md_file.relative_to(docs_dir).as_posix()
        if md_file != readme and rel not in readme_text:
            failures.append(f"{rel} is not referenced from docs/README.md")

        for target in iter_markdown_links(text):
            if is_external_link(target):
                continue
            target_path = (md_file.parent / target).resolve()
            if not target_path.exists():
                failures.append(f"{md_file}: broken local link target '{target}'")

    if failures:
        print("docs validation failed:")
        for failure in failures:
            print(f"  - {failure}")
        return 1

    print(f"docs validation passed: {len(markdown_files)} markdown files checked in {docs_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
