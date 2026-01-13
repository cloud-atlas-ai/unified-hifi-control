//! Web UI handlers - daily-use interface for zone and HQPlayer control
//!
//! Multiple clients exist for unified-hifi-control:
//! - Web UI (this) - full control interface, better than HQPlayer Embedded UI
//! - S3 Knob (hardware surface via /now_playing, /control APIs)
//! - Apple Watch / iOS apps (via REST API + SSE)
//! - Home Assistant (via MQTT)
//!
//! Using Pico CSS (classless CSS framework) for clean, accessible,
//! mobile-friendly design without custom CSS maintenance burden.

use axum::{
    extract::State,
    response::{Html, IntoResponse},
};

use crate::api::AppState;

/// HTML document wrapper with Pico CSS
fn html_doc(title: &str, nav_active: &str, content: &str) -> String {
    let nav = nav_html(nav_active);
    format!(
        r#"<!DOCTYPE html>
<html lang="en" data-theme="dark">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{title} - Unified Hi-Fi Control</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@picocss/pico@2/css/pico.min.css">
    <style>
        :root {{ --pico-font-size: 15px; }}
        .status-ok {{ color: var(--pico-ins-color); }}
        .status-err {{ color: var(--pico-del-color); }}
        .zone-grid {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 1rem; }}
        .controls {{ display: flex; gap: 0.5rem; margin-top: 0.5rem; }}
        .controls button {{ margin: 0; padding: 0.5rem 1rem; }}
        small {{ color: var(--pico-muted-color); }}
    </style>
</head>
<body>
    <header class="container">
        {nav}
    </header>
    <main class="container">
        {content}
    </main>
    <footer class="container">
        <small>Unified Hi-Fi Control (Rust) - Admin Interface</small>
    </footer>
</body>
</html>"#
    )
}

