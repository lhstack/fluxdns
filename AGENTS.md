# AGENTS.md

## 项目概述

- FluxDNS 是一个 Rust 编写的 DNS 代理服务，同时提供 Web 管理界面。
- 后端支持多协议 DNS 服务端（UDP、DoT、DoH、DoQ；DoH3 在 `listener_manager` 中显式返回未实现）与多协议上游客户端（UDP、DoT、DoH、DoQ、DoH3）。
- 已有能力包括本地 DNS 记录、重写规则、响应缓存、上游健康与查询策略、查询日志、系统设置、Webhook 告警和内置 LLM 助手。
- 仓库为前后端分离的单仓：`backend/` 是单个 Cargo package（`fluxdns` v1.1.6），`frontend/` 是 Vue 3 + Vite 应用（package version 1.1.4，与后端版本不一致）。

## 工程环境与主要工具

- Rust edition 2021；未在 `Cargo.toml` 中声明 rust-version。
- 异步运行时与服务端使用 Tokio、Axum 0.7、Hyper 1、tower-http。
- DNS 协议使用 hickory-proto 0.25；QUIC 使用 quinn 0.11；HTTP/3 使用 git 依赖 `hyperium/h3`（branch = "master"，未固定 commit）。
- TLS 使用 rustls 0.23（ring）、tokio-rustls、rustls-pemfile；上游 DoH 使用 reqwest 0.11（rustls-tls）。
- 数据库使用 sqlx 0.7 + SQLite，运行时 `PRAGMA journal_mode=WAL`，迁移以代码内 `CREATE TABLE IF NOT EXISTS` 方式执行，无独立 migrations 目录。
- 认证使用 jsonwebtoken 9；`bcrypt` 已在依赖中声明但源码中无任何调用。
- 前端静态资源通过 rust-embed 从 `backend/dist` 嵌入二进制。
- 未发现 `rustfmt.toml` 或 `clippy.toml`，使用工具链默认格式约定。
- CI 只有 `.github/workflows/docker-build.yml`，用于构建并推送 Docker 镜像；未发现 cargo test / clippy 的 CI 环节。

## 目录与模块结构

- `backend/src/main.rs`：binary crate 入口，只声明模块并调用 `bootstrap::run()`。
- `backend/src/bootstrap.rs`：启动装配，负责配置、日志、数据库、DNS 组件、监听器、路由与关闭信号。
- `backend/src/config/`：`AppConfig` / `PartialConfig` / `ConfigManager`，环境变量优先于 `config.toml`，再回落默认值。
- `backend/src/state.rs`：`AppState`，聚合全部共享组件，仅被 LLM 模块与 `AlertManager` 使用。
- `backend/src/db/`：`Database` 门面、`models.rs` 数据模型、`repository.rs` 各仓储、`stats_cache.rs` 内存统计计数。
- `backend/src/dns/`：`message.rs` 报文编解码、`cache.rs` 响应缓存、`rewrite.rs` 重写引擎、`resolver.rs` 解析主流程、`proxy/` 上游管理与客户端与策略、`server/` 各协议服务端。
- `backend/src/web/`：Axum 路由与 REST handler，每个业务域一个文件并各自持有独立 State 结构。
- `backend/src/services/`：`listener_manager.rs` 监听器生命周期、`alert_manager.rs` 延迟告警轮询。
- `backend/src/llm/`：LLM 客户端、配置与 `functions/` 下的工具函数实现。
- `backend/src/log/`：日志初始化、轮转与清理。
- `frontend/src/`：Vue 管理界面；`backend/dist` 是被嵌入的前端产物。

## 分层架构与依赖方向

- 实际依赖方向为 `web/server -> dns::resolver -> {rewrite, cache, proxy, db}`，以及 `web -> db`。
- `bootstrap` 是唯一的装配点，负责构造全部 `Arc` 组件并注入各 State。
- `web` 层的每个模块自带 State，直接持有 `Arc<Database>` 与对应的 DNS 组件，没有独立的业务用例层。
- `dns::resolver` 是解析主流程的唯一编排者，按 域名校验 -> 禁用类型 -> 重写 -> 本地记录 -> 缓存 -> 上游 的顺序执行。
- `dns::proxy::strategy` 负责策略选择与失败转移，`dns::proxy::client` 负责单协议传输，`dns::proxy::upstream` 负责服务器集合、统计与健康状态。
- `db::repository` 只做 SQL 访问；`Database` 的 `xxx()` 方法每次新建仓储实例并克隆连接池。
- `llm::functions` 是一个例外：它直接持有 `AppState` 并用 `state.db.pool()` 执行原始 SQL，绕过了 `db::repository`。这是当前事实结构，不代表推荐方向。

## 构建、测试和验证方式

