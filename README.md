# DNS Proxy Service

一个功能完整的 DNS 代理服务，支持多种协议和 Web 管理界面。

## 功能特性

### DNS 协议支持
- **UDP DNS** - 标准 DNS 查询 (端口 53)
- **DoH** - DNS over HTTPS (端口 443)
- **DoT** - DNS over TLS (端口 853)
- **DoQ** - DNS over QUIC (端口 853)
- **DoH3** - DNS over HTTP/3 (端口 443)

### 核心功能
- 多上游 DNS 服务器支持
- 查询策略：并发、轮询、随机、最快响应
- DNS 缓存管理
- 域名重写规则
- 本地 DNS 记录
- 查询日志记录

### Web 管理界面
- 仪表盘统计
- DNS 记录管理
- 重写规则配置
- 上游服务器管理
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
- h3/h3-quinn (HTTP/3 支持)

### 前端
- Vue 3
- TypeScript
- Element Plus
- Vite

## 快速开始

### 环境要求
- Rust 1.70+
- Node.js 18+
- pnpm

### 构建后端
```bash
cd backend
cargo build --release
```

### 构建前端
```bash
cd frontend
pnpm install
pnpm build
```

### 运行服务
```bash
cd backend
cargo run --release
```

服务启动后访问 `http://localhost:8080` 进入管理界面。

## 配置

### 环境变量
复制 `backend/.env.example` 为 `backend/.env` 并修改配置：

```env
DATABASE_URL=sqlite:dns_proxy.db?mode=rwc
WEB_PORT=8080
DNS_UDP_PORT=53
LOG_LEVEL=info
```

### 默认账户
- 用户名: `admin`
- 密码: `admin`

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
