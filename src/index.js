const os = require('os');
const { createRoonClient } = require('./roon/client');
const { createUPnPClient } = require('./upnp/client');
const { createOpenHomeClient } = require('./openhome/client');
const { HQPClient } = require('./hqplayer/client');
const { createMqttService } = require('./mqtt');
const { createApp } = require('./server/app');
const { createLogger } = require('./lib/logger');
const { loadAppSettings } = require('./lib/settings');
const { advertise } = require('./lib/mdns');
const { createKnobsStore } = require('./knobs/store');
const { createBus } = require('./bus');
const { RoonAdapter } = require('./bus/adapters/roon');
const { UPnPAdapter } = require('./bus/adapters/upnp');
const { OpenHomeAdapter } = require('./bus/adapters/openhome');
const busDebug = require('./bus/debug');

const PORT = process.env.PORT || 8088;
const log = createLogger('Main');

log.info('Starting Unified Hi-Fi Control');

// Get local IP for base URL
function getLocalIp() {
  const interfaces = os.networkInterfaces();
  for (const name of Object.keys(interfaces)) {
    for (const iface of interfaces[name]) {
      if (iface.family === 'IPv4' && !iface.internal) {
        return iface.address;
      }
    }
  }
  return 'localhost';
}

const localIp = getLocalIp();
const baseUrl = `http://${localIp}:${PORT}`;

// Load settings for adapter configuration
const appSettings = loadAppSettings();
const adapterConfig = appSettings.adapters || { roon: true, upnp: false, openhome: false };

log.info('Adapter configuration', adapterConfig);

// Create bus first so we can reference it in callbacks
const bus = createBus({ logger: createLogger('Bus') });

// Conditionally create and register adapters based on settings
let roon = null;
if (adapterConfig.roon !== false) {
  roon = createRoonClient({
    logger: createLogger('Roon'),
    base_url: baseUrl,
    onZonesChanged: () => bus.refreshZones('roon'),
  });
  const roonAdapter = new RoonAdapter(roon);
  bus.registerBackend('roon', roonAdapter);
  log.info('Roon adapter enabled');
}

if (adapterConfig.upnp) {
  const upnp = createUPnPClient({
    logger: createLogger('UPnP'),
  });
  const upnpAdapter = new UPnPAdapter(upnp, {
    onZonesChanged: () => bus.refreshZones('upnp'),
  });
  bus.registerBackend('upnp', upnpAdapter);
  log.info('UPnP adapter enabled');
}

if (adapterConfig.openhome) {
  const openhome = createOpenHomeClient({
    logger: createLogger('OpenHome'),
    onZonesChanged: () => bus.refreshZones('openhome'),
  });
  const openhomeAdapter = new OpenHomeAdapter(openhome, {
    onZonesChanged: () => bus.refreshZones('openhome'),
  });
  bus.registerBackend('openhome', openhomeAdapter);
  log.info('OpenHome adapter enabled');
}

// Create HQPlayer client (unconfigured initially, configured via API or env vars)
const hqp = new HQPClient({
  logger: createLogger('HQP'),
});

// Initialize debug consumer
busDebug.init(bus);

// Create knobs store for ESP32 knob configuration
const knobs = createKnobsStore({
  logger: createLogger('Knobs'),
});

// Pre-configure HQPlayer if env vars set
if (process.env.HQP_HOST) {
  hqp.configure({
    host: process.env.HQP_HOST,
    port: process.env.HQP_PORT || 8088,
    username: process.env.HQP_USER,
    password: process.env.HQP_PASS,
  });
  log.info('HQPlayer pre-configured from environment', { host: process.env.HQP_HOST });
}

// Create MQTT service (opt-in via MQTT_BROKER env var)
const mqttService = createMqttService({
  hqp,
  logger: createLogger('MQTT'),
});

// Create HTTP server
const app = createApp({
  bus,     // Pass bus to app
  roon,    // Keep for backward compat during Phase 2 testing
  hqp,
  knobs,
  logger: createLogger('Server'),
});

// Start services
bus.start();  // Starts all registered backends (including roon)
mqttService.connect();

let mdnsService;

app.listen(PORT, () => {
  log.info(`HTTP server listening on port ${PORT}`);
  if (adapterConfig.roon !== false) {
    log.info('Waiting for Roon Core authorization...');
  }

  // Advertise via mDNS for knob discovery
  mdnsService = advertise(PORT, {
    name: 'Unified Hi-Fi Control',
    base: `http://${localIp}:${PORT}`,
  }, createLogger('mDNS'));
});

// Graceful shutdown
process.on('SIGTERM', async () => {
  log.info('Shutting down...');
  if (mdnsService) mdnsService.stop();
  mqttService.disconnect();
  await bus.stop();
  process.exit(0);
});

process.on('unhandledRejection', (err) => {
  log.error('Unhandled rejection', { error: err.message, stack: err.stack });
});

// Export bus for other modules
// Don't export bus - causes circular dependency with routes
