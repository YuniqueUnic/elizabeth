from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "prepare_appstore_release.py"
SPEC = importlib.util.spec_from_file_location("prepare_appstore_release", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class PrepareAppstoreReleaseTests(unittest.TestCase):
    def create_package(self, root: Path, versions: tuple[str, ...] = ("1.4.0",)) -> Path:
        app = root / "source" / "elizabeth"
        app.mkdir(parents=True)
        (app / "data.yml").write_text(
            "document: https://github.com/YuniqueUnic/elizabeth/blob/v1.4.0/docs/DOCKER_QUICK_START.md\n",
            encoding="utf-8",
        )
        (app / "README.md").write_text("Elizabeth\n", encoding="utf-8")
        (app / "source-evidence.json").write_text(
            "{\n"
            '  "dockerDocs": "https://github.com/YuniqueUnic/elizabeth/blob/v1.4.0/docs/DOCKER_QUICK_START.md",\n'
            '  "release": "https://github.com/YuniqueUnic/elizabeth/releases/tag/v1.4.0"\n'
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
            self.assertIn("/blob/v1.5.0/docs/DOCKER_QUICK_START.md", metadata)
            evidence = (prepared / "source-evidence.json").read_text(encoding="utf-8")
            self.assertIn("/blob/v1.5.0/docs/DOCKER_QUICK_START.md", evidence)
            self.assertIn("/releases/tag/v1.5.0", evidence)

    def test_rejects_ambiguous_source_versions(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = self.create_package(root, versions=("1.4.0", "1.4.1"))
            with self.assertRaisesRegex(ValueError, "exactly one source version"):
                MODULE.prepare_release(source, root / "output", "1.5.0")


if __name__ == "__main__":
    unittest.main()
