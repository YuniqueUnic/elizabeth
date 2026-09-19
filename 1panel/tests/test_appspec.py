from __future__ import annotations

import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class AppSpecTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.spec = json.loads((ROOT / "elizabeth-1panel-appspec.json").read_text(encoding="utf-8"))

    def test_release_image_and_architectures_are_pinned(self) -> None:
        service = self.spec["services"][0]
        self.assertEqual(service["image"], f"yunique001/elizabeth:{self.spec['version']}")
        self.assertEqual(self.spec["architectures"], ["amd64", "arm64"])

    def test_document_link_stays_on_the_default_branch(self) -> None:
        # A version-pinned `document` would have to be edited on every release.
        self.assertEqual(
            self.spec["document"],
            "https://github.com/YuniqueUnic/elizabeth/blob/main/docs/DOCKER_QUICK_START.md",
        )

    def test_service_runs_as_non_root_and_probes_without_curl(self) -> None:
        service = self.spec["services"][0]
        self.assertEqual(service["user"], "65532:65532")
        self.assertEqual(
            service["healthcheck"]["test"],
            ["CMD", "/app/board", "health", "--config-file", "/app/config/backend.yaml"],
        )

    def test_generated_jwt_secret_meets_backend_minimum(self) -> None:
        field = next(item for item in self.spec["form_fields"] if item["envKey"] == "JWT_SECRET")
        self.assertTrue(field["random"])
        self.assertEqual(field["type"], "password")
        # 1Panel appends "_" and six random characters when random=true.
        self.assertGreaterEqual(len(field["default"]) + 7, 32)

    def test_logo_is_portable_and_matches_store_limits(self) -> None:
        self.assertEqual(self.spec["logo"], "1panel/logo.png")
        payload = (ROOT / "logo.png").read_bytes()
        self.assertLessEqual(len(payload), 10 * 1024)
        self.assertEqual(payload[:8], b"\x89PNG\r\n\x1a\n")


if __name__ == "__main__":
    unittest.main()
