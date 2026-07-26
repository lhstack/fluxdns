# FluxDNS

<div align="center">

[![Docker](https://img.shields.io/badge/Docker-lhstack%2Ffluxdns-blue?logo=docker)](https://hub.docker.com/r/lhstack/fluxdns)
[![License](https://img.shields.io/badge/License-Apache%202.0-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange?logo=rust)](https://www.rust-lang.org/)
[![Vue](https://img.shields.io/badge/Vue-3.x-brightgreen?logo=vuedotjs)](https://vuejs.org/)

**A fully-featured DNS proxy service with multi-protocol support and a modern web management interface.**

English | [中文](./README.md)

</div>

---

## 🎯 Demo

### [fluxdns.lhstack.xyz](https://fluxdns.lhstack.xyz)

| Item | Value |
|------|-------|
| Username | `admin` |
| Password | `admin` |
| DoH URL | `https://fluxdns.lhstack.xyz/dns-query` |

**DoH Test Command:**
```bash
curl "https://fluxdns.lhstack.xyz/dns-query?dns=q80BAAABAAAAAAAAA3d3dwdleGFtcGxlA2NvbQAAAQAB"
```

## 📸 Screenshots

<details>
<summary>Click to expand</summary>

![Dashboard](./images/1.png)
![DNS Records](./images/2.png)
![Rewrite Rules](./images/3.png)
![Upstream Servers](./images/4.png)
![Query Logs](./images/5.png)
![Listeners](./images/6.png)
![Settings](./images/7.png)
![Cache](./images/8.png)
![Real-time Monitor](./images/10.png)

</details>

## ✨ Features

### 🌐 DNS Protocol Support

| Protocol | Port | Secure Transport | Status |
|----------|------|------------------|--------|
| UDP DNS | 53 | ❌ | ✅ Implemented |
| DoT (DNS over TLS) | 853 | ✅ | ✅ Implemented |
| DoH (DNS over HTTPS) | 443 | ✅ | ✅ Implemented |
| DoQ (DNS over QUIC) | 853 | ✅ | ✅ Implemented |
| DoH3 (DNS over HTTP/3) | 443 | ✅ | 🚧 In Development |

### 📡 Upstream Server Protocols

- **UDP** - Standard DNS upstream
- **DoT** - DNS over TLS upstream (connection reuse supported)
- **DoH** - DNS over HTTPS upstream
- **DoQ** - DNS over QUIC upstream (endpoint reuse supported)
- **DoH3** - DNS over HTTP/3 upstream (endpoint reuse supported)

### 🎛️ Core Features

| Feature | Description |
|---------|-------------|
| Multi-Upstream DNS | Configure multiple upstream DNS servers |
| Query Strategies | Concurrent, Round-robin, Random, Fastest response |
| DNS Cache | Smart cache management with manual purge |
| Domain Rewrite | Exact match, Wildcard, and Regex support |
| Local Records | Custom DNS records with wildcard support |
| Query Logs | Detailed query logs with time range filtering and export |
| Request Tracing | trace_id support for troubleshooting |

### 📊 Real-time Dashboard

- **QPS Trend Chart** - Real-time query volume visualization
- **Latency Monitoring** - Response time trend analysis
- **Upstream Status** - Health status of each upstream server
- **Top N Statistics** - Popular domains and active clients ranking
- **SSE Real-time Push** - High-performance in-memory caching, sub-millisecond latency for massive datasets

### 🔔 Alert Notifications

- **Webhook Push** - Custom webhook URL support
- **Latency Threshold Alerts** - Automatic alerts on response timeout
- **Test Notifications** - One-click alert configuration testing

### 🎚️ Dynamic Listener Management

- **No Restart Required** - Dynamically start/stop protocol listeners
- **TLS Certificate Configuration** - Upload and manage certificates via web UI
- **Certificate Info Viewer** - View certificate subject, validity, issuer
- **Strict Validation** - Refuses to start TLS listeners without certificates

### 🖥️ Web Management Interface

- Modern Vue 3 + Element Plus UI
- Responsive design with mobile support
- Dark mode support
- Real-time data updates

## 🛠️ Tech Stack

### Backend
| Component | Technology |
|-----------|------------|
| Language | Rust |
| Web Framework | Axum |
| Database | SQLite |
| DNS Protocol | hickory-proto |
| QUIC | Quinn |
| TLS | rustls + tokio-rustls |
| HTTP/3 | h3 + h3-quinn |

### Frontend
| Component | Technology |
|-----------|------------|
| Framework | Vue 3 |
| Language | TypeScript |
| UI Library | Element Plus |
| Charts | ECharts |
| Build Tool | Vite |

## 🚀 Quick Start

### Docker Deployment (Recommended)

**Prerequisites (fix permission issues):**
```bash
mkdir -p data logs
chown -R 1000:1000 data logs
```

Supported architectures: `linux/amd64`, `linux/arm64`

#### Docker Compose

```yaml
services:
  fluxdns:
    image: lhstack/fluxdns:latest
    container_name: fluxdns
    restart: unless-stopped
    user: "1000:1000"
    environment:
      - TZ=Asia/Shanghai
      - DATABASE_URL=sqlite:/app/data/fluxdns.db?mode=rwc
      - WEB_PORT=8080
      - ADMIN_USERNAME=admin
      - ADMIN_PASSWORD=admin
      - LOG_PATH=/app/logs
      - LOG_LEVEL=info
    logging:
      options:
        max-file: "2"
        max-size: '32k'
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: '16M'
    ports:
      - "8080:8080"
      - "53:53/udp"
      - "53:53/tcp"
      - "853:853"
      - "443:443"
    volumes:
      - ./data:/app/data
      - ./logs:/app/logs
    cap_add:
      - NET_BIND_SERVICE
```

```bash
docker-compose up -d
```

#### Docker Run

```bash
docker run -d \
  --name fluxdns \
  --restart unless-stopped \
  --user 1000:1000 \
  -e TZ=Asia/Shanghai \
  -e ADMIN_USERNAME=admin \
  -e ADMIN_PASSWORD=admin \
  -p 8080:8080 \
  -p 53:53/udp \
  -p 53:53/tcp \
  -p 853:853 \
  -p 443:443 \
  -v ./data:/app/data \
  -v ./logs:/app/logs \
  --cap-add NET_BIND_SERVICE \
  lhstack/fluxdns:latest
```

After startup, access `http://localhost:8080` to enter the management interface.

### Build from Source

#### Requirements
- Rust 1.70+
- Node.js 18+
- pnpm

#### Build Backend
```bash
cd backend
cargo build --release
```

#### Build Frontend
```bash
cd frontend
pnpm install
pnpm build
```

#### Run Service
```bash
cd backend
cargo run --release
```

## ⚙️ Configuration

FluxDNS uses layered configuration:
- **Database Configuration** (via Web UI): DNS listeners, upstream servers, cache settings, query strategies
- **File/Environment Configuration**: Database path, web port, admin account, log settings

### Environment Variables

Copy `backend/.env.example` to `backend/.env` and modify:

```env
# Database
DATABASE_URL=sqlite:fluxdns.db?mode=rwc

# Web Management Port
WEB_PORT=8080

# Admin Account (change in production!)
ADMIN_USERNAME=admin
ADMIN_PASSWORD=admin

# Log Configuration
LOG_PATH=logs
LOG_LEVEL=info
LOG_MAX_SIZE=10485760
LOG_RETENTION_DAYS=30

```

### Default Credentials
- Username: `admin`
- Password: `admin`

> ⚠️ **Security Notice**: Please change the default password in production!

## 📖 User Guide

### DNS Record Configuration

#### Wildcard DNS
Use `*` wildcard for subdomain matching:

| Record Name | Type | Value | Matches |
|-------------|------|-------|---------|
| `*.example.com` | A | 192.168.1.100 | `sub.example.com`, `www.example.com` |
| `*.example.com` | A | 192.168.1.100 | `a.b.example.com` (multi-level) |
| `example.com` | A | 192.168.1.1 | `example.com` (exact match priority) |

**Match Priority:**
1. Exact matches take priority over wildcards
2. More specific wildcards take priority (`*.sub.example.com` > `*.example.com`)

### Upstream Server Examples

| Protocol | Address Example |
|----------|-----------------|
| UDP | `8.8.8.8:53`, `1.1.1.1:53` |
| DoT | `dns.google:853`, `cloudflare-dns.com:853` |
| DoH | `https://dns.google/dns-query` |
| DoQ | `dns.adguard.com:853`, `94.140.14.14:853` |
| DoH3 | `https://dns.adguard-dns.com/dns-query` |

### TLS Certificate Configuration

DoT, DoH, DoQ and other TLS protocols require certificates:

1. Go to **Listener Configuration** page and select target protocol
2. Click **Configure Certificate** to upload PEM format certificate
3. Click **Configure Private Key** to upload PEM format private key
4. After configuration, click **View** to check certificate info
5. Toggle the listener switch to start the service

## 🔌 API Endpoints

### DoH Query

```bash
# GET Method
curl -H "Accept: application/dns-message" \
  "http://localhost:8080/dns-query?dns=<base64url-encoded-query>"

# POST Method
curl -X POST \
  -H "Content-Type: application/dns-message" \
  -H "Accept: application/dns-message" \
  --data-binary @query.bin \
  "http://localhost:8080/dns-query"
```

### Management API

All management APIs require JWT authentication with `/api/` prefix:

| Endpoint | Description |
|----------|-------------|
| `/api/records` | DNS record management |
| `/api/rewrite` | Rewrite rule management |
| `/api/upstreams` | Upstream server management |
| `/api/cache` | Cache management |
| `/api/logs` | Query logs (with export) |
| `/api/status` | System status |
| `/api/strategy` | Query strategy |
| `/api/listeners` | Listener configuration |
| `/api/stats/stream` | Real-time statistics (SSE) |
| `/api/stats/top-domains` | Top N popular domains |
| `/api/stats/top-clients` | Top N active clients |

## 📝 Changelog

### v1.1.7 (Latest)
- 🏗️ **Architecture Refactoring** - Reorganized backend responsibilities and dependency direction into Application, Business, and Infrastructure layers; removed legacy directories and the built-in AI assistant
- 🚀 **DNS Hot-Path Optimization** - Moved disabled record types and local-record availability into memory so cache hits no longer perform per-query SQLite reads
- 💾 **Cache Optimization** - Cache entries now honor the minimum response TTL within a 5–3600 second range, with lower allocation overhead and improved approximate-LRU eviction
- 📝 **Query Log Optimization** - Replaced unbounded per-query tasks and single-row transactions with bounded, batched SQLite writes, and exposed dropped-log counters
- 🌐 **Upstream Stability** - Improved health probing, failure recovery, connection reuse, and query-strategy statistics while reducing per-query work at the default log level
- 🐳 **Container Runtime** - Added tini for signal forwarding and zombie reaping, and explicitly exposed the UDP ports required by DoQ and DoH3

### v1.1.6
- 🚀 **Connection Stability** - Implemented Endpoint Pool (10 sockets) for UDP and Connection Pool (10 conns) for DoQ/DoH3, significantly improving stability under high concurrency
- ✨ **DoQ Optimization** - Enabled QUIC Keep-Alive (5s) and idle timeout to resolve connection loss issues
- 🐛 **DoH3 Fixes** - Fixed 500 errors caused by missing headers and implemented connection reuse
- 📊 **Enhanced Monitoring** - Added global failure counter and tuned upstream statistics algorithm

### v1.1.5
- ✨ **Dashboard Redesign** - Totally redesigned homepage with clearer feature showcase
- 🐛 **Listener Fixes** - Fixed an issue where listeners might fail to start when toggled on
- 🐛 **Certificate Management** - Fixed issues with TLS certificate clearing and incorrect status display

### v1.1.4
- 🚀 **Performance Optimization** - Refactored SSE real-time stats with in-memory atomic caching, enabling millisecond-level response
- ✨ **IPv6 Support** - Full IPv6 address support for upstream DNS servers
- 🐛 **UI Fixes** - Fixed content overflow issues in query logs and upstream lists

### v1.1.3
- ✨ Dynamic listener management (no restart required)
- ✨ DoH with real HTTPS support
- ✨ TLS certificate info viewer
- ✨ Certificate edit pre-fill feature
- 🐛 Fix strict startup validation logic

### v1.1.2
- ✨ AI assistant
- ✨ Real-time monitoring dashboard
- ✨ Top N statistics
- ✨ Alert notifications
- ✨ Log export feature

## 📄 License

[Apache License 2.0](LICENSE)

## 🤝 Contributing

Issues and Pull Requests are welcome!
