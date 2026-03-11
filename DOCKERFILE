# NewClaw Dockerfile (可选)
# 用于容器化部署

FROM rust:1.75-slim as builder

WORKDIR /app

# 安装依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 复制源代码
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# 构建
RUN cargo build --release

# 运行时镜像
FROM debian:bookworm-slim

WORKDIR /app

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# 复制二进制文件
COPY --from=builder /app/target/release/newclaw /usr/local/bin/newclaw

# 复制配置文件
COPY config.example.toml /etc/newclaw/config.toml

# 创建非 root 用户
RUN useradd -r -s /bin/false newclaw
USER newclaw

# 暴露端口
EXPOSE 3000

# 健康检查
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# 启动
CMD ["newclaw", "gateway"]
