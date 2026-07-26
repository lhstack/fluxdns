# FluxDNS

<div align="center">

[![Docker](https://img.shields.io/badge/Docker-lhstack%2Ffluxdns-blue?logo=docker)](https://hub.docker.com/r/lhstack/fluxdns)
[![License](https://img.shields.io/badge/License-Apache%202.0-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange?logo=rust)](https://www.rust-lang.org/)
[![Vue](https://img.shields.io/badge/Vue-3.x-brightgreen?logo=vuedotjs)](https://vuejs.org/)

**一个功能完整的 DNS 代理服务，支持多种协议和现代化 Web 管理界面。**

[English](./README_EN.md) | 中文

</div>

---

## 🎯 演示地址

### [fluxdns.lhstack.xyz](https://fluxdns.lhstack.xyz)

| 项目 | 值 |
|------|-----|
| 用户名 | `admin` |
| 密码 | `admin` |
| DoH 地址 | `https://fluxdns.lhstack.xyz/dns-query` |

**DoH 测试命令:**
```bash
curl "https://fluxdns.lhstack.xyz/dns-query?dns=q80BAAABAAAAAAAAA3d3dwdleGFtcGxlA2NvbQAAAQAB"
```

## 📸 界面预览

<details>
<summary>点击展开截图</summary>

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

## ✨ 功能特性

### 🌐 DNS 协议支持

| 协议 | 端口 | 安全传输 | 状态 |
|------|------|---------|------|
| UDP DNS | 53 | ❌ | ✅ 已实现 |
| DoT (DNS over TLS) | 853 | ✅ | ✅ 已实现 |
| DoH (DNS over HTTPS) | 443 | ✅ | ✅ 已实现 |
| DoQ (DNS over QUIC) | 853 | ✅ | ✅ 已实现 |
| DoH3 (DNS over HTTP/3) | 443 | ✅ | 🚧 开发中 |

### 📡 上游服务器协议

- **UDP** - 标准 DNS 上游
- **DoT** - DNS over TLS 上游 (支持连接复用)
- **DoH** - DNS over HTTPS 上游
- **DoQ** - DNS over QUIC 上游 (支持 Endpoint 复用)
- **DoH3** - DNS over HTTP/3 上游 (支持 Endpoint 复用)

### 🎛️ 核心功能

| 功能 | 描述 |
|------|------|
| 多上游 DNS | 配置多个上游 DNS 服务器 |
| 查询策略 | 并发、轮询、随机、最快响应 |
| DNS 缓存 | 智能缓存管理，支持手动清除 |
| 域名重写 | 支持精确匹配、通配符、正则表达式 |
| 本地记录 | 自定义 DNS 记录，支持泛域名解析 |
| 查询日志 | 详细的查询记录，支持时间范围筛选和导出 |
| 链路追踪 | trace_id 支持，便于问题排查 |

### 📊 实时监控仪表盘

- **QPS 趋势图** - 实时查询量可视化
- **延迟监控** - 响应时间趋势分析
- **上游状态** - 各上游服务器健康状态
- **Top N 统计** - 热门域名和活跃客户端排行
- **SSE 实时推送** - 高性能无锁内存缓存，百万级数据毫秒级响应

### 🔔 告警通知

- **Webhook 推送** - 支持自定义 Webhook URL
- **延迟阈值告警** - 响应超时自动告警
- **测试通知** - 一键测试告警配置

### 🎚️ 动态监听器管理

- **无需重启** - 动态启停各协议监听器
- **TLS 证书配置** - Web 界面上传和管理证书
- **证书信息查看** - 查看证书主题、有效期、颁发者
- **严格校验** - 缺少证书时拒绝启动 TLS 监听器

### 🖥️ Web 管理界面

- 现代化 Vue 3 + Element Plus UI
- 响应式设计，支持移动端
- 深色模式支持
- 实时数据更新

## 🛠️ 技术栈

### 后端
| 组件 | 技术 |
|------|------|
| 语言 | Rust |
| Web 框架 | Axum |
| 数据库 | SQLite |
| DNS 协议 | hickory-proto |
| QUIC | Quinn |
| TLS | rustls + tokio-rustls |
| HTTP/3 | h3 + h3-quinn |

### 前端
| 组件 | 技术 |
|------|------|
| 框架 | Vue 3 |
| 语言 | TypeScript |
| UI 库 | Element Plus |
| 图表 | ECharts |
| 构建 | Vite |

## 🚀 快速开始

### Docker 部署 (推荐)

**前置处理 (解决权限问题):**
```bash
mkdir -p data logs
chown -R 1000:1000 data logs
```

支持架构: `linux/amd64`, `linux/arm64`

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

服务启动后访问 `http://localhost:8080` 进入管理界面。

### 源码构建

#### 环境要求
- Rust 1.70+
- Node.js 18+
- pnpm

#### 构建后端
```bash
cd backend
cargo build --release
```

#### 构建前端
```bash
cd frontend
pnpm install
pnpm build
```

#### 运行服务
```bash
cd backend
cargo run --release
```

## ⚙️ 配置

FluxDNS 采用分层配置方式：
- **数据库配置** (通过 Web 界面管理): DNS 监听器、上游服务器、缓存设置、查询策略
- **文件/环境变量配置**: 数据库路径、Web 端口、管理员账户、日志设置

### 环境变量

复制 `backend/.env.example` 为 `backend/.env` 并修改配置：

```env
# 数据库
DATABASE_URL=sqlite:fluxdns.db?mode=rwc

# Web 管理端口
WEB_PORT=8080

# 管理员账户 (生产环境请修改!)
ADMIN_USERNAME=admin
ADMIN_PASSWORD=admin

# 日志配置
LOG_PATH=logs
LOG_LEVEL=info
LOG_MAX_SIZE=10485760
LOG_RETENTION_DAYS=30

```

### 默认账户
- 用户名: `admin`
- 密码: `admin`

> ⚠️ **安全提示**: 生产环境请务必修改默认密码！

## 📖 使用指南

### DNS 记录配置

#### 泛域名解析
支持使用 `*` 通配符配置泛域名解析：

| 记录名称 | 类型 | 值 | 匹配示例 |
|---------|------|-----|---------|
| `*.example.com` | A | 192.168.1.100 | `sub.example.com`, `www.example.com` |
| `*.example.com` | A | 192.168.1.100 | `a.b.example.com` (多级子域名) |
| `example.com` | A | 192.168.1.1 | `example.com` (精确匹配优先) |

**匹配优先级：**
1. 精确匹配优先于泛域名匹配
2. 更具体的泛域名优先 (`*.sub.example.com` > `*.example.com`)

### 上游服务器配置示例

| 协议 | 地址示例 |
|------|---------|
| UDP | `8.8.8.8:53`, `1.1.1.1:53` |
| DoT | `dns.google:853`, `cloudflare-dns.com:853` |
| DoH | `https://dns.google/dns-query` |
| DoQ | `dns.adguard.com:853`, `94.140.14.14:853` |
| DoH3 | `https://dns.adguard-dns.com/dns-query` |

### TLS 证书配置

DoT、DoH、DoQ 等 TLS 协议需要配置证书：

1. 在 **服务监听配置** 页面选择目标协议
2. 点击 **配置证书** 上传 PEM 格式证书
3. 点击 **配置私钥** 上传 PEM 格式私钥
4. 证书配置完成后可点击 **查看** 检查证书信息
5. 开启监听器开关启动服务

## 🔌 API 端点

### DoH 查询

```bash
# GET 方式
curl -H "Accept: application/dns-message" \
  "http://localhost:8080/dns-query?dns=<base64url-encoded-query>"

# POST 方式
curl -X POST \
  -H "Content-Type: application/dns-message" \
  -H "Accept: application/dns-message" \
  --data-binary @query.bin \
  "http://localhost:8080/dns-query"
```

### 管理 API

所有管理 API 需要 JWT 认证，前缀为 `/api/`：

| 端点 | 描述 |
|------|------|
| `/api/records` | DNS 记录管理 |
| `/api/rewrite` | 重写规则管理 |
| `/api/upstreams` | 上游服务器管理 |
| `/api/cache` | 缓存管理 |
| `/api/logs` | 查询日志 (支持导出) |
| `/api/status` | 系统状态 |
| `/api/strategy` | 查询策略 |
| `/api/listeners` | 服务监听配置 |
| `/api/stats/stream` | 实时统计数据 (SSE) |
| `/api/stats/top-domains` | Top N 热门域名 |
| `/api/stats/top-clients` | Top N 活跃客户端 |

## 📝 更新日志

### v1.1.7 (Latest)
- 🏗️ **架构重构** - 按 Application、Business、Infrastructure 重新组织后端职责与依赖方向，移除旧目录和内置 AI 助手模块
- 🚀 **DNS 热路径优化** - 将禁用记录类型和本地记录存在性改为内存状态，缓存命中不再执行每查询 SQLite 读取
- 💾 **缓存优化** - 按响应最小 TTL 缓存并限制在 5～3600 秒，优化缓存键分配、重写匹配和近似 LRU 淘汰
- 📝 **查询日志优化** - 使用有界队列批量写入 SQLite，移除每查询无界任务与单条事务，并暴露日志丢弃计数
- 🌐 **上游稳定性** - 完善健康探测、失败恢复、连接复用和查询策略统计，降低默认日志级别下的逐查询开销
- 🐳 **容器运行优化** - 使用 tini 转发退出信号并回收僵尸进程，明确暴露 DoQ/DoH3 所需的 UDP 端口

### v1.1.6
- 🚀 **连接稳定性优化** - 为 UDP 实现 Endpoint Pool (10 socket)，为 DoQ/DoH3 实现连接池 (10 连接)，大幅提升高并发下的稳定性
- ✨ **DoQ 深度优化** - 启用 QUIC Keep-Alive (5s) 和空闲超时机制，彻底解决连接丢失问题
- 🐛 **DoH3 修复** - 修复因缺少请求头导致的 500 错误，并实现连接复用
- 📊 **监控增强** - 增加全局失败计数器，优化上游统计算法

### v1.1.5
- ✨ **仪表盘重构** - 全新设计的功能介绍首页，更清晰展示核心功能
- 🐛 **监听器修复** - 修复监听器开启时可能无法启动的问题
- 🐛 **证书管理** - 修复 TLS 证书无法清除及状态显示不正确的问题

### v1.1.4
- 🚀 **性能优化** - 重构 SSE 实时统计接口，引入内存原子缓存，支持百万级数据毫秒级响应
- ✨ **IPv6 支持** - 上游 DNS 服务器现已完整支持 IPv6 地址配置
- 🐛 **UI 修复** - 修复查询日志和上游列表在特定分辨率下的内容溢出问题

### v1.1.3
- ✨ 动态监听器管理 (无需重启)
- ✨ DoH 真正的 HTTPS 支持
- ✨ TLS 证书信息查看
- ✨ 证书编辑回显功能
- 🐛 修复严格启动校验逻辑

### v1.1.2
- ✨ AI 智能助手
- ✨ 实时监控仪表盘
- ✨ Top N 统计
- ✨ 告警通知功能
- ✨ 日志导出功能

## 📄 许可证

[Apache License 2.0](LICENSE)

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！
