const https = require('https');
const fs = require('fs');
const path = require('path');

const DEFAULT_POLL_INTERVAL_MS = 6 * 60 * 60 * 1000; // 6 hours

/**
 * FirmwareService - Polls GitHub for new firmware releases and notifies via bus
 */
function createFirmwareService({ logger, pollIntervalMs } = {}) {
  const log = logger || console;
  let pollTimer = null;
  let lastKnownVersion = null;
  let isStarted = false;
  const eventListeners = [];

  // Config from environment
  const CONFIG_DIR = process.env.CONFIG_DIR || path.join(__dirname, '..', '..', 'data');
  const FIRMWARE_DIR = process.env.FIRMWARE_DIR || path.join(CONFIG_DIR, 'firmware');
  const GITHUB_REPO = process.env.FIRMWARE_REPO || 'muness/roon-knob';
  const POLL_INTERVAL = pollIntervalMs || parseInt(process.env.FIRMWARE_POLL_INTERVAL_MS, 10) || DEFAULT_POLL_INTERVAL_MS;

  /**
   * Subscribe to firmware events
   */
  function on(event, callback) {
    eventListeners.push({ event, callback });
    return () => {
      const idx = eventListeners.findIndex(l => l.event === event && l.callback === callback);
      if (idx >= 0) eventListeners.splice(idx, 1);
    };
  }

  function emit(event, data) {
    eventListeners
      .filter(l => l.event === event)
      .forEach(l => {
        try {
          l.callback(data);
        } catch (err) {
          log.error('Firmware event listener error', { event, error: err.message });
        }
      });
  }

  /**
   * Get currently installed firmware version from version.json
   */
  function getCurrentVersion() {
    const versionFile = path.join(FIRMWARE_DIR, 'version.json');
    if (fs.existsSync(versionFile)) {
      try {
        const data = JSON.parse(fs.readFileSync(versionFile, 'utf8'));
        return data.version || null;
      } catch (e) {
        log.warn('Failed to read version.json', { error: e.message });
      }
    }
    return null;
  }

  /**
   * Fetch latest release info from GitHub API
   */
  async function fetchLatestRelease() {
    return new Promise((resolve, reject) => {
      const options = {
        hostname: 'api.github.com',
        path: `/repos/${GITHUB_REPO}/releases/latest`,
        headers: { 'User-Agent': 'unified-hifi-control' }
      };

      https.get(options, (response) => {
        let data = '';
        response.on('data', chunk => data += chunk);
        response.on('end', () => {
          if (response.statusCode === 200) {
            try {
              resolve(JSON.parse(data));
            } catch (e) {
              reject(new Error(`Failed to parse GitHub response: ${e.message}`));
            }
          } else if (response.statusCode === 404) {
            resolve(null); // No releases yet
          } else {
            reject(new Error(`GitHub API error: ${response.statusCode}`));
          }
        });
      }).on('error', reject);
    });
  }

  /**
   * Download firmware binary from GitHub release
   */
  async function downloadFirmware(asset, version, releaseUrl) {
    const downloadUrl = asset.browser_download_url;
    log.info('Downloading firmware', { version, url: downloadUrl });

    if (!fs.existsSync(FIRMWARE_DIR)) {
      fs.mkdirSync(FIRMWARE_DIR, { recursive: true });
    }

    // Download to temp file first, rename on success
    const firmwarePath = path.join(FIRMWARE_DIR, 'roon_knob.bin');
    const tempPath = path.join(FIRMWARE_DIR, 'roon_knob.bin.tmp');
    const file = fs.createWriteStream(tempPath);

    await new Promise((resolve, reject) => {
      const MAX_REDIRECTS = 5;
      let redirectCount = 0;

      const download = (url) => {
        https.get(url, (response) => {
          if (response.statusCode === 302 || response.statusCode === 301) {
            redirectCount++;
            if (redirectCount > MAX_REDIRECTS) {
              file.close();
              fs.unlinkSync(tempPath);
              reject(new Error('Too many redirects'));
              return;
            }
            response.resume(); // Consume response to free up memory
            download(response.headers.location);
            return;
          }
          if (response.statusCode !== 200) {
            file.close();
            fs.unlinkSync(tempPath);
            reject(new Error(`Download failed: ${response.statusCode}`));
            return;
          }
          response.pipe(file);
          file.on('finish', () => {
            file.close();
            // Rename temp to final on success
            fs.renameSync(tempPath, firmwarePath);
            resolve();
          });
        }).on('error', (err) => {
          file.close();
          if (fs.existsSync(tempPath)) fs.unlinkSync(tempPath);
          reject(err);
        });
      };
      download(downloadUrl);
    });

    const versionPath = path.join(FIRMWARE_DIR, 'version.json');
    fs.writeFileSync(versionPath, JSON.stringify({
      version,
      file: 'roon_knob.bin',
      fetched_at: new Date().toISOString(),
      release_url: releaseUrl
    }, null, 2));

    const stats = fs.statSync(firmwarePath);
    log.info('Firmware downloaded successfully', { version, size: stats.size });

    return { version, size: stats.size, file: 'roon_knob.bin' };
  }

  /**
   * Compare semver versions: returns true if remote > local
   */
  function isNewerVersion(remoteVersion, localVersion) {
    if (!localVersion) return true;
    if (!remoteVersion) return false;

    const remote = remoteVersion.replace(/^v/, '').split('.').map(Number);
    const local = localVersion.replace(/^v/, '').split('.').map(Number);

    for (let i = 0; i < 3; i++) {
      const r = remote[i] || 0;
      const l = local[i] || 0;
      if (r > l) return true;
      if (r < l) return false;
    }
    return false;
  }

  /**
   * Check for updates and optionally download
   */
  async function checkForUpdates({ autoDownload = true } = {}) {
    try {
      const releaseData = await fetchLatestRelease();

      if (!releaseData) {
        log.debug('No releases found on GitHub');
        return { updateAvailable: false, currentVersion: getCurrentVersion() };
      }

      const latestVersion = releaseData.tag_name.replace(/^v/, '');
      const currentVersion = getCurrentVersion();
      const asset = releaseData.assets.find(a => a.name === 'roon_knob.bin');

      const status = {
        currentVersion,
        latestVersion,
        updateAvailable: isNewerVersion(latestVersion, currentVersion),
        releaseUrl: releaseData.html_url,
        hasAsset: !!asset,
        timestamp: Date.now()
      };

      if (status.updateAvailable && asset) {
        log.info('New firmware version available', {
          current: currentVersion,
          latest: latestVersion
        });

        // Emit event before download
        emit('update_available', {
          currentVersion,
          latestVersion,
          releaseUrl: releaseData.html_url
        });

        if (autoDownload) {
          const downloaded = await downloadFirmware(asset, latestVersion, releaseData.html_url);
          status.downloaded = true;
          status.size = downloaded.size;

          // Emit event after download
          emit('firmware_downloaded', {
            version: latestVersion,
            size: downloaded.size,
            releaseUrl: releaseData.html_url
          });
        }
      } else {
        log.debug('Firmware is up to date', { version: currentVersion });
      }

      lastKnownVersion = getCurrentVersion();
      return status;
    } catch (err) {
      log.error('Failed to check for firmware updates', { error: err.message });
      return {
        error: err.message,
        currentVersion: getCurrentVersion(),
        timestamp: Date.now()
      };
    }
  }

  /**
   * Start periodic polling
   */
  function start() {
    if (isStarted) {
      log.warn('FirmwareService already started');
      return;
    }

    isStarted = true;
    log.info('FirmwareService started', {
      repo: GITHUB_REPO,
      pollInterval: `${POLL_INTERVAL / 1000 / 60 / 60}h`,
      firmwareDir: FIRMWARE_DIR
    });

    // Check immediately on start
    checkForUpdates().catch(err => {
      log.error('Initial firmware check failed', { error: err.message });
    });

    // Then poll at interval
    pollTimer = setInterval(() => {
      checkForUpdates().catch(err => {
        log.error('Periodic firmware check failed', { error: err.message });
      });
    }, POLL_INTERVAL);
  }

  /**
   * Stop periodic polling
   */
  function stop() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
    isStarted = false;
    log.info('FirmwareService stopped');
  }

  /**
   * Get current status
   */
  function getStatus() {
    return {
      currentVersion: getCurrentVersion(),
      lastKnownVersion,
      isPolling: isStarted,
      pollInterval: POLL_INTERVAL,
      repo: GITHUB_REPO
    };
  }

  return {
    start,
    stop,
    checkForUpdates,
    getStatus,
    getCurrentVersion,
    on,
  };
}

module.exports = { createFirmwareService };