/// Navigation HTML
fn nav_html(active: &str) -> String {
    let links = [
        ("dashboard", "Dashboard", "/"),
        ("zones", "Zones", "/zones"),
        ("hqplayer", "HQPlayer", "/hqplayer"),
        ("lms", "LMS", "/lms"),
        ("settings", "Settings", "/settings"),
    ];

    let items: String = links
        .iter()
        .map(|(id, label, href)| {
            if *id == active {
                format!(
                    r#"<li><a href="{href}" aria-current="page"><strong>{label}</strong></a></li>"#
                )
            } else {
                format!(r#"<li><a href="{href}">{label}</a></li>"#)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<nav>
        <ul><li><strong>Hi-Fi Control</strong></li></ul>
        <ul>{items}</ul>
    </nav>"#
    )
}

/// GET / - Dashboard with status overview
pub async fn dashboard_page(State(_state): State<AppState>) -> impl IntoResponse {
    let content = r#"
<h1>Dashboard</h1>

<section id="status">
    <hgroup>
        <h2>Service Status</h2>
        <p>Connection status for all adapters</p>
    </hgroup>
    <article aria-busy="true">Loading status...</article>
</section>

<script>
function esc(s) { return String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'})[c]); }

async function loadStatus() {
    const section = document.querySelector('#status article');
    try {
        const [status, roon, hqp, lms] = await Promise.all([
            fetch('/status').then(r => r.json()),
            fetch('/roon/status').then(r => r.json()).catch(() => ({ connected: false })),
            fetch('/hqplayer/status').then(r => r.json()).catch(() => ({ connected: false })),
            fetch('/lms/status').then(r => r.json()).catch(() => ({ connected: false }))
        ]);

        section.removeAttribute('aria-busy');
        section.innerHTML = `
            <p><strong>Version:</strong> ${esc(status.version)}</p>
            <p><strong>Uptime:</strong> ${status.uptime_secs}s</p>
            <p><strong>Event Bus Subscribers:</strong> ${status.bus_subscribers}</p>
            <hr>
            <table>
                <thead><tr><th>Adapter</th><th>Status</th><th>Details</th></tr></thead>
                <tbody>
                    <tr>
                        <td>Roon</td>
                        <td class="${roon.connected ? 'status-ok' : 'status-err'}">${roon.connected ? '✓ Connected' : '✗ Disconnected'}</td>
                        <td><small>${esc(roon.core_name || '')} ${roon.core_version ? 'v' + esc(roon.core_version) : ''}</small></td>
                    </tr>
                    <tr>
                        <td>HQPlayer</td>
                        <td class="${hqp.connected ? 'status-ok' : 'status-err'}">${hqp.connected ? '✓ Connected' : '✗ Disconnected'}</td>
                        <td><small>${esc(hqp.host || '')}</small></td>
                    </tr>
                    <tr>
                        <td>LMS</td>
                        <td class="${lms.connected ? 'status-ok' : 'status-err'}">${lms.connected ? '✓ Connected' : '✗ Disconnected'}</td>
                        <td><small>${lms.host ? esc(lms.host) + ':' + lms.port : ''}</small></td>
                    </tr>
                    <tr>
                        <td>MQTT</td>
                        <td class="${status.mqtt_connected ? 'status-ok' : 'status-err'}">${status.mqtt_connected ? '✓ Connected' : '✗ Disconnected'}</td>
                        <td></td>
                    </tr>
                </tbody>
            </table>
        `;
    } catch (e) {
        section.removeAttribute('aria-busy');
        section.innerHTML = `<p class="status-err">Error loading status: ${esc(e.message)}</p>`;
    }
}
loadStatus();
setInterval(loadStatus, 10000);
</script>
"#;

    Html(html_doc("Dashboard", "dashboard", content))
}

/// GET /zones - Zones listing and control
pub async fn zones_page(State(_state): State<AppState>) -> impl IntoResponse {
    let content = r#"
<h1>Zones</h1>

<section id="zones">
    <article aria-busy="true">Loading zones...</article>
</section>

<script>
function esc(s) { return String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'})[c]); }

async function loadZones() {
    const section = document.querySelector('#zones');
    try {
        const zones = await fetch('/roon/zones').then(r => r.json());

        if (!zones.length) {
            section.innerHTML = '<article>No zones available. Is Roon Core running?</article>';
            return;
        }

        section.innerHTML = '<div class="zone-grid">' + zones.map(zone => {
            const np = zone.now_playing;
            const nowPlaying = np ? `${esc(np.title)}<br><small>${esc(np.artist)} - ${esc(np.album)}</small>` : '<small>Nothing playing</small>';

            return `
                <article>
                    <header>
                        <strong>${esc(zone.display_name)}</strong>
                        <small> (${esc(zone.state)})</small>
                    </header>
                    <p>${nowPlaying}</p>
                    <footer>
                        <div class="controls" data-zone-id="${esc(zone.zone_id)}">
                            <button data-action="previous" ${zone.is_previous_allowed ? '' : 'disabled'}>⏮</button>
                            <button data-action="play_pause">⏯</button>
                            <button data-action="next" ${zone.is_next_allowed ? '' : 'disabled'}>⏭</button>
                        </div>
                    </footer>
                </article>
            `;
        }).join('') + '</div>';
    } catch (e) {
        section.innerHTML = `<article class="status-err">Error: ${esc(e.message)}</article>`;
    }
}

async function control(zoneId, action) {
    try {
        await fetch('/roon/control', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ zone_id: zoneId, action })
        });
        setTimeout(loadZones, 300);
    } catch (e) {
        console.error('Control error:', e);
    }
}

// Event delegation for zone controls (prevents XSS)
document.querySelector('#zones').addEventListener('click', e => {
    const btn = e.target.closest('button[data-action]');
    if (!btn) return;
    const container = btn.closest('[data-zone-id]');
    if (!container) return;
    control(container.dataset.zoneId, btn.dataset.action);
});

loadZones();
setInterval(loadZones, 4000);
</script>
"#;

    Html(html_doc("Zones", "zones", content))
}

/// GET /hqplayer - HQPlayer status and DSP controls
pub async fn hqplayer_page(State(_state): State<AppState>) -> impl IntoResponse {
    let content = r#"
<h1>HQPlayer</h1>

<section id="hqp-status">
    <hgroup>
        <h2>Connection Status</h2>
        <p>HQPlayer DSP engine connection</p>
    </hgroup>
    <article aria-busy="true">Loading...</article>
</section>

<section id="hqp-pipeline">
    <hgroup>
        <h2>Pipeline Settings</h2>
        <p>Current DSP configuration</p>
    </hgroup>
    <article aria-busy="true">Loading...</article>
</section>

<section id="hqp-profiles">
    <hgroup>
        <h2>Profiles</h2>
        <p>Saved configurations</p>
    </hgroup>
    <article aria-busy="true">Loading...</article>
</section>

<script>
function esc(s) { return String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'})[c]); }

async function loadHqpStatus() {
    const section = document.querySelector('#hqp-status article');
    try {
        const status = await fetch('/hqplayer/status').then(r => r.json());
        section.removeAttribute('aria-busy');

        if (!status.connected) {
            section.innerHTML = '<p class="status-err">Not connected to HQPlayer</p>';
            return;
        }

        section.innerHTML = `
            <p class="status-ok">✓ Connected to ${esc(status.host || 'HQPlayer')}</p>
            <p>State: <strong>${esc(status.state || 'unknown')}</strong></p>
        `;
    } catch (e) {
        section.removeAttribute('aria-busy');
        section.innerHTML = `<p class="status-err">Error: ${esc(e.message)}</p>`;
    }
}

async function loadHqpPipeline() {
    const section = document.querySelector('#hqp-pipeline article');
    try {
        const pipeline = await fetch('/hqplayer/pipeline').then(r => r.json());
        section.removeAttribute('aria-busy');

        section.innerHTML = `
            <table>
                <tr><td>Mode</td><td>${esc(pipeline.mode_str || 'N/A')}</td></tr>
                <tr><td>Filter</td><td>${esc(pipeline.filter_str || 'N/A')}</td></tr>
                <tr><td>Shaper</td><td>${esc(pipeline.shaper_str || 'N/A')}</td></tr>
                <tr><td>Sample Rate</td><td>${esc(pipeline.rate_str || 'N/A')}</td></tr>
                <tr><td>Volume</td><td>${pipeline.volume != null ? pipeline.volume + ' dB' : 'N/A'}</td></tr>
            </table>
            <hr>
            <label>Volume Control
                <input type="range" min="-60" max="0" value="${pipeline.volume || -20}"
                    oninput="this.nextElementSibling.textContent = this.value + ' dB'"
                    onchange="setVolume(this.value)">
                <output>${pipeline.volume || -20} dB</output>
            </label>
        `;
    } catch (e) {
        section.removeAttribute('aria-busy');
        section.innerHTML = `<p class="status-err">Error: ${esc(e.message)}</p>`;
    }
}

async function loadHqpProfiles() {
    const section = document.querySelector('#hqp-profiles article');
    try {
        const profiles = await fetch('/hqplayer/profiles').then(r => r.json());
        section.removeAttribute('aria-busy');

        if (!profiles || !profiles.length) {
            section.innerHTML = '<p>No profiles available</p>';
            return;
        }

        section.innerHTML = `
            <table id="profiles-table">
                <thead><tr><th>Profile</th><th>Status</th><th>Action</th></tr></thead>
                <tbody>
                    ${profiles.map(p => `
                        <tr>
                            <td>${esc(p.name)}</td>
                            <td>${p.active ? '<span class="status-ok">Active</span>' : ''}</td>
                            <td><button data-profile="${esc(p.name)}" ${p.active ? 'disabled' : ''}>Load</button></td>
                        </tr>
                    `).join('')}
                </tbody>
            </table>
        `;
        // Attach click handlers safely
        section.querySelectorAll('button[data-profile]').forEach(btn => {
            btn.addEventListener('click', () => loadProfile(btn.dataset.profile));
        });
    } catch (e) {
        section.removeAttribute('aria-busy');
        section.innerHTML = `<p class="status-err">Error: ${esc(e.message)}</p>`;
    }
}

async function setVolume(value) {
    try {
        await fetch('/hqplayer/volume', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ value: parseInt(value) })
        });
    } catch (e) { console.error(e); }
}

async function loadProfile(name) {
    try {
        await fetch('/hqplayer/profile', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ profile: name })
        });
        setTimeout(loadHqpProfiles, 500);
        setTimeout(loadHqpPipeline, 500);
    } catch (e) { console.error(e); }
}

