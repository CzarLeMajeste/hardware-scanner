# hardware-scanner

Hardware scanner that inspects the current machine and returns the **top 4 OS matches** in JSON, categorized by compatibility score and including detailed hardware improvement guidance for each option.

## CLI usage

Rust implementation:

```bash
cargo run --release
```

Compact output:

```bash
cargo run --release -- --compact
```

Legacy Python implementation (kept for compatibility):

```bash
python /tmp/workspace/CzarLeMajeste/hardware-scanner/hardware_scanner.py
```

## Tauri web demo

The Tauri app runs a **real local hardware scan** in Rust and displays results in a modern HTML/CSS UI.

### Prerequisites

- Rust toolchain
- Tauri CLI:

```bash
cargo install tauri-cli
```

### Run desktop app

```bash
cd /tmp/workspace/CzarLeMajeste/hardware-scanner/src-tauri
cargo tauri dev
```

Click **Run Live Scan** in the app window to execute `run_live_hardware_scan` and render detected hardware plus ranked OS matches.

## Output structure

The scanner prints JSON with:

- `hardware`: detected CPU, RAM, storage, architecture, TPM, boot mode, GPU
- `os_matches`: exactly 4 ranked OS recommendations with:
  - `compatibility_score`
  - `compatibility_category` (`Excellent`, `Good`, `Fair`, `Low`)
  - `compatibility_notes`
  - `improvements`
