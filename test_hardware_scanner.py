import unittest

from hardware_scanner import recommend_os


class HardwareScannerTests(unittest.TestCase):
    def test_returns_top_four_sorted_matches(self):
        hardware = {
            "cpu_model": "Example CPU",
            "cpu_cores": 8,
            "memory_gb": 16.0,
            "storage_gb": 512.0,
            "architecture": "x86_64",
            "has_tpm": True,
            "boot_mode": "UEFI",
            "gpu": "Example GPU",
        }

        matches = recommend_os(hardware)

        self.assertEqual(len(matches), 4)
        scores = [item["compatibility_score"] for item in matches]
        self.assertEqual(scores, sorted(scores, reverse=True))

    def test_missing_requirements_generate_improvements(self):
        hardware = {
            "cpu_model": "Legacy CPU",
            "cpu_cores": 1,
            "memory_gb": 2.0,
            "storage_gb": 20.0,
            "architecture": "x86",
            "has_tpm": False,
            "boot_mode": "Legacy BIOS",
            "gpu": "Unknown",
        }

        matches = recommend_os(hardware)
        windows_match = next(item for item in matches if item["os"] == "Windows 11")

        self.assertIn("Low", windows_match["compatibility_category"])
        self.assertTrue(any("TPM" in rec for rec in windows_match["improvements"]))
        self.assertTrue(any("64-bit" in rec for rec in windows_match["improvements"]))


if __name__ == "__main__":
    unittest.main()
