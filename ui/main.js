const localScanBtn = document.querySelector("#scan-local-btn");
const serverScanBtn = document.querySelector("#scan-server-btn");
const localServiceUrlInput = document.querySelector("#local-service-url");
const serverServiceUrlInput = document.querySelector("#server-service-url");
const serverApiKeyInput = document.querySelector("#server-api-key");
const statusEl = document.querySelector("#status");
const hardwareEl = document.querySelector("#hardware");
const matchesEl = document.querySelector("#matches");

const categoryClass = {
  Excellent: "excellent",
  Good: "good",
  Fair: "fair",
  Low: "low",
};

function setBusyState(isBusy) {
  localScanBtn.disabled = isBusy;
  serverScanBtn.disabled = isBusy;
}

function renderHardware(hardware, scannedAt) {
  hardwareEl.classList.remove("hidden");
  hardwareEl.innerHTML = `
    <h2>Detected Hardware</h2>
    <div class="hardware-grid">
      <div><span>CPU</span><strong>${hardware.cpu_model} (${hardware.cpu_cores} cores)</strong></div>
      <div><span>Memory</span><strong>${hardware.memory_gb} GB</strong></div>
      <div><span>Storage</span><strong>${hardware.storage_gb} GB</strong></div>
      <div><span>Architecture</span><strong>${hardware.architecture}</strong></div>
      <div><span>TPM</span><strong>${hardware.has_tpm ? "Present" : "Not detected"}</strong></div>
      <div><span>Boot Mode</span><strong>${hardware.boot_mode}</strong></div>
      <div><span>GPU</span><strong>${hardware.gpu}</strong></div>
      <div><span>Scanned At</span><strong>${new Date(scannedAt).toLocaleString()}</strong></div>
    </div>
  `;
}

function renderMatches(matches) {
  matchesEl.classList.remove("hidden");
  matchesEl.innerHTML = `
    <h2>Top OS Matches</h2>
    <div class="card-grid">
      ${matches
        .map(
          (match) => `
          <article class="match-card">
            <header>
              <h3>${match.os}</h3>
              <span class="badge ${categoryClass[match.compatibility_category] || "fair"}">${match.compatibility_category} · ${match.compatibility_score}</span>
            </header>
            <p class="assessment">${match.assessment}</p>
            <p><strong>Notes:</strong> ${match.profile_notes}</p>
            <ul>
              ${match.compatibility_notes.map((note) => `<li>${note}</li>`).join("")}
            </ul>
            <p><strong>Improvements</strong></p>
            <ul>
              ${match.improvements.map((improvement) => `<li>${improvement}</li>`).join("")}
            </ul>
          </article>
        `,
        )
        .join("")}
    </div>
  `;
}

function renderReport(report, scopeLabel) {
  renderHardware(report.hardware, report.scanned_at);
  renderMatches(report.os_matches);
  statusEl.textContent = `${scopeLabel} scan complete.`;
}

async function scanWithTauriIfAvailable() {
  if (!window.__TAURI__?.core?.invoke) {
    return null;
  }

  const json = await window.__TAURI__.core.invoke("run_live_hardware_scan");
  return JSON.parse(json);
}

async function requestLocalAccessToken(localServiceUrl) {
  const response = await fetch(`${localServiceUrl}/api/local/token`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ consent: true }),
  });

  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    throw new Error(body.error || `token request failed (${response.status})`);
  }

  const body = await response.json();
  return body.access_token;
}

async function scanViaLocalService() {
  const localServiceUrl = localServiceUrlInput.value.trim();
  if (!localServiceUrl) {
    throw new Error("Set a local scanner URL first.");
  }

  const consent = window.confirm(
    "This will scan hardware from this machine and share the report with this page. Continue?",
  );
  if (!consent) {
    throw new Error("Scan cancelled by user.");
  }

  const accessToken = await requestLocalAccessToken(localServiceUrl);
  const response = await fetch(`${localServiceUrl}/api/local/scan`, {
    method: "POST",
    headers: {
      "x-local-token": accessToken,
    },
  });

  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    throw new Error(body.error || `local scan failed (${response.status})`);
  }

  return response.json();
}

async function runLocalScan() {
  statusEl.textContent = "Running local scan...";
  setBusyState(true);

  try {
    const tauriReport = await scanWithTauriIfAvailable();
    if (tauriReport) {
      renderReport(tauriReport, "Local (Tauri)");
      return;
    }

    const localReport = await scanViaLocalService();
    renderReport(localReport, "Localhost service");
  } catch (error) {
    statusEl.textContent = `Local scan failed: ${error.message || error}`;
  } finally {
    setBusyState(false);
  }
}

async function runServerScan() {
  statusEl.textContent = "Running server scan...";
  setBusyState(true);

  try {
    const serverUrl = serverServiceUrlInput.value.trim();
    const apiKey = serverApiKeyInput.value.trim();

    if (!serverUrl) {
      throw new Error("Set server scanner URL first.");
    }
    if (!apiKey) {
      throw new Error("Set server API key first.");
    }

    const response = await fetch(`${serverUrl}/api/server/scan`, {
      method: "POST",
      headers: {
        "x-api-key": apiKey,
      },
    });

    if (!response.ok) {
      const body = await response.json().catch(() => ({}));
      throw new Error(body.error || `server scan failed (${response.status})`);
    }

    const body = await response.json();
    renderReport(body.report, "Server");
  } catch (error) {
    statusEl.textContent = `Server scan failed: ${error.message || error}`;
  } finally {
    setBusyState(false);
  }
}

localScanBtn.addEventListener("click", runLocalScan);
serverScanBtn.addEventListener("click", runServerScan);
