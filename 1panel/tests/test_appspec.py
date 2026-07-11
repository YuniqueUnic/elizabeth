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
        self.assertEqual(service["image"], "yunique001/elizabeth:1.4.0")
        self.assertEqual(self.spec["architectures"], ["amd64", "arm64"])

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
