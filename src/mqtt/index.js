const mqtt = require('mqtt');

const DEFAULT_TOPIC_PREFIX = 'unified-hifi';
const PUBLISH_INTERVAL_MS = 5000; // Publish state every 5 seconds when connected

function createMqttService({ hqp, logger } = {}) {
  const log = logger || console;
  let client = null;
  let publishTimer = null;
  let topicPrefix = DEFAULT_TOPIC_PREFIX;

  function isEnabled() {
    return !!process.env.MQTT_BROKER;
  }

  function connect() {
    if (!isEnabled()) {
      log.info('MQTT disabled (set MQTT_BROKER to enable)');
      return;
    }

    const broker = process.env.MQTT_BROKER;
    topicPrefix = process.env.MQTT_TOPIC_PREFIX || DEFAULT_TOPIC_PREFIX;

    const options = {
      clientId: `unified-hifi-${Date.now()}`,
      clean: true,
      reconnectPeriod: 5000,
    };

    if (process.env.MQTT_USER) {
      options.username = process.env.MQTT_USER;
      options.password = process.env.MQTT_PASS || '';
    }

    log.info('Connecting to MQTT broker', { broker, topicPrefix });

    client = mqtt.connect(broker, options);

    client.on('connect', () => {
      log.info('MQTT connected');

      // Subscribe to command topics
      const commandTopics = [
        `${topicPrefix}/command/hqplayer/load`,
        `${topicPrefix}/command/hqplayer/pipeline`,
      ];

      commandTopics.forEach(topic => {
        client.subscribe(topic, (err) => {
          if (err) {
            log.error('MQTT subscribe failed', { topic, error: err.message });
          } else {
            log.info('MQTT subscribed', { topic });
          }
        });
      });

      // Start publishing state
      startPublishing();

      // Publish discovery config for Home Assistant
      publishHADiscovery();
    });

    client.on('error', (err) => {
      log.error('MQTT error', { error: err.message });
    });

    client.on('reconnect', () => {
      log.info('MQTT reconnecting...');
    });

    client.on('message', async (topic, message) => {
      try {
        await handleMessage(topic, message.toString());
      } catch (err) {
        log.error('MQTT message handler error', { topic, error: err.message });
      }
    });
  }

  async function handleMessage(topic, payload) {
    log.debug('MQTT message received', { topic, payload });

    if (topic === `${topicPrefix}/command/hqplayer/load`) {
      // Load a profile
      if (!hqp.isConfigured()) {
        log.warn('HQPlayer not configured, ignoring load command');
        return;
      }
      const profile = payload.trim();
      if (profile) {
        log.info('Loading HQPlayer profile via MQTT', { profile });
        await hqp.loadProfile(profile);
        // Publish updated state after a delay (HQP restarts)
        setTimeout(() => publishHqpState(), 10000);
      }
    } else if (topic === `${topicPrefix}/command/hqplayer/pipeline`) {
      // Set pipeline setting: { "setting": "filter1x", "value": "poly-sinc-gauss-xl" }
      if (!hqp.isConfigured()) {
        log.warn('HQPlayer not configured, ignoring pipeline command');
        return;
      }
      try {
        const { setting, value } = JSON.parse(payload);
        if (setting && value !== undefined) {
          log.info('Setting HQPlayer pipeline via MQTT', { setting, value });
          await hqp.setPipelineSetting(setting, value);
          setTimeout(() => publishHqpState(), 1000);
        }
      } catch (e) {
        log.warn('Invalid pipeline command payload', { payload, error: e.message });
      }
    }
  }

  function startPublishing() {
    if (publishTimer) {
      clearInterval(publishTimer);
    }

    // Publish immediately
    publishHqpState();

    // Then publish periodically
    publishTimer = setInterval(() => {
      publishHqpState();
    }, PUBLISH_INTERVAL_MS);
  }

  async function publishHqpState() {
    if (!client || !client.connected) return;

    try {
      const status = await hqp.getStatus();

      // Publish status
      client.publish(
        `${topicPrefix}/hqplayer/status`,
        JSON.stringify(status),
        { retain: true }
      );

      // If connected, publish detailed pipeline
      if (status.connected && status.pipeline) {
        client.publish(
          `${topicPrefix}/hqplayer/pipeline`,
          JSON.stringify(status.pipeline),
          { retain: true }
        );
      }

      // Publish profiles list
      if (status.profiles && status.profiles.length > 0) {
        client.publish(
          `${topicPrefix}/hqplayer/profiles`,
          JSON.stringify(status.profiles),
          { retain: true }
        );
      }

      log.debug('Published HQPlayer state to MQTT');
    } catch (err) {
      log.warn('Failed to publish HQPlayer state', { error: err.message });
    }
  }

  function publishHADiscovery() {
    if (!client || !client.connected) return;

    // HQPlayer config name sensor
    const configSensor = {
      name: 'HQPlayer Config',
      unique_id: 'unified_hifi_hqp_config',
      state_topic: `${topicPrefix}/hqplayer/status`,
      value_template: '{{ value_json.configName | default("Unknown", true) }}',
      availability_topic: `${topicPrefix}/hqplayer/status`,
      availability_template: '{{ "online" if value_json.connected else "offline" }}',
      icon: 'mdi:audio-video',
    };

    client.publish(
      'homeassistant/sensor/unified_hifi_hqp_config/config',
      JSON.stringify(configSensor),
      { retain: true }
    );

    // HQPlayer state sensor (playing/stopped)
    const stateSensor = {
      name: 'HQPlayer State',
      unique_id: 'unified_hifi_hqp_state',
      state_topic: `${topicPrefix}/hqplayer/status`,
      value_template: '{{ value_json.pipeline.status.state | default("Unknown", true) }}',
      availability_topic: `${topicPrefix}/hqplayer/status`,
      availability_template: '{{ "online" if value_json.connected else "offline" }}',
      icon: 'mdi:play-circle',
    };

    client.publish(
      'homeassistant/sensor/unified_hifi_hqp_state/config',
      JSON.stringify(stateSensor),
      { retain: true }
    );

    // HQPlayer filter sensor
    const filterSensor = {
      name: 'HQPlayer Filter',
      unique_id: 'unified_hifi_hqp_filter',
      state_topic: `${topicPrefix}/hqplayer/status`,
      value_template: '{{ value_json.pipeline.settings.filter1x.selected.label | default("Unknown", true) }}',
      availability_topic: `${topicPrefix}/hqplayer/status`,
      availability_template: '{{ "online" if value_json.connected else "offline" }}',
      icon: 'mdi:tune',
    };

    client.publish(
      'homeassistant/sensor/unified_hifi_hqp_filter/config',
      JSON.stringify(filterSensor),
      { retain: true }
    );

    // HQPlayer sample rate sensor
    const samplerateSensor = {
      name: 'HQPlayer Sample Rate',
      unique_id: 'unified_hifi_hqp_samplerate',
      state_topic: `${topicPrefix}/hqplayer/status`,
      value_template: '{{ value_json.pipeline.settings.samplerate.selected.label | default("Unknown", true) }}',
      availability_topic: `${topicPrefix}/hqplayer/status`,
      availability_template: '{{ "online" if value_json.connected else "offline" }}',
      icon: 'mdi:waveform',
    };

    client.publish(
      'homeassistant/sensor/unified_hifi_hqp_samplerate/config',
      JSON.stringify(samplerateSensor),
      { retain: true }
    );

    // HQPlayer dither sensor
    const ditherSensor = {
      name: 'HQPlayer Dither',
      unique_id: 'unified_hifi_hqp_dither',
      state_topic: `${topicPrefix}/hqplayer/status`,
      value_template: '{{ value_json.pipeline.settings.dither.selected.label | default("Unknown", true) }}',
      availability_topic: `${topicPrefix}/hqplayer/status`,
      availability_template: '{{ "online" if value_json.connected else "offline" }}',
      icon: 'mdi:sine-wave',
    };

    client.publish(
      'homeassistant/sensor/unified_hifi_hqp_dither/config',
      JSON.stringify(ditherSensor),
      { retain: true }
    );

    log.info('Published Home Assistant MQTT discovery configs');
  }

  function disconnect() {
    if (publishTimer) {
      clearInterval(publishTimer);
      publishTimer = null;
    }
    if (client) {
      client.end();
      client = null;
    }
  }

  return {
    isEnabled,
    connect,
    disconnect,
    publishHqpState,
  };
}

module.exports = { createMqttService };
