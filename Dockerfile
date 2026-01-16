# 基于 pdf2htmlEX 镜像
FROM pdf2htmlex/pdf2htmlex:0.18.8.rc2-master-20200820-ubuntu-20.04-x86_64

# 安装 Rust 和必要的构建工具
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    build-essential \
    ca-certificates \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 安装 Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /build
COPY Cargo.toml ./
COPY src ./src

# 编译
RUN cargo build --release

# 设置运行环境
WORKDIR /app
RUN cp /build/target/release/pdf2html-web /app/
COPY static /app/static

# 创建必要的目录
RUN mkdir -p /tmp/pdf2html/output

EXPOSE 8080

ENTRYPOINT []
CMD ["/app/pdf2html-web"]
