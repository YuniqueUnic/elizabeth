from __future__ import annotations

import hashlib
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "prepare_appstore_release.py"
SPEC = importlib.util.spec_from_file_location("prepare_appstore_release", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

LOGO_BYTES = b"\x89PNG\r\n\x1a\n" + b"prepare-appstore-release-test-logo"
LOGO_SHA256 = hashlib.sha256(LOGO_BYTES).hexdigest()


class PrepareAppstoreReleaseTests(unittest.TestCase):
    def create_package(self, root: Path, versions: tuple[str, ...] = ("1.4.0",)) -> Path:
        app = root / "source" / "elizabeth"
        app.mkdir(parents=True)
        (app / "data.yml").write_text(
            f"document: {MODULE.STABLE_DOCUMENT_URL}\n",
            encoding="utf-8",
        )
        (app / "README.md").write_text("Elizabeth\n", encoding="utf-8")
        (app / "logo.png").write_bytes(LOGO_BYTES)
        (app / "source-evidence.json").write_text(
            "{\n"
            '  "image": "yunique001/elizabeth:1.4.0",\n'
            '  "dockerDocs": "https://github.com/YuniqueUnic/elizabeth/blob/v1.4.0/docs/DOCKER_QUICK_START.md",\n'
            '  "release": "https://github.com/YuniqueUnic/elizabeth/releases/tag/v1.4.0",\n'
            '  "notes": [\n'
            '    "The yunique001/elizabeth:1.3.0 image still carried the findings this release removes."\n'
            "  ],\n"
            '  "images": [\n'
            "    {\n"
            '      "version": "1.4.0",\n'
            '      "service": "elizabeth",\n'
            '      "reference": "yunique001/elizabeth:1.4.0",\n'
            f'      "digest": "sha256:{"a" * 64}"\n'
            "    }\n"
            "  ],\n"
            '  "logoEvidence": {\n'
            '    "source": "bundled:logo.png",\n'
            '    "license": "AGPL-3.0-only",\n'
            f'    "sha256": "{LOGO_SHA256}"\n'
            "  },\n"
            '  "redistributionEvidence": {\n'
            '    "status": "verified",\n'
            '    "assets": [\n'
            "      {\n"
            '        "path": "logo.png",\n'
            '        "source": "bundled:logo.png",\n'
            f'        "sha256": "{LOGO_SHA256}"\n'
            "      }\n"
            "    ]\n"
            "  }\n"
            "}\n",
            encoding="utf-8",
        )
        for version in versions:
            version_dir = app / version
            version_dir.mkdir()
            (version_dir / "docker-compose.yml").write_text(
                "services:\n  elizabeth:\n    image: yunique001/elizabeth:1.4.0\n",
                encoding="utf-8",
            )
        return app

    def test_normalizes_v_prefix(self) -> None:
        self.assertEqual(MODULE.normalize_version("v1.5.0"), "1.5.0")

    def test_rejects_invalid_version(self) -> None:
        with self.assertRaises(ValueError):
            MODULE.normalize_version("release/1.5.0")

    def test_prepares_new_version_without_mutating_source(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = self.create_package(root)
            output = root / "output"

            prepared = MODULE.prepare_release(source, output, "v1.5.0")

            self.assertTrue((source / "1.4.0" / "docker-compose.yml").exists())
            self.assertFalse((prepared / "1.4.0").exists())
            compose = (prepared / "1.5.0" / "docker-compose.yml").read_text(encoding="utf-8")
            metadata = (prepared / "data.yml").read_text(encoding="utf-8")
            self.assertIn("image: yunique001/elizabeth:1.5.0", compose)
            self.assertIn(MODULE.STABLE_DOCUMENT_URL, metadata)
            self.assertNotIn("/blob/v1.5.0/", metadata)
            evidence = (prepared / "source-evidence.json").read_text(encoding="utf-8")
            self.assertIn("yunique001/elizabeth:1.5.0", evidence)
            self.assertIn("/blob/v1.5.0/docs/DOCKER_QUICK_START.md", evidence)
            self.assertIn("/releases/tag/v1.5.0", evidence)
            self.assertNotIn("1.4.0", evidence)
            # References to an earlier release describe that release, so repinning
            # must leave them alone instead of retagging history.
            self.assertIn("yunique001/elizabeth:1.3.0", evidence)
            # `images[]` is keyed by version directory, so the entry has to follow
            # the release instead of stranding the delivery validator.
            self.assertEqual(
                json.loads(evidence)["images"][0]["version"],
                "1.5.0",
            )

    def test_drops_stale_image_digest_on_version_bump(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = self.create_package(root)

            prepared = MODULE.prepare_release(source, root / "output", "1.5.0")

            image = json.loads(
                (prepared / "source-evidence.json").read_text(encoding="utf-8")
            )["images"][0]
            self.assertEqual(image["version"], "1.5.0")
            # A digest identifies one build, so it must not be carried forward.
            self.assertNotIn("digest", image)

    def test_keeps_image_digest_when_the_version_is_unchanged(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = self.create_package(root)

            prepared = MODULE.prepare_release(source, root / "output", "1.4.0")

            image = json.loads(
                (prepared / "source-evidence.json").read_text(encoding="utf-8")
            )["images"][0]
            self.assertEqual(image["digest"], "sha256:" + "a" * 64)

    def test_rejects_stale_logo_hash(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = self.create_package(root)
            (source / "logo.png").write_bytes(LOGO_BYTES + b"rotated")

            with self.assertRaisesRegex(ValueError, "does not match the shipped logo.png"):
                MODULE.prepare_release(source, root / "output", "1.5.0")

    def test_rejects_missing_logo(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = self.create_package(root)
            (source / "logo.png").unlink()

            with self.assertRaisesRegex(ValueError, "missing package logo"):
                MODULE.prepare_release(source, root / "output", "1.5.0")

    def test_rejects_version_pinned_document_url(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = self.create_package(root)
            (source / "data.yml").write_text(
                "document: https://github.com/YuniqueUnic/elizabeth/blob/v1.4.0/docs/DOCKER_QUICK_START.md\n",
                encoding="utf-8",
            )

            with self.assertRaisesRegex(ValueError, "must point at the default branch"):
                MODULE.prepare_release(source, root / "output", "1.5.0")

    def test_rejects_missing_document_url(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = self.create_package(root)
            (source / "data.yml").write_text("name: Elizabeth\n", encoding="utf-8")

            with self.assertRaisesRegex(ValueError, "missing `document` field"):
                MODULE.prepare_release(source, root / "output", "1.5.0")

    def test_rejects_ambiguous_source_versions(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = self.create_package(root, versions=("1.4.0", "1.4.1"))
            with self.assertRaisesRegex(ValueError, "exactly one source version"):
                MODULE.prepare_release(source, root / "output", "1.5.0")


if __name__ == "__main__":
    unittest.main()
