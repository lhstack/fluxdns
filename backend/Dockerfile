# 后端构建 Dockerfile - 支持 amd64 和 arm64
FROM rust:1.83-alpine AS builder

# 设置时区和语言
ENV TZ=Asia/Shanghai \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8

RUN apk add --no-cache tzdata musl-dev pkgconfig openssl-dev openssl-libs-static git && \
    cp /usr/share/zoneinfo/$TZ /etc/localtime && \
    echo $TZ > /etc/timezone

WORKDIR /app

# 复制所有源码和依赖文件
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# 复制前端构建产物
COPY dist ./dist

# 构建
RUN cargo build --release

# 运行阶段
FROM alpine:3.20

# 设置时区和语言
ENV TZ=Asia/Shanghai \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8

RUN apk add --no-cache tzdata ca-certificates && \
    cp /usr/share/zoneinfo/$TZ /etc/localtime && \
    echo $TZ > /etc/timezone

# 创建非 root 用户
RUN addgroup -g 1000 fluxdns && \
    adduser -u 1000 -G fluxdns -s /bin/sh -D fluxdns

WORKDIR /app

# 复制二进制文件
COPY --from=builder /app/target/release/fluxdns /app/fluxdns

# 创建数据目录并设置权限
RUN mkdir -p /app/data /app/logs && \
    chown -R fluxdns:fluxdns /app && \
    chmod +x /app/fluxdns

# 切换用户
USER fluxdns

# 暴露端口
EXPOSE 8080 53/udp 53/tcp 853 443

# 启动命令
CMD ["/app/fluxdns"]
