use serde::Serialize;
use std::cmp::Reverse;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Clone, Copy)]
struct OSProfile {
    name: &'static str,
    min_ram_gb: f64,
    min_storage_gb: f64,
    min_cores: usize,
    requires_tpm: bool,
    requires_64bit: bool,
    notes: &'static str,
}

const OS_PROFILES: [OSProfile; 4] = [
    OSProfile {
        name: "Windows 11",
        min_ram_gb: 4.0,
        min_storage_gb: 64.0,
        min_cores: 2,
        requires_tpm: true,
        requires_64bit: true,
        notes: "Best on modern hardware with TPM 2.0 and secure firmware setup.",
    },
    OSProfile {
        name: "Ubuntu 24.04 LTS",
        min_ram_gb: 4.0,
        min_storage_gb: 25.0,
        min_cores: 2,
        requires_tpm: false,
        requires_64bit: true,
        notes: "Balanced for workstations and development environments.",
    },
    OSProfile {
        name: "Fedora Workstation 40",
        min_ram_gb: 4.0,
        min_storage_gb: 20.0,
        min_cores: 2,
        requires_tpm: false,
        requires_64bit: true,
        notes: "Great for up-to-date Linux desktop workflows.",
    },
    OSProfile {
        name: "Debian 12",
        min_ram_gb: 2.0,
        min_storage_gb: 10.0,
        min_cores: 1,
        requires_tpm: false,
        requires_64bit: true,
        notes: "Stable option that works well on lower-spec hardware.",
    },
];

#[derive(Serialize, Debug, Clone)]
pub struct Hardware {
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub memory_gb: f64,
    pub storage_gb: f64,
    pub architecture: String,
    pub has_tpm: bool,
    pub boot_mode: String,
    pub gpu: String,
}

#[derive(Serialize)]
pub struct OSMatch {
    pub os: String,
    pub compatibility_score: i32,
    pub compatibility_category: String,
    pub assessment: String,
    pub compatibility_notes: Vec<String>,
    pub improvements: Vec<String>,
    pub profile_notes: String,
}

#[derive(Serialize)]
pub struct Report {
    pub scanned_at: String,
    pub hardware: Hardware,
    pub os_matches: Vec<OSMatch>,
}

fn round_2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn run_command(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn read_memtotal_kb() -> u64 {
    fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|line| line.starts_with("MemTotal:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|num| num.parse::<u64>().ok())
        })
        .unwrap_or(0)
}

