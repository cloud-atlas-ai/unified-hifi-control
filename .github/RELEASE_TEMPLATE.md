## Installation

### Docker

```yaml
services:
  unified-hifi-control:
    image: muness/unified-hifi-control:{{VERSION}}
    network_mode: host
    volumes:
      - ./data:/data
    environment:
      - CONFIG_DIR=/data
    restart: unless-stopped
```

```bash
docker compose up -d
# Access http://localhost:8088
```

### LMS Plugin

Add this repository URL in LMS Settings → Plugins → Additional Repositories:
```
https://raw.githubusercontent.com/open-horizon-labs/unified-hifi-control/v3/lms-plugin/repo.xml
```
Then install "Unified Hi-Fi Control" from the plugin list.

### Synology / QNAP

Download the SPK or QPKG package from the assets below.

---

## MCP Server (Claude Integration)

```json
{
  "mcpServers": {
    "hifi": {
      "command": "npx",
      "args": ["unified-hifi-control-mcp"],
      "env": {
        "HIFI_BRIDGE_URL": "http://localhost:8088"
      }
    }
  }
}
```

---

## Configuration

Configure all backends (Roon, LMS, HQPlayer, UPnP/OpenHome) via the web UI at `/settings`.
