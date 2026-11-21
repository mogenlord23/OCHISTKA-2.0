"""Offline regressions for the NetZapret resource importer (no network or driver)."""
import importlib.util
import pathlib
import unittest

SCRIPT = pathlib.Path(__file__).with_name("preset_runtime_assets.py")
spec = importlib.util.spec_from_file_location("preset_runtime_assets", SCRIPT)
asset = importlib.util.module_from_spec(spec)
spec.loader.exec_module(asset)


class PresetImportTests(unittest.TestCase):
    def test_real_upstream_resource_map_is_complete(self):
        inventory = asset.read_map()
        self.assertEqual(len(inventory["files"]), 135)
        self.assertEqual(len(asset.referenced_resources(asset.PRESET.read_text(encoding="utf-8"))), 135)

    def test_git_blob_fingerprint_uses_raw_file_bytes(self):
        self.assertEqual(
            asset.git_blob_sha1(b"test\n"),
            "9daeafb9864cf43055ae93beb0afd6c7d144bfa4",
        )

    def test_rejects_unsafe_paths(self):
        for value in ("../hidden", "lua/../../winws2.exe", "C:/Windows", "/tmp/pass",
                      "lua\\evil.lua", "lists//discord.txt", "bin/file:stream"):
            with self.subTest(path=value):
                with self.assertRaises(ValueError):
                    asset.safe_parts(value)


if __name__ == "__main__":
    unittest.main()
