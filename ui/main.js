const scanBtn = document.querySelector("#scan-btn");
const statusEl = document.querySelector("#status");
const hardwareEl = document.querySelector("#hardware");
const matchesEl = document.querySelector("#matches");

const categoryClass = {
  Excellent: "excellent",
  Good: "good",
  Fair: "fair",
  Low: "low",
};

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

async function runScan() {
  statusEl.textContent = "Scanning local hardware...";
  scanBtn.disabled = true;

  try {
    const json = await window.__TAURI__.core.invoke("run_live_hardware_scan");
    const report = JSON.parse(json);

    renderHardware(report.hardware, report.scanned_at);
    renderMatches(report.os_matches);
    statusEl.textContent = "Scan complete.";
  } catch (error) {
    statusEl.textContent = `Scan failed: ${error}`;
  } finally {
    scanBtn.disabled = false;
  }
}

scanBtn.addEventListener("click", runScan);
