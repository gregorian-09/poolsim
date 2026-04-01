#!/usr/bin/env python3
import argparse
import re
import sys
from pathlib import Path


CRATES = {
    "poolsim_core": Path("crates/poolsim-core/src"),
    "poolsim_web": Path("crates/poolsim-web/src"),
    "poolsim_cli": Path("crates/poolsim-cli/src"),
}

PUB_MOD_RE = re.compile(r"^\s*pub\s+mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;")
PUB_USE_RE = re.compile(r"^\s*pub\s+use\s+[^;]+::([A-Za-z_][A-Za-z0-9_]*)\s*;")
PUB_CONST_RE = re.compile(r"^\s*pub\s+const\s+([A-Z_][A-Z0-9_]*)\b")
PUB_STRUCT_RE = re.compile(r"^\s*pub\s+struct\s+([A-Za-z_][A-Za-z0-9_]*)\b")
PUB_ENUM_RE = re.compile(r"^\s*pub\s+enum\s+([A-Za-z_][A-Za-z0-9_]*)\b")
IMPL_RE = re.compile(r"^\s*impl\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{")
PUB_FN_RE = re.compile(r"^\s*pub\s+(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\b")


def module_path(source_root: Path, source_file: Path) -> list[str]:
    rel = source_file.relative_to(source_root)
    parts = list(rel.parts)

    if parts[-1] in {"lib.rs", "main.rs"}:
        return []

    parts[-1] = parts[-1][:-3]
    if parts[-1] == "mod":
        parts = parts[:-1]
    return parts


def inventory_public_symbols(crate_name: str, source_root: Path) -> set[str]:
    symbols: set[str] = set()

    for source_file in sorted(source_root.rglob("*.rs")):
        mod = module_path(source_root, source_file)
        depth = 0
        current_impl = None
        impl_depth = None

        for line in source_file.read_text(encoding="utf-8").splitlines():
            if match := PUB_MOD_RE.match(line):
                symbols.add("::".join([crate_name, *mod, match.group(1)]))

            if not mod and (match := PUB_USE_RE.match(line)):
                symbols.add("::".join([crate_name, match.group(1)]))

            if match := PUB_CONST_RE.match(line):
                symbols.add("::".join([crate_name, *mod, match.group(1)]))

            if match := PUB_STRUCT_RE.match(line):
                symbols.add("::".join([crate_name, *mod, match.group(1)]))

            if match := PUB_ENUM_RE.match(line):
                symbols.add("::".join([crate_name, *mod, match.group(1)]))

            if match := IMPL_RE.match(line):
                current_impl = match.group(1)
                impl_depth = depth + line.count("{") - line.count("}")

            if match := PUB_FN_RE.match(line):
                if current_impl:
                    symbols.add("::".join([crate_name, *mod, current_impl, match.group(1)]))
                else:
                    symbols.add("::".join([crate_name, *mod, match.group(1)]))

            depth += line.count("{") - line.count("}")
            if current_impl and impl_depth is not None and depth < impl_depth:
                current_impl = None
                impl_depth = None

    return symbols


def load_docs_text(docs_dir: Path, extra_docs: list[Path]) -> str:
    parts = []
    for doc in sorted(docs_dir.rglob("*.md")):
        parts.append(doc.read_text(encoding="utf-8"))
    for doc in extra_docs:
        parts.append(doc.read_text(encoding="utf-8"))
    return "\n".join(parts)


def main() -> int:
    parser = argparse.ArgumentParser(description="Ensure every public API symbol is named in docs/.")
    parser.add_argument("--docs-dir", default="docs", help="Path to the docs directory")
    parser.add_argument(
        "--inventory-file",
        default="tools/public-api-index.md",
        help="Path to the maintainer-facing API inventory markdown file",
    )
    parser.add_argument(
        "--print-symbols",
        action="store_true",
        help="Print the discovered public symbol inventory and exit",
    )
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parent.parent
    docs_dir = (repo_root / args.docs_dir).resolve()
    if not docs_dir.exists():
        print(f"docs directory not found: {docs_dir}")
        return 1
    inventory_file = (repo_root / args.inventory_file).resolve()
    if not inventory_file.exists():
        print(f"inventory file not found: {inventory_file}")
        return 1

    symbols: set[str] = set()
    for crate_name, relative_root in CRATES.items():
        symbols.update(inventory_public_symbols(crate_name, repo_root / relative_root))

    ordered_symbols = sorted(symbols)
    if args.print_symbols:
        for symbol in ordered_symbols:
            print(symbol)
        return 0

    docs_text = load_docs_text(docs_dir, [inventory_file])
    missing = [symbol for symbol in ordered_symbols if f"`{symbol}`" not in docs_text]

    if missing:
        print("docs API coverage failed; missing symbols:")
        for symbol in missing:
            print(f"  - {symbol}")
        print(f"missing {len(missing)} of {len(ordered_symbols)} public symbols")
        return 1

    print(
        "docs API coverage passed: "
        f"{len(ordered_symbols)} public symbols found in user docs and maintainer inventory"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