fn detect_gpu() -> String {
    if run_command("sh", &["-c", "command -v lspci"]).is_empty() {
        return "Unknown".to_string();
    }

    let output = run_command("lspci", &[]);
    output
        .lines()
        .find(|line| line.contains("VGA compatible controller") || line.contains("3D controller"))
        .map(|line| line.split_once(": ").map_or(line, |(_, gpu)| gpu).to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

pub fn scan_hardware() -> Hardware {
    let architecture = run_command("uname", &["-m"]);
    let cpu = run_command("sh", &["-c", "grep -m1 'model name' /proc/cpuinfo | cut -d: -f2-"]);
    let cpu_model = if cpu.trim().is_empty() {
        "Unknown".to_string()
    } else {
        cpu.trim().to_string()
    };

    let mem_kb = read_memtotal_kb();
    let memory_gb = if mem_kb == 0 {
        0.0
    } else {
        round_2(mem_kb as f64 / (1024.0 * 1024.0))
    };

    let storage_gb = run_command("sh", &["-c", "df -B1 / | tail -1 | awk '{print $2}'"])
        .parse::<f64>()
        .map(|bytes| round_2(bytes / (1024_f64.powi(3))))
        .unwrap_or(0.0);

    Hardware {
        cpu_model,
        cpu_cores: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
        memory_gb,
        storage_gb,
        architecture: if architecture.is_empty() {
            "Unknown".to_string()
        } else {
            architecture
        },
        has_tpm: Path::new("/dev/tpm0").exists(),
        boot_mode: if Path::new("/sys/firmware/efi").exists() {
            "UEFI".to_string()
        } else {
            "Legacy BIOS".to_string()
        },
        gpu: detect_gpu(),
    }
}

fn compatibility_category(score: i32) -> String {
    if score >= 85 {
        "Excellent".to_string()
    } else if score >= 70 {
        "Good".to_string()
    } else if score >= 50 {
        "Fair".to_string()
    } else {
        "Low".to_string()
    }
}

fn score_profile(hardware: &Hardware, profile: &OSProfile) -> (i32, Vec<String>, Vec<String>) {
    let mut score = 100;
    let mut issues = Vec::new();
    let mut improvements = Vec::new();

    if hardware.memory_gb < profile.min_ram_gb {
        let deficit = round_2(profile.min_ram_gb - hardware.memory_gb);
        score -= 30;
        issues.push(format!(
            "RAM below minimum ({} GB < {} GB)",
            hardware.memory_gb, profile.min_ram_gb
        ));
        improvements.push(format!("Upgrade RAM by at least {} GB.", deficit));
    } else if hardware.memory_gb < profile.min_ram_gb + 2.0 {
        score -= 8;
        improvements.push("Add extra RAM headroom for smoother multitasking.".to_string());
    }

    if hardware.storage_gb < profile.min_storage_gb {
        let deficit = round_2(profile.min_storage_gb - hardware.storage_gb);
        score -= 25;
        issues.push(format!(
            "Storage below minimum ({} GB < {} GB)",
            hardware.storage_gb, profile.min_storage_gb
        ));
        improvements.push(format!("Increase storage capacity by at least {} GB.", deficit));
    } else if hardware.storage_gb < profile.min_storage_gb + 30.0 {
        score -= 5;
        improvements.push("Reserve extra free storage for updates and application caches.".to_string());
    }

    if hardware.cpu_cores < profile.min_cores {
        score -= 20;
        issues.push(format!(
            "CPU cores below minimum ({} < {})",
            hardware.cpu_cores, profile.min_cores
        ));
        improvements.push("Use a CPU with more cores for better responsiveness.".to_string());
    }

    let is_64bit = hardware.architecture.contains("64");
    if profile.requires_64bit && !is_64bit {
        score -= 50;
        issues.push("64-bit architecture required.".to_string());
        improvements.push("Move to a 64-bit capable processor and firmware.".to_string());
    }

    if profile.requires_tpm && !hardware.has_tpm {
        score -= 20;
        issues.push("TPM is required but not detected.".to_string());
        improvements.push("Enable TPM 2.0 in firmware or install a TPM module.".to_string());
    }

    if hardware.boot_mode != "UEFI" {
        score -= 5;
        improvements.push("Switch to UEFI boot mode for broader modern OS support.".to_string());
    }

    if improvements.is_empty() {
        improvements.push("No major upgrades required; current hardware is already well-matched.".to_string());
    }

    (score.max(0), issues, improvements)
}

pub fn recommend_os(hardware: &Hardware) -> Vec<OSMatch> {
    let mut matches: Vec<OSMatch> = OS_PROFILES
        .iter()
        .map(|profile| {
            let (score, issues, improvements) = score_profile(hardware, profile);
            OSMatch {
                os: profile.name.to_string(),
                compatibility_score: score,
                compatibility_category: compatibility_category(score),
                assessment: if score >= 70 {
                    "Compatible".to_string()
                } else {
                    "Potential compatibility constraints".to_string()
                },
                compatibility_notes: if issues.is_empty() {
                    vec!["All key minimum requirements are met.".to_string()]
                } else {
                    issues
                },
                improvements,
                profile_notes: profile.notes.to_string(),
            }
        })
        .collect();

    matches.sort_by_key(|item| Reverse(item.compatibility_score));
    matches.truncate(4);
    matches
}

pub fn generate_report() -> Report {
    let hardware = scan_hardware();
    let scanned_at = chrono::Utc::now().to_rfc3339();
    let os_matches = recommend_os(&hardware);

    Report {
        scanned_at,
        hardware,
        os_matches,
    }
}

#[cfg(test)]
mod tests {
    use super::{recommend_os, Hardware};

    #[test]
    fn returns_top_four_sorted_matches() {
        let hardware = Hardware {
            cpu_model: "Example CPU".to_string(),
            cpu_cores: 8,
            memory_gb: 16.0,
            storage_gb: 512.0,
            architecture: "x86_64".to_string(),
            has_tpm: true,
            boot_mode: "UEFI".to_string(),
            gpu: "Example GPU".to_string(),
        };

        let matches = recommend_os(&hardware);

        assert_eq!(matches.len(), 4);
        let scores: Vec<i32> = matches.iter().map(|item| item.compatibility_score).collect();
        let mut sorted = scores.clone();
        sorted.sort_by(|a, b| b.cmp(a));
        assert_eq!(scores, sorted);
    }

    #[test]
    fn missing_requirements_generate_improvements() {
        let hardware = Hardware {
            cpu_model: "Legacy CPU".to_string(),
            cpu_cores: 1,
            memory_gb: 2.0,
            storage_gb: 20.0,
            architecture: "x86".to_string(),
            has_tpm: false,
            boot_mode: "Legacy BIOS".to_string(),
            gpu: "Unknown".to_string(),
        };

        let matches = recommend_os(&hardware);
        let windows_match = matches.iter().find(|item| item.os == "Windows 11").unwrap();

        assert_eq!(windows_match.compatibility_category, "Low");
        assert!(windows_match
            .improvements
            .iter()
            .any(|recommendation| recommendation.contains("TPM")));
        assert!(windows_match
            .improvements
            .iter()
            .any(|recommendation| recommendation.contains("64-bit")));
    }
}