loadHqpStatus();
loadHqpPipeline();
loadHqpProfiles();
setInterval(loadHqpStatus, 5000);
setInterval(loadHqpPipeline, 5000);
</script>
"#;

    Html(html_doc("HQPlayer", "hqplayer", content))
}

/// GET /lms - LMS status and players
pub async fn lms_page(State(_state): State<AppState>) -> impl IntoResponse {
    let content = r#"
<h1>Logitech Media Server</h1>

<section id="lms-status">
    <hgroup>
        <h2>Connection Status</h2>
        <p>LMS server connection</p>
    </hgroup>
    <article aria-busy="true">Loading...</article>
</section>

<section id="lms-players">
    <hgroup>
        <h2>Players</h2>
        <p>Connected Squeezebox players</p>
    </hgroup>
    <article aria-busy="true">Loading...</article>
</section>

<script>
function esc(s) { return String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'})[c]); }

async function loadLmsStatus() {
    const section = document.querySelector('#lms-status article');
    try {
        const status = await fetch('/lms/status').then(r => r.json());
        section.removeAttribute('aria-busy');

        if (!status.connected) {
            section.innerHTML = '<p class="status-err">Not connected to LMS</p>';
            return;
        }

        section.innerHTML = `
            <p class="status-ok">✓ Connected to ${esc(status.host)}:${status.port}</p>
            <p>Players: <strong>${status.player_count}</strong></p>
        `;
    } catch (e) {
        section.removeAttribute('aria-busy');
        section.innerHTML = `<p class="status-err">Error: ${esc(e.message)}</p>`;
    }
}

async function loadLmsPlayers() {
    const section = document.querySelector('#lms-players article');
    try {
        const players = await fetch('/lms/players').then(r => r.json());
        section.removeAttribute('aria-busy');

        if (!players || !players.length) {
            section.innerHTML = '<p>No players found</p>';
            return;
        }

        section.innerHTML = '<div class="zone-grid" id="lms-grid">' + players.map(player => `
            <article>
                <header>
                    <strong>${esc(player.name)}</strong>
                    <small> (${esc(player.mode)})</small>
                </header>
                <p>
                    ${player.current_title ? esc(player.current_title) : '<small>Nothing playing</small>'}
                    ${player.artist ? `<br><small>${esc(player.artist)}</small>` : ''}
                </p>
                <footer>
                    <div class="controls" data-player-id="${esc(player.player_id)}">
                        <button data-action="previous">⏮</button>
                        <button data-action="play_pause">⏯</button>
                        <button data-action="next">⏭</button>
                    </div>
                    <p>Volume: ${player.volume}%</p>
                </footer>
            </article>
        `).join('') + '</div>';
    } catch (e) {
        section.removeAttribute('aria-busy');
        section.innerHTML = `<p class="status-err">Error: ${esc(e.message)}</p>`;
    }
}

async function lmsControl(playerId, action) {
    try {
        await fetch('/lms/control', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ player_id: playerId, action })
        });
        setTimeout(loadLmsPlayers, 300);
    } catch (e) { console.error(e); }
}