- 后端构建：在 `backend/` 执行 `cargo build`；发布构建 `cargo build --release`。
- 前端构建：在 `frontend/` 执行 `pnpm install` 与 `pnpm build`（`vue-tsc -b && vite build`）。
- 后端测试全部为内联 `#[cfg(test)] mod tests`，另有 `dns/proxy/forwarding_tests.rs` 与 `dns/server/protocol_consistency_tests.rs` 两个 `#[cfg(test)]` 模块；无根级 `tests/` 目录。
- dev-dependencies 含 proptest、tokio-test、tempfile；仓储测试使用 `tempfile` 建临时 SQLite。
- 前端 `package.json` 只声明 `dev`、`build`、`preview`，无测试或 lint 脚本。
- 修改 Rust 文件后应执行格式化、IDE inspection，并按改动范围执行项目构建与相关测试。

## 项目编码约定

- 模块通过 `mod.rs` 声明并大量使用 `pub use xxx::*` 通配重导出。
- 结构体与方法普遍带 `///` 文档注释；`web` 层 handler 的注释中会写出 HTTP 方法与路径。
- 枚举普遍手写 `from_str(&str) -> Option<Self>` 与 `as_str(&self) -> &'static str`，并基于 `as_str` 实现 `Display`；注意这些 `from_str` 是自定义关联函数，不是 `std::str::FromStr`。
- 命名遵循 Rust snake_case / CamelCase；方法名表达意图（`get_healthy_servers`、`needs_reprobe`、`check_local_records`）。
- 组件普遍以 `new()` / `with_db()` / `new_shared()` 三件套构造，其中 `new_shared()` 在多个组件中已无调用方。
- 共享可变状态使用 `tokio::sync::RwLock`；缓存使用 `DashMap` 加原子计数；`ConfigManager` 使用 `std::sync::RwLock`。
- 现存大量单点调用的短方法（如 `Database::pool()`、`CacheManager::get_ttl()`、`DnsResolver::resolve_without_rewrite()`）以及 55 处 `#[allow(dead_code)]`；这是现状，不是应当扩散的约定。

## 错误处理约定

- 应用层错误以 `anyhow::Result` 为主；`error.rs` 定义了 `AppError` / `AppResult`，但整体标注 `#![allow(dead_code)]`，实际只在 `web::auth` 中使用 `AppError::Auth`。
- `web` 层统一返回自定义 `ApiError { code, message, details }`，由 `code` 字符串映射 HTTP 状态码。
- DNS 服务端在解析失败或解析器报错时返回 SERVFAIL 报文，不向调用方传播错误。
- 存在较多 `unwrap_or` / `unwrap_or_default` 形式的静默降级，尤其在读取 `system_config` 与告警设置处；这是现状，新代码不应继续扩大。
- 后台任务（日志清理、告警、监听器）通过 `tokio::spawn` 启动，多数只在内部 `tracing::error!` 记录，没有统一的生命周期与失败上报。

## 版本控制信息

- 使用 Git，当前 40 个提交全部由 `lhstack <lhstack@foxmail.com>` 提交。
- 提交信息以中英双语的 Conventional Commits 为主（`feat:`、`fix:`、`docs:`、`chore:`），也存在纯中文标题。
- `.gitignore` 忽略 `backend/target/`、`backend/*.db`、`backend/logs/`、`frontend/node_modules/`、`frontend/dist/`、`.idea/`、`.vscode/`、`.kiro/`；`git ls-files` 确认数据库文件未被跟踪。

## 有证据支持的用户编码习惯

仓库提交身份单一，以下为基于该身份提交代码的可观察习惯：

- 偏好在同一文件内集中一个业务域的模型、State、handler 与测试。
- 偏好把可调参数写成模块内 `const`（`EMA_ALPHA`、`ENDPOINT_POOL_SIZE`、`MAX_DEPTH`、`TOKEN_EXPIRATION_HOURS`）。
- 在关键路径上写密集的 `debug!` / `info!` 日志，并在上游查询中用 `trace_id` 串联；部分日志文案为中英混排。
- 倾向以内存缓存与连接池换取性能（`StatsCache`、DoT 连接池、QUIC endpoint 池、`ProxyManager` 客户端缓存）。
- 倾向在函数内部就地 `use`（如 `use tracing::info;` 写在方法体首行）而非集中在文件头部。
- 倾向以代码注释解释历史决策，包括说明为何移除某项检查（见 `listener_manager::start_listener` 中关于 enabled 检查的注释）。

## 当前无法确认的事项

- 未确认的强制质量门禁：无 clippy 配置、无测试 CI，实际发布前的验证流程未确认。
- `bcrypt` 依赖的用途未确认：源码中无调用，密码为明文比较。
- 前后端版本号不一致（后端 1.1.6 / 前端 1.1.4）是否有意为之未确认。
- `h3` / `h3-quinn` 使用 git master 分支的锁定与升级策略未确认。
- `backend/dns_proxy.db` 与 `backend/fluxdns.db` 两个本地数据库文件的用途区分未确认。
