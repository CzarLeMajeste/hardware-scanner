# hardware-scanner

Hardware scanner that inspects the current machine and returns the **top 4 OS matches** in JSON, categorized by compatibility score and including detailed hardware improvement guidance for each option.

## Usage

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

## Output structure

The scanner prints JSON with:

- `hardware`: detected CPU, RAM, storage, architecture, TPM, boot mode, GPU
- `os_matches`: exactly 4 ranked OS recommendations with:
  - `compatibility_score`
  - `compatibility_category` (`Excellent`, `Good`, `Fair`, `Low`)
  - `compatibility_notes`
  - `improvements`
