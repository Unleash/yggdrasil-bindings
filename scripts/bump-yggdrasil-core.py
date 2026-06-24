#!/usr/bin/env python3
"""Bump the Yggdrasil core version and patch wrapper package versions."""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SEMVER_RE = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$")


@dataclass(frozen=True)
class Replacement:
    path: Path
    pattern: str
    replacement: str
    label: str


@dataclass(frozen=True)
class VersionField:
    path: Path
    pattern: str
    label: str


CORE_FIELDS = [
    VersionField(
        Path("yggdrasilffi/Cargo.toml"),
        r'(unleash-yggdrasil\s*=\s*\{\s*version\s*=\s*")([^"]+)(")',
        "Rust FFI core",
    ),
    VersionField(
        Path("pure-wasm/Cargo.toml"),
        r'(unleash-yggdrasil\s*=\s*\{\s*version\s*=\s*")([^"]+)(")',
        "Pure WASM core",
    ),
    VersionField(
        Path("yggdrasilwasm/Cargo.toml"),
        r'(unleash-yggdrasil\s*=\s*\{\s*version\s*=\s*")([^"]+)(")',
        "WASM core",
    ),
    VersionField(
        Path("java-engine/gradle.properties"),
        r"^(yggdrasilCoreVersion=)(.+)()$",
        "Java core",
    ),
    VersionField(
        Path("python-engine/yggdrasil_engine/__init__.py"),
        r'^(__yggdrasil_core_version__\s*=\s*")([^"]+)(")$',
        "Python core",
    ),
    VersionField(
        Path("ruby-engine/yggdrasil-engine.gemspec"),
        r'^(  s\.metadata\["yggdrasil_core_version"\]\s*=\s*\')([^\']+)(\')$',
        "Ruby core",
    ),
    VersionField(
        Path("dotnet-engine/Yggdrasil.Engine/Yggdrasil.Engine.csproj"),
        r"^(\s*<YggdrasilCoreVersion>)([^<]+)(</YggdrasilCoreVersion>)$",
        ".NET core",
    ),
]


PACKAGE_FIELDS = [
    VersionField(
        Path("java-engine/gradle.properties"),
        r"^(version=)(.+)()$",
        "Java package",
    ),
    VersionField(
        Path("python-engine/pyproject.toml"),
        r'^(version\s*=\s*")([^"]+)(")$',
        "Python package",
    ),
    VersionField(
        Path("ruby-engine/yggdrasil-engine.gemspec"),
        r"^(  s\.version\s*=\s*')([^']+)(')$",
        "Ruby package",
    ),
    VersionField(
        Path("dotnet-engine/Yggdrasil.Engine/Yggdrasil.Engine.csproj"),
        r"^(\s*<Version>)([^<]+)(</Version>)$",
        ".NET package",
    ),
]


def parse_semver(version: str) -> tuple[int, int, int]:
    match = SEMVER_RE.match(version)
    if not match:
        raise ValueError(f"expected x.y.z semver, got {version!r}")
    return tuple(int(part) for part in match.groups())


def patch_bump(version: str) -> str:
    major, minor, patch = parse_semver(version)
    return f"{major}.{minor}.{patch + 1}"


def read_text(path: Path) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def extract_version(field: VersionField) -> str:
    text = read_text(field.path)
    matches = list(re.finditer(field.pattern, text, flags=re.MULTILINE))
    if len(matches) != 1:
        raise RuntimeError(
            f"{field.label}: expected exactly one match in {field.path}, found {len(matches)}"
        )
    return matches[0].group(2)


def build_replacement(field: VersionField, new_version: str) -> Replacement:
    return Replacement(
        path=field.path,
        pattern=field.pattern,
        replacement=rf"\g<1>{new_version}\g<3>",
        label=field.label,
    )


def apply_replacements(replacements: list[Replacement]) -> None:
    by_path: dict[Path, list[Replacement]] = {}
    for replacement in replacements:
        by_path.setdefault(replacement.path, []).append(replacement)

    for path, path_replacements in by_path.items():
        full_path = ROOT / path
        text = full_path.read_text(encoding="utf-8")
        for replacement in path_replacements:
            text, count = re.subn(
                replacement.pattern,
                replacement.replacement,
                text,
                count=1,
                flags=re.MULTILINE,
            )
            if count != 1:
                raise RuntimeError(f"{replacement.label}: replacement failed in {path}")
        full_path.write_text(text, encoding="utf-8")


def plan(target_core_version: str) -> list[tuple[str, str, str]]:
    parse_semver(target_core_version)

    core_versions = {field.label: extract_version(field) for field in CORE_FIELDS}
    for label, version in core_versions.items():
        parse_semver(version)

    current_core_version = max(core_versions.values(), key=parse_semver)
    if parse_semver(target_core_version) <= parse_semver(current_core_version):
        raise ValueError(
            f"target core version {target_core_version} must be greater than "
            f"current highest core version {current_core_version}"
        )

    changes: list[tuple[str, str, str]] = []
    changes.extend(
        (field.label, core_versions[field.label], target_core_version)
        for field in CORE_FIELDS
    )

    for field in PACKAGE_FIELDS:
        current = extract_version(field)
        changes.append((field.label, current, patch_bump(current)))

    return changes


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Update Yggdrasil core pins and patch-bump Java, Python, Ruby, and .NET packages."
        )
    )
    parser.add_argument("core_version", help="New unleash-yggdrasil core version, e.g. 0.21.3")
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print the planned changes without modifying files.",
    )
    args = parser.parse_args()

    try:
        changes = plan(args.core_version)
    except (RuntimeError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    for label, old, new in changes:
        print(f"{label}: {old} -> {new}")

    if args.dry_run:
        return 0

    replacements = [
        build_replacement(field, args.core_version)
        for field in CORE_FIELDS
    ]
    replacements.extend(
        build_replacement(field, patch_bump(extract_version(field)))
        for field in PACKAGE_FIELDS
    )

    try:
        apply_replacements(replacements)
    except RuntimeError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
