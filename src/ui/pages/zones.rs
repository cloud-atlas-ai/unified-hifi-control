//! Zones listing page component.
//!
//! Shows all available zones with:
//! - Zone cards in a grid layout
//! - Now playing info for each zone
//! - Transport controls per zone
//! - HQPlayer DSP section for linked zones

use dioxus::prelude::*;

use crate::ui::components::Layout;

/// Client-side JavaScript for the Zones page.
const ZONES_SCRIPT: &str = r#"
function esc(s) { return String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'})[c]); }

let hqpZoneLinks = {};
let matrixProfiles = [];

async function loadZones() {
    const section = document.querySelector('#zones');
    try {
        const [zonesRes, linksRes] = await Promise.all([
            fetch('/zones').then(r => r.json()),
            fetch('/hqp/zones/links').then(r => r.json()).catch(() => ({ links: [] }))
        ]);
        // /zones returns {zones: [...]} with zone_id and zone_name
        const zones = zonesRes.zones || zonesRes || [];

        // Build HQP link lookup (API returns {links: [...]})
        const links = linksRes.links || linksRes || [];
        hqpZoneLinks = {};
        links.forEach(l => { hqpZoneLinks[l.zone_id] = l.instance; });

        if (!zones.length) {
            section.innerHTML = '<article>No zones available. Check that adapters are connected.</article>';
            return;
        }

        section.innerHTML = '<div class="zone-grid">' + zones.map(zone => {
            const playIcon = zone.state === 'playing' ? '⏸︎' : '▶';
            const hqpLink = hqpZoneLinks[zone.zone_id];
            const hqpBadge = hqpLink ? `<mark style="font-size:0.7em;padding:0.1em 0.3em;margin-left:0.5em;">HQP</mark>` : '';
            const sourceBadge = zone.source ? `<mark style="font-size:0.7em;padding:0.1em 0.3em;margin-left:0.5em;background:var(--pico-muted-background);">${esc(zone.source)}</mark>` : '';

            return `
                <article>
                    <header>
                        <strong>${esc(zone.zone_name)}</strong>${hqpBadge}${sourceBadge}
                        <small> (${esc(zone.state)})</small>
                    </header>
                    <div id="zone-info-${esc(zone.zone_id)}" style="min-height:40px;overflow:hidden;"><small>Loading...</small></div>
                    <footer>
                        <div class="controls" data-zone-id="${esc(zone.zone_id)}">
                            <button data-action="previous">◀◀</button>
                            <button data-action="play_pause">${playIcon}</button>
                            <button data-action="next">▶▶</button>
                        </div>
                    </footer>
                </article>
            `;
        }).join('') + '</div>';

        // Fetch now playing info for each zone
        zones.forEach(async zone => {
            const infoEl = document.getElementById('zone-info-' + zone.zone_id);
            if (!infoEl) return;
            try {
                const np = await fetch('/now_playing?zone_id=' + encodeURIComponent(zone.zone_id)).then(r => r.json());
                if (np && np.line1 && np.line1 !== 'Idle') {
                    infoEl.innerHTML = '<strong style="font-size:0.9em;">' + esc(np.line1) + '</strong><br><small>' + esc(np.line2 || '') + '</small>';
                } else {
                    infoEl.innerHTML = '<small>Nothing playing</small>';
                }
            } catch (e) {
                infoEl.innerHTML = '<small>—</small>';
            }
        });

        // Show HQP DSP section if any zone is linked
        const hasHqpLinks = Object.keys(hqpZoneLinks).length > 0;
        document.getElementById('hqp-dsp').style.display = hasHqpLinks ? 'block' : 'none';
        if (hasHqpLinks) loadHqpDsp();
    } catch (e) {
        section.innerHTML = `<article class="status-err">Error: ${esc(e.message)}</article>`;
    }
}

async function loadHqpDsp() {
    const section = document.getElementById('hqp-dsp-controls');
    try {
        const [profiles, pipeline] = await Promise.all([
            fetch('/hqplayer/matrix/profiles').then(r => r.json()).catch(() => []),
            fetch('/hqplayer/pipeline').then(r => r.json()).catch(() => null)
        ]);
        matrixProfiles = profiles || [];

        const st = pipeline?.status || {};
        const currentProfile = st.active_convolution || st.convolution || 'None';

        if (!matrixProfiles.length) {
            section.innerHTML = '<p>No matrix profiles available. Configure HQPlayer first.</p>';
            return;
        }

        section.innerHTML = `
            <div style="display:flex;gap:1rem;align-items:center;flex-wrap:wrap;">
                <label style="margin:0;">Matrix Profile:
                    <select id="matrix-select" style="width:auto;margin-left:0.5rem;">
                        <option value="">-- Select --</option>
                        ${matrixProfiles.map(p => {
                            const name = p.name || p;
                            const selected = name === currentProfile ? ' selected' : '';
                            return `<option value="${esc(name)}"${selected}>${esc(name)}</option>`;
                        }).join('')}
                    </select>
                </label>
                <span id="matrix-status"></span>
            </div>
            <p style="margin-top:0.5rem;"><small>Current: <strong>${esc(st.active_filter || 'N/A')}</strong> / <strong>${esc(st.active_shaper || 'N/A')}</strong></small></p>
        `;

        document.getElementById('matrix-select').addEventListener('change', async (e) => {
            const profile = e.target.value;
            if (!profile) return;
            const statusEl = document.getElementById('matrix-status');
            statusEl.textContent = 'Loading...';
            try {
                await fetch('/hqplayer/matrix/profile', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ profile })
                });
                statusEl.innerHTML = '<span class="status-ok">✓</span>';
                setTimeout(loadHqpDsp, 500);
            } catch (err) {
                statusEl.innerHTML = '<span class="status-err">Failed</span>';
            }
        });
    } catch (e) {
        section.innerHTML = `<p class="status-err">Error: ${esc(e.message)}</p>`;
    }
}

async function control(zoneId, action) {
    try {
        await fetch('/control', {
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
"#;

/// Zones listing page component.
#[component]
pub fn ZonesPage() -> Element {
    rsx! {
        Layout {
            title: "Zones".to_string(),
            nav_active: "zones".to_string(),
            scripts: Some(ZONES_SCRIPT.to_string()),

            h1 { "Zones" }

            section { id: "zones",
                article {
                    aria_busy: "true",
                    "Loading zones..."
                }
            }

            // HQPlayer DSP section (hidden initially, shown if zones are linked)
            section { id: "hqp-dsp", style: "display:none;",
                hgroup {
                    h2 { "HQPlayer DSP" }
                    p { "Matrix profiles for linked zones" }
                }
                article { id: "hqp-dsp-controls",
                    "Loading..."
                }
            }
        }
    }
}
