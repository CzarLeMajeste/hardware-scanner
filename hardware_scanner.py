#!/usr/bin/env python3
"""Scan host hardware and recommend compatible operating systems."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import platform
import shutil
import subprocess
from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class OSProfile:
    name: str
    min_ram_gb: float
    min_storage_gb: float
    min_cores: int
    requires_tpm: bool = False
    requires_64bit: bool = True
    notes: str = ""


OS_PROFILES: tuple[OSProfile, ...] = (
    OSProfile(
        name="Windows 11",
        min_ram_gb=4,
        min_storage_gb=64,
        min_cores=2,
        requires_tpm=True,
        notes="Best on modern hardware with TPM 2.0 and secure firmware setup.",
    ),
    OSProfile(
        name="Ubuntu 24.04 LTS",
        min_ram_gb=4,
        min_storage_gb=25,
        min_cores=2,
        notes="Balanced for workstations and development environments.",
    ),
    OSProfile(
        name="Fedora Workstation 40",
        min_ram_gb=4,
        min_storage_gb=20,
        min_cores=2,
        notes="Great for up-to-date Linux desktop workflows.",
    ),
    OSProfile(
        name="Debian 12",
        min_ram_gb=2,
        min_storage_gb=10,
        min_cores=1,
        notes="Stable option that works well on lower-spec hardware.",
    ),
)


def _run_command(command: list[str]) -> str:
    try:
        completed = subprocess.run(
            command,
            check=False,
            capture_output=True,
            text=True,
            timeout=5,
        )
    except (subprocess.SubprocessError, OSError):
        return ""
    return completed.stdout.strip()


def _read_memtotal_kb() -> int:
    try:
        with open("/proc/meminfo", "r", encoding="utf-8") as file:
            for line in file:
                if line.startswith("MemTotal:"):
                    parts = line.split()
                    return int(parts[1])
    except (OSError, ValueError, IndexError):
        return 0
    return 0


def _detect_gpu() -> str:
    if shutil.which("lspci"):
        output = _run_command(["lspci"])
        for line in output.splitlines():
            if "VGA compatible controller" in line or "3D controller" in line:
                return line.split(": ", maxsplit=1)[-1]
    return "Unknown"


def scan_hardware() -> dict[str, Any]:
    architecture = platform.machine() or "Unknown"
    cpu = _run_command(["bash", "-lc", "grep -m1 'model name' /proc/cpuinfo | cut -d: -f2-"]).strip()
    if not cpu:
        cpu = platform.processor() or "Unknown"

    mem_kb = _read_memtotal_kb()
    memory_gb = round(mem_kb / (1024 * 1024), 2) if mem_kb else 0.0

    disk = shutil.disk_usage("/")
    storage_gb = round(disk.total / (1024**3), 2)

    return {
        "cpu_model": cpu.strip(),
        "cpu_cores": os.cpu_count() or 1,
        "memory_gb": memory_gb,
        "storage_gb": storage_gb,
        "architecture": architecture,
        "has_tpm": os.path.exists("/dev/tpm0"),
        "boot_mode": "UEFI" if os.path.exists("/sys/firmware/efi") else "Legacy BIOS",
        "gpu": _detect_gpu(),
    }


def _compatibility_category(score: int) -> str:
    if score >= 85:
        return "Excellent"
    if score >= 70:
        return "Good"
    if score >= 50:
        return "Fair"
    return "Low"


def _score_profile(hardware: dict[str, Any], profile: OSProfile) -> tuple[int, list[str], list[str]]:
    score = 100
    issues: list[str] = []
    improvements: list[str] = []

    if hardware["memory_gb"] < profile.min_ram_gb:
        deficit = round(profile.min_ram_gb - hardware["memory_gb"], 2)
        score -= 30
        issues.append(f"RAM below minimum ({hardware['memory_gb']} GB < {profile.min_ram_gb} GB)")
        improvements.append(f"Upgrade RAM by at least {deficit} GB.")
    elif hardware["memory_gb"] < profile.min_ram_gb + 2:
        score -= 8
        improvements.append("Add extra RAM headroom for smoother multitasking.")

    if hardware["storage_gb"] < profile.min_storage_gb:
        deficit = round(profile.min_storage_gb - hardware["storage_gb"], 2)
        score -= 25
        issues.append(
            f"Storage below minimum ({hardware['storage_gb']} GB < {profile.min_storage_gb} GB)"
        )
        improvements.append(f"Increase storage capacity by at least {deficit} GB.")
    elif hardware["storage_gb"] < profile.min_storage_gb + 30:
        score -= 5
        improvements.append("Reserve extra free storage for updates and application caches.")

    if hardware["cpu_cores"] < profile.min_cores:
        score -= 20
        issues.append(f"CPU cores below minimum ({hardware['cpu_cores']} < {profile.min_cores})")
        improvements.append("Use a CPU with more cores for better responsiveness.")

    is_64bit = "64" in str(hardware["architecture"])
    if profile.requires_64bit and not is_64bit:
        score -= 50
        issues.append("64-bit architecture required.")
        improvements.append("Move to a 64-bit capable processor and firmware.")

    if profile.requires_tpm and not hardware["has_tpm"]:
        score -= 20
        issues.append("TPM is required but not detected.")
        improvements.append("Enable TPM 2.0 in firmware or install a TPM module.")

    if hardware["boot_mode"] != "UEFI":
        score -= 5
        improvements.append("Switch to UEFI boot mode for broader modern OS support.")

    if not improvements:
        improvements.append("No major upgrades required; current hardware is already well-matched.")

    return max(score, 0), issues, improvements


def recommend_os(hardware: dict[str, Any]) -> list[dict[str, Any]]:
    matches: list[dict[str, Any]] = []
    for profile in OS_PROFILES:
        score, issues, improvements = _score_profile(hardware, profile)
        matches.append(
            {
                "os": profile.name,
                "compatibility_score": score,
                "compatibility_category": _compatibility_category(score),
                "assessment": "Compatible" if score >= 70 else "Potential compatibility constraints",
                "compatibility_notes": issues or ["All key minimum requirements are met."],
                "improvements": improvements,
                "profile_notes": profile.notes,
            }
        )
    matches.sort(key=lambda item: item["compatibility_score"], reverse=True)
    return matches[:4]


def generate_report() -> dict[str, Any]:
    hardware = scan_hardware()
    return {
        "scanned_at": dt.datetime.now(tz=dt.timezone.utc).isoformat(),
        "hardware": hardware,
        "os_matches": recommend_os(hardware),
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description=(
            "Scan current hardware and produce JSON with top 4 OS compatibility matches "
            "and detailed improvement recommendations."
        )
    )
    parser.add_argument("--compact", action="store_true", help="Output compact JSON without indentation.")
    args = parser.parse_args()

    report = generate_report()
    if args.compact:
        print(json.dumps(report))
        return
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
