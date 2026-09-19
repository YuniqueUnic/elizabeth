#!/usr/bin/env python3
"""Prepare a versioned Elizabeth package for the 1Panel App Store."""

from __future__ import annotations

import argparse
import re
import shutil
from pathlib import Path


IMAGE_RE = re.compile(r"(?m)^(\s*image:\s*)yunique001/elizabeth:[^\s]+\s*$")
DOCUMENT_RE = re.compile(r"(?m)^(\s*document:\s*)(\S+)\s*$")
VERSION_RE = re.compile(r"^[0-9][0-9A-Za-z._+-]*$")

# Source evidence is release-scoped on purpose: it has to keep pointing at the
# exact commit, release, and image the package was cut from, so every version
# reference inside it is repinned on each release.
EVIDENCE_IMAGE_RE = re.compile(r"yunique001/elizabeth:[0-9][0-9A-Za-z._+-]*")
EVIDENCE_BLOB_RE = re.compile(r"/blob/v[0-9][0-9A-Za-z._+-]*/")
EVIDENCE_RELEASE_RE = re.compile(r"/releases/tag/v[0-9][0-9A-Za-z._+-]*")

# The store metadata must not pin a release path. `document` is the link users
# open from the app card, so a version-pinned URL would have to be edited on
# every release and would rot as soon as the tag stops being the tip.
STABLE_DOCUMENT_URL = (
    "https://github.com/YuniqueUnic/elizabeth/blob/main/docs/DOCKER_QUICK_START.md"
)


def normalize_version(raw_version: str) -> str:
    version = raw_version.strip()
    if version.startswith("v"):
        version = version[1:]
    if not VERSION_RE.fullmatch(version):
        raise ValueError(f"invalid release version: {raw_version!r}")
    return version


def version_directories(app_dir: Path) -> list[Path]:
    return sorted(
        path
        for path in app_dir.iterdir()
        if path.is_dir() and (path / "docker-compose.yml").is_file()
    )


def replace_once(path: Path, pattern: re.Pattern[str], replacement: str) -> None:
    original = path.read_text(encoding="utf-8")
    updated, count = pattern.subn(replacement, original)
    if count != 1:
        raise ValueError(f"expected one versioned value in {path}, found {count}")
    path.write_text(updated, encoding="utf-8")


def pin_evidence_versions(path: Path, version: str) -> None:
    """Repin every release-scoped reference inside the source evidence."""
    original = path.read_text(encoding="utf-8")
    updated = EVIDENCE_IMAGE_RE.sub(f"yunique001/elizabeth:{version}", original)
    updated = EVIDENCE_BLOB_RE.sub(f"/blob/v{version}/", updated)
    updated = EVIDENCE_RELEASE_RE.sub(f"/releases/tag/v{version}", updated)
    path.write_text(updated, encoding="utf-8")


def assert_stable_document_url(path: Path) -> None:
    """Fail the release when the store metadata pins `document` to a release path.

    This is a guard rather than a rewrite: silently repointing the link is what
    produced version-pinned metadata in the first place, and the mistake only
    shows up on the next release.
    """
    match = DOCUMENT_RE.search(path.read_text(encoding="utf-8"))
    if match is None:
        raise ValueError(f"missing `document` field in {path}")
    if match.group(2) != STABLE_DOCUMENT_URL:
        raise ValueError(
            f"`document` in {path} must point at the default branch "
            f"({STABLE_DOCUMENT_URL}), found {match.group(2)}"
        )


def prepare_release(source: Path, output_root: Path, raw_version: str) -> Path:
    version = normalize_version(raw_version)
    if not source.is_dir():
        raise FileNotFoundError(f"source app package not found: {source}")

    source_versions = version_directories(source)
    if len(source_versions) != 1:
        raise ValueError(
            f"expected exactly one source version directory in {source}, "
            f"found {len(source_versions)}"
        )

    output_app = output_root / "elizabeth"
    if output_app.exists():
        shutil.rmtree(output_app)
    output_root.mkdir(parents=True, exist_ok=True)
    shutil.copytree(source, output_app)

    current_version_dir = output_app / source_versions[0].name
    target_version_dir = output_app / version
    if current_version_dir != target_version_dir:
        if target_version_dir.exists():
            shutil.rmtree(target_version_dir)
        current_version_dir.rename(target_version_dir)

    compose_path = target_version_dir / "docker-compose.yml"
    replace_once(
        compose_path,
        IMAGE_RE,
        rf"\g<1>yunique001/elizabeth:{version}",
    )
    assert_stable_document_url(output_app / "data.yml")

    evidence_path = output_app / "source-evidence.json"
    if evidence_path.is_file():
        pin_evidence_versions(evidence_path, version)

    return output_app


def main() -> None:
    repo_root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True, help="Release version, with or without a v prefix")
    parser.add_argument(
        "--source",
        type=Path,
        default=repo_root / "1panel" / "apps" / "elizabeth",
        help="Checked-in Elizabeth app package",
    )
    parser.add_argument("--output", type=Path, required=True, help="Output apps directory")
    args = parser.parse_args()

    output_app = prepare_release(args.source.resolve(), args.output.resolve(), args.version)
    print(output_app)


if __name__ == "__main__":
    main()