// Event delegation for LMS player controls (runs once, not per-refresh)
document.querySelector('#lms-players').addEventListener('click', e => {
    const btn = e.target.closest('button[data-action]');
    if (!btn) return;
    const container = btn.closest('[data-player-id]');
    if (!container) return;
    lmsControl(container.dataset.playerId, btn.dataset.action);
});

loadLmsStatus();
loadLmsPlayers();
setInterval(loadLmsStatus, 5000);
setInterval(loadLmsPlayers, 4000);
</script>
"#;

    Html(html_doc("LMS", "lms", content))
}

/// GET /settings - Settings page (adapter configuration)
pub async fn settings_page(State(_state): State<AppState>) -> impl IntoResponse {
    let content = r#"
<h1>Settings</h1>

<section id="lms-config">
    <hgroup>
        <h2>Lyrion Music Server (LMS)</h2>
        <p>Configure connection to your Squeezebox server</p>
    </hgroup>
    <article id="lms-config-card">
        <div id="lms-status-line">Checking...</div>
        <div id="lms-config-form" style="display:none;">
            <div class="grid">
                <label>Host
                    <input type="text" id="lms-host" placeholder="192.168.1.x or hostname">
                </label>
                <label>Port
                    <input type="number" id="lms-port" value="9000" min="1" max="65535">
                </label>
            </div>
            <div class="grid">
                <label>Username (optional)
                    <input type="text" id="lms-username" placeholder="Leave blank if not required">
                </label>
                <label>Password (optional)
                    <input type="password" id="lms-password" placeholder="Leave blank if not required">
                </label>
            </div>
            <button onclick="saveLmsConfig()">Save & Connect</button>
            <span id="lms-save-msg"></span>
        </div>
        <button id="lms-reconfig-btn" style="display:none;" onclick="showLmsForm()">Reconfigure</button>
    </article>
</section>

<section id="hqp-config">
    <hgroup>
        <h2>HQPlayer</h2>
        <p>Configure connection to HQPlayer for DSP control</p>
    </hgroup>
    <article id="hqp-config-card">
        <div id="hqp-status-line">Checking...</div>
        <div id="hqp-config-form" style="display:none;">
            <div class="grid">
                <label>Host
                    <input type="text" id="hqp-host" placeholder="192.168.1.x or hostname">
                </label>
                <label>Port
                    <input type="number" id="hqp-port" value="4321" min="1" max="65535">
                </label>
            </div>
            <div class="grid">
                <label>Username (optional)
                    <input type="text" id="hqp-username" placeholder="For web UI profile loading">
                </label>
                <label>Password (optional)
                    <input type="password" id="hqp-password">
                </label>
            </div>
            <button onclick="saveHqpConfig()">Save & Connect</button>
            <span id="hqp-save-msg"></span>
        </div>
        <button id="hqp-reconfig-btn" style="display:none;" onclick="showHqpForm()">Reconfigure</button>
    </article>
</section>

<section id="discovery-status">
    <hgroup>
        <h2>Auto-Discovery</h2>
        <p>Devices found via SSDP (no configuration needed)</p>
    </hgroup>
    <article>
        <table>
            <thead><tr><th>Protocol</th><th>Status</th><th>Devices</th></tr></thead>
            <tbody id="discovery-table">
                <tr><td colspan="3">Loading...</td></tr>
            </tbody>
        </table>
    </article>
</section>

<script>
function esc(s) { return String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'})[c]); }

// LMS Config
async function loadLmsConfig() {
    const statusLine = document.getElementById('lms-status-line');
    const form = document.getElementById('lms-config-form');
    const reconfigBtn = document.getElementById('lms-reconfig-btn');

    try {
        const res = await fetch('/lms/config');
        const data = await res.json();

        if (data.configured && data.connected) {
            statusLine.innerHTML = `<span class="status-ok">✓ Connected to ${esc(data.host)}:${data.port}</span>`;
            statusLine.className = '';
            form.style.display = 'none';
            reconfigBtn.style.display = 'inline-block';
            document.getElementById('lms-host').value = data.host || '';
            document.getElementById('lms-port').value = data.port || 9000;
        } else if (data.configured) {
            statusLine.innerHTML = `<span class="status-err">✗ Configured but not connected (${esc(data.host)}:${data.port})</span>`;
            form.style.display = 'none';
            reconfigBtn.style.display = 'inline-block';
        } else {
            statusLine.textContent = 'Not configured';
            statusLine.className = '';
            form.style.display = 'block';
            reconfigBtn.style.display = 'none';
        }
    } catch (e) {
        statusLine.innerHTML = `<span class="status-err">Error: ${esc(e.message)}</span>`;
        form.style.display = 'block';
    }
}

function showLmsForm() {
    document.getElementById('lms-config-form').style.display = 'block';
    document.getElementById('lms-reconfig-btn').style.display = 'none';
}

async function saveLmsConfig() {
    const msg = document.getElementById('lms-save-msg');
    const host = document.getElementById('lms-host').value.trim();
    const port = parseInt(document.getElementById('lms-port').value) || 9000;
    const username = document.getElementById('lms-username').value.trim() || null;
    const password = document.getElementById('lms-password').value || null;

    if (!host) {
        msg.innerHTML = '<span class="status-err">Host is required</span>';
        return;
    }

    msg.textContent = 'Connecting...';
    try {
        const res = await fetch('/lms/configure', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ host, port, username, password })
        });
        const data = await res.json();
        if (res.ok) {
            msg.innerHTML = '<span class="status-ok">Connected!</span>';
            setTimeout(loadLmsConfig, 500);
        } else {
            msg.innerHTML = `<span class="status-err">${esc(data.error)}</span>`;
        }
    } catch (e) {
        msg.innerHTML = `<span class="status-err">${esc(e.message)}</span>`;
    }
}

