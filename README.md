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

## Hybrid deployment modes

### 1) Scan my device (local machine)

You have two local options:

- **Tauri desktop app** (recommended packaged UX)
- **Local scanner service** running on localhost and called by a hosted web page

#### Tauri mode

```bash
cd /tmp/workspace/CzarLeMajeste/hardware-scanner/src-tauri
cargo tauri dev
```

Click **Scan my device** in the UI. This path invokes Rust directly via Tauri command `run_live_hardware_scan`.

#### Local scanner service mode

Run the localhost API on the user machine:

```bash
cargo run --bin scanner_service
```

Defaults:

- bind: `127.0.0.1:7878`
- allowed origins: `http://localhost:3000,http://127.0.0.1:3000,http://localhost:5173,http://127.0.0.1:5173`
- token TTL: `120` seconds

Environment variables:

- `SCANNER_SERVICE_BIND` (example: `127.0.0.1:7878`)
- `LOCAL_ALLOWED_ORIGINS` (comma-separated exact origins)
- `LOCAL_TOKEN_TTL_SECONDS`

Local endpoints:

- `POST /api/local/token` body `{"consent": true}`
- `POST /api/local/scan` header `x-local-token: <token from /api/local/token>`

Security controls in local mode:

- explicit user consent (`consent: true`) required
- strict origin allowlist check
- short-lived one-time token for scan call

### 2) Scan server (server-side machine)

Run the same service in your server environment and set an API key:

```bash
SERVER_SCAN_API_KEY="replace-me" SCANNER_SERVICE_BIND="0.0.0.0:8787" cargo run --bin scanner_service
```

Server endpoint:

- `POST /api/server/scan` with header `x-api-key: <SERVER_SCAN_API_KEY>`

This scans the **server hardware only**, not the client browser device.

## Hosted web UI usage

The UI in `/tmp/workspace/CzarLeMajeste/hardware-scanner/ui` supports both flows:

- **Scan my device**
  - uses Tauri invoke when running as desktop app
  - falls back to localhost service (`/api/local/*`) in normal web browser mode
- **Scan server**
  - calls `/api/server/scan` on configured backend URL

## Installers / packaging

### Tauri desktop installers

Build installers for supported desktop targets:

```bash
cd /tmp/workspace/CzarLeMajeste/hardware-scanner/src-tauri
cargo tauri build
```

### Local service binary distribution

Build release binaries per target OS/arch and distribute them with your own installer process:

```bash
cargo build --bin scanner_service --release
```

## Output structure

The scanner returns JSON with:

- `hardware`: detected CPU, RAM, storage, architecture, TPM, boot mode, GPU
- `os_matches`: exactly 4 ranked OS recommendations with:
  - `compatibility_score`
  - `compatibility_category` (`Excellent`, `Good`, `Fair`, `Low`)
  - `compatibility_notes`
  - `improvements`
