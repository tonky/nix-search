FROM debian:stable-slim AS builder
WORKDIR /workspace

ENV DEBIAN_FRONTEND=noninteractive \
    CARGO_HOME=/root/.cargo \
    RUSTUP_HOME=/root/.rustup \
    PATH=/root/.cargo/bin:/usr/local/bin:/usr/local/sbin:/usr/bin:/usr/sbin:/bin:/sbin

RUN apt-get update && apt-get install -y --no-install-recommends \
      bash \
      ca-certificates \
      curl \
      build-essential \
      pkg-config \
      libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
RUN rustup target add wasm32-unknown-unknown

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates
COPY src ./src

RUN cargo build -p ci --release --locked

FROM debian:stable-slim

ENV DEBIAN_FRONTEND=noninteractive \
    CARGO_HOME=/root/.cargo \
    RUSTUP_HOME=/root/.rustup \
    PATH=/root/.cargo/bin:/usr/local/bin:/usr/local/sbin:/usr/bin:/usr/sbin:/bin:/sbin

RUN apt-get update && apt-get install -y --no-install-recommends \
            bash \
            brotli \
            ca-certificates \
            curl \
            git \
            jq \
            libssl-dev \
            pkg-config \
            build-essential \
        && rm -rf /var/lib/apt/lists/*

RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
RUN rustup target add wasm32-unknown-unknown \
    && rm -rf /root/.rustup/downloads /root/.rustup/tmp
RUN cargo install trunk --version 0.21.14 --locked \
    && rm -rf /root/.cargo/registry /root/.cargo/git

COPY --from=builder /workspace/target/release/ci /usr/local/bin/ci

WORKDIR /workspace
ENTRYPOINT ["ci"]