// HQPlayer Config
async function loadHqpConfig() {
    const statusLine = document.getElementById('hqp-status-line');
    const form = document.getElementById('hqp-config-form');
    const reconfigBtn = document.getElementById('hqp-reconfig-btn');

    try {
        const res = await fetch('/hqplayer/config');
        const data = await res.json();

        if (data.configured && data.connected) {
            statusLine.innerHTML = `<span class="status-ok">✓ Connected to ${esc(data.host)}:${data.port}</span>`;
            form.style.display = 'none';
            reconfigBtn.style.display = 'inline-block';
            document.getElementById('hqp-host').value = data.host || '';
            document.getElementById('hqp-port').value = data.port || 4321;
        } else if (data.configured) {
            statusLine.innerHTML = `<span class="status-err">✗ Configured but not connected (${esc(data.host)}:${data.port})</span>`;
            form.style.display = 'none';
            reconfigBtn.style.display = 'inline-block';
        } else {
            statusLine.textContent = 'Not configured';
            form.style.display = 'block';
            reconfigBtn.style.display = 'none';
        }
    } catch (e) {
        statusLine.innerHTML = `<span class="status-err">Error: ${esc(e.message)}</span>`;
        form.style.display = 'block';
    }
}

function showHqpForm() {
    document.getElementById('hqp-config-form').style.display = 'block';
    document.getElementById('hqp-reconfig-btn').style.display = 'none';
}

