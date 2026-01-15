# FluxDNS

一个功能完整的 DNS 代理服务，支持多种协议和 Web 管理界面。
## 演示图
![1](./images/1.png)
![2](./images/2.png)
![3](./images/3.png)
![4](./images/4.png)
![5](./images/5.png)
![6](./images/6.png)
![7](./images/7.png)
![8](./images/8.png)
![9](./images/9.png)
![10](./images/10.png)
## 功能特性
### DNS 协议支持
- **UDP DNS** - 标准 DNS 查询 (端口 53)
- **DoH** - DNS over HTTPS (端口 443)
- **DoT** - DNS over TLS (端口 853)
- **DoQ** - DNS over QUIC (端口 853)
- **DoH3** - DNS over HTTP/3 (端口 443)

### 上游服务器协议支持
- **UDP** - 标准 DNS 上游
- **DoT** - DNS over TLS 上游 (支持连接复用)
- **DoH** - DNS over HTTPS 上游
- **DoQ** - DNS over QUIC 上游 (支持 Endpoint 复用)
- **DoH3** - DNS over HTTP/3 上游 (支持 Endpoint 复用)

### 核心功能
- 多上游 DNS 服务器支持
- 查询策略：并发、轮询、随机、最快响应
- DNS 缓存管理
- 域名重写规则
- 本地 DNS 记录 (支持泛域名解析)
- 查询日志记录
- 链路追踪 (trace_id) 支持

### Web 管理界面
- 仪表盘统计
- DNS 记录管理
- 重写规则配置
- 上游服务器管理 (支持分页)
- 缓存管理
- 查询日志查看
- 服务监听配置
- 系统设置

## 技术栈

### 后端
- Rust
- Axum (Web 框架)
- SQLite (数据库)
- trust-dns-proto (DNS 协议)
- Quinn (QUIC 支持)
- rustls (TLS 支持)
- h3/h3-quinn (HTTP/3 支持)

### 前端
- Vue 3
- TypeScript
- Element Plus
- Vite

## 快速开始

### Docker 部署 (推荐)

支持架构: `linux/amd64`, `linux/arm64`

#### Docker Compose

```yaml
version: '3.8'

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

服务启动后访问 `http://localhost:8080` 进入管理界面。

## 配置

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

### Web 界面配置项
以下配置通过 Web 管理界面进行管理：
- **服务监听**: UDP/DoT/DoH/DoQ/DoH3 的端口、绑定地址、TLS 证书
- **上游服务器**: 添加/编辑/删除上游 DNS 服务器
- **缓存设置**: TTL、最大条目数
- **查询策略**: 并发/轮询/随机/最快

## DNS 记录配置

### 泛域名解析
支持使用 `*` 通配符配置泛域名解析，匹配所有子域名：

| 记录名称 | 类型 | 值 | 匹配示例 |
|---------|------|-----|---------|
| `*.example.com` | A | 192.168.1.100 | `sub.example.com`, `www.example.com` |
| `*.example.com` | A | 192.168.1.100 | `a.b.example.com` (多级子域名) |
| `example.com` | A | 192.168.1.1 | `example.com` (精确匹配优先) |

匹配优先级：
1. 精确匹配优先于泛域名匹配
2. 更具体的泛域名优先 (`*.sub.example.com` > `*.example.com`)

## 上游服务器配置示例

### UDP
```
8.8.8.8:53
1.1.1.1:53
```

### DoT (DNS over TLS)
```
dns.google:853
cloudflare-dns.com:853
```

### DoH (DNS over HTTPS)
```
https://dns.google/dns-query
https://cloudflare-dns.com/dns-query
```

### DoQ (DNS over QUIC)
```
dns.adguard.com:853
94.140.14.14:853
```

### DoH3 (DNS over HTTP/3)
```
https://dns.adguard-dns.com/dns-query
```

## API 端点

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
- `/api/records` - DNS 记录管理
- `/api/rewrite` - 重写规则管理
- `/api/upstreams` - 上游服务器管理
- `/api/cache` - 缓存管理
- `/api/logs` - 查询日志
- `/api/status` - 系统状态
- `/api/strategy` - 查询策略
- `/api/listeners` - 服务监听配置

## 许可证

Apache License 2.0
