#!/usr/bin/env python3
"""Prepare a versioned Elizabeth package for the 1Panel App Store."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import sys
from pathlib import Path


IMAGE_RE = re.compile(r"(?m)^(\s*image:\s*)yunique001/elizabeth:[^\s]+\s*$")
DOCUMENT_RE = re.compile(r"(?m)^(\s*document:\s*)(\S+)\s*$")
VERSION_RE = re.compile(r"^[0-9][0-9A-Za-z._+-]*$")

# Source evidence is release-scoped on purpose: it has to keep pointing at the
# exact commit, release, and image the package was cut from, so the references
# to the version it was cut from are repinned on each release. Only that version
# is rewritten: notes that describe an earlier release, such as the findings
# recorded against it, have to keep naming the version they actually describe.
EVIDENCE_IMAGE_TEMPLATE = r"yunique001/elizabeth:{version}(?![\w.+-])"
EVIDENCE_BLOB_TEMPLATE = r"/blob/v{version}/"
EVIDENCE_RELEASE_TEMPLATE = r"/releases/tag/v{version}(?![\w.+-])"

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


def pin_evidence_versions(path: Path, source_version: str, version: str) -> None:
    """Repin the release-scoped references inside the source evidence.

    `images[]` entries are keyed by version directory as well, and leaving that
    field behind makes the delivery validator report the Compose service as
    uncovered.
    """
    token = re.escape(source_version)
    rewrites = (
        (
            re.compile(EVIDENCE_IMAGE_TEMPLATE.format(version=token)),
            f"yunique001/elizabeth:{version}",
        ),
        (
            re.compile(EVIDENCE_BLOB_TEMPLATE.format(version=token)),
            f"/blob/v{version}/",
        ),
        (
            re.compile(EVIDENCE_RELEASE_TEMPLATE.format(version=token)),
            f"/releases/tag/v{version}",
        ),
    )

    def repin(value: object) -> object:
        if isinstance(value, str):
            for pattern, replacement in rewrites:
                value = pattern.sub(replacement, value)
            return value
        if isinstance(value, list):
            return [repin(item) for item in value]
        if isinstance(value, dict):
            return {key: repin(item) for key, item in value.items()}
        return value

    payload = repin(json.loads(path.read_text(encoding="utf-8")))
    images = payload.get("images") if isinstance(payload, dict) else None
    dropped: list[str] = []
    if isinstance(images, list):
        for image in images:
            if not isinstance(image, dict) or "version" not in image:
                continue
            if image["version"] != version:
                # A digest identifies one image build, so it cannot follow the
                # release. Drop it and say so rather than attributing the
                # previous build's hash to this one; the delivery validator then
                # reports the missing digest instead of passing a stale hash.
                if image.pop("digest", None) is not None:
                    dropped.append(str(image.get("service", "?")))
            image["version"] = version
    path.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    if dropped:
        print(
            f"notice: dropped the stale image digest for {version} "
            f"(service {', '.join(dropped)}); record the digest of the "
            f"published {version} image before submitting with source evidence",
            file=sys.stderr,
        )


def assert_logo_evidence_hashes(path: Path, package_root: Path) -> None:
    """Fail the release when the recorded logo hash stops matching the shipped file.

    The hash binds `logoEvidence` and the `logo.png` redistribution asset to the
    delivered artwork. Nothing regenerates it, so without this guard replacing
    the logo would silently ship stale redistribution evidence.
    """
    logo = package_root / "logo.png"
    if not logo.is_file():
        raise ValueError(f"missing package logo: {logo}")
    actual = hashlib.sha256(logo.read_bytes()).hexdigest()
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"source evidence must be a JSON object: {path}")

    recorded: list[tuple[str, object]] = []
    logo_evidence = payload.get("logoEvidence")
    if isinstance(logo_evidence, dict):
        recorded.append(("logoEvidence.sha256", logo_evidence.get("sha256")))
    assets = payload.get("redistributionEvidence", {})
    if isinstance(assets, dict) and isinstance(assets.get("assets"), list):
        for index, asset in enumerate(assets["assets"]):
            if isinstance(asset, dict) and asset.get("path") == "logo.png":
                recorded.append(
                    (
                        f"redistributionEvidence.assets[{index}].sha256",
                        asset.get("sha256"),
                    )
                )

    for field, value in recorded:
        if not isinstance(value, str) or value.lower() != actual:
            raise ValueError(
                f"{field} does not match the shipped logo.png ({actual})"
            )


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
        pin_evidence_versions(evidence_path, source_versions[0].name, version)
        assert_logo_evidence_hashes(evidence_path, output_app)

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