async function saveHqpConfig() {
    const msg = document.getElementById('hqp-save-msg');
    const host = document.getElementById('hqp-host').value.trim();
    const port = parseInt(document.getElementById('hqp-port').value) || 4321;
    const username = document.getElementById('hqp-username').value.trim() || null;
    const password = document.getElementById('hqp-password').value || null;

    if (!host) {
        msg.innerHTML = '<span class="status-err">Host is required</span>';
        return;
    }

    msg.textContent = 'Connecting...';
    try {
        const res = await fetch('/hqplayer/configure', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ host, port, username, password })
        });
        const data = await res.json();
        if (res.ok) {
            if (data.connected) {
                msg.innerHTML = '<span class="status-ok">Connected!</span>';
            } else {
                msg.innerHTML = '<span class="status-err">Saved but not connected - check host</span>';
            }
            setTimeout(loadHqpConfig, 500);
        } else {
            msg.innerHTML = `<span class="status-err">${esc(data.error)}</span>`;
        }
    } catch (e) {
        msg.innerHTML = `<span class="status-err">${esc(e.message)}</span>`;
    }
}

// Discovery status
async function loadDiscoveryStatus() {
    const tbody = document.getElementById('discovery-table');
    try {
        const [openhome, upnp, roon] = await Promise.all([
            fetch('/openhome/status').then(r => r.json()).catch(() => ({ connected: false, device_count: 0 })),
            fetch('/upnp/status').then(r => r.json()).catch(() => ({ connected: false, renderer_count: 0 })),
            fetch('/roon/status').then(r => r.json()).catch(() => ({ connected: false }))
        ]);

        tbody.innerHTML = `
            <tr>
                <td>Roon</td>
                <td class="${roon.connected ? 'status-ok' : 'status-err'}">${roon.connected ? '✓ Connected' : '✗ Not connected'}</td>
                <td>${roon.connected ? esc(roon.core_name || 'Core') : '-'}</td>
            </tr>
            <tr>
                <td>OpenHome</td>
                <td class="${openhome.device_count > 0 ? 'status-ok' : ''}">${openhome.device_count > 0 ? '✓ Active' : 'Searching...'}</td>
                <td>${openhome.device_count} device${openhome.device_count !== 1 ? 's' : ''}</td>
            </tr>
            <tr>
                <td>UPnP/DLNA</td>
                <td class="${upnp.renderer_count > 0 ? 'status-ok' : ''}">${upnp.renderer_count > 0 ? '✓ Active' : 'Searching...'}</td>
                <td>${upnp.renderer_count} renderer${upnp.renderer_count !== 1 ? 's' : ''}</td>
            </tr>
        `;
    } catch (e) {
        tbody.innerHTML = `<tr><td colspan="3" class="status-err">Error: ${esc(e.message)}</td></tr>`;
    }
}

// Load all on page load
loadLmsConfig();
loadHqpConfig();
loadDiscoveryStatus();
setInterval(loadDiscoveryStatus, 10000);
</script>
"#;

    Html(html_doc("Settings", "settings", content))
}

/// Legacy redirects
pub async fn control_redirect() -> impl IntoResponse {
    axum::response::Redirect::to("/zones")
}

pub async fn settings_redirect() -> impl IntoResponse {
    axum::response::Redirect::to("/settings")
}
