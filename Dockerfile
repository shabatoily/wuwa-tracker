# syntax=docker/dockerfile:1.7

FROM rust:alpine AS build

WORKDIR /wuwa-tracker

RUN apk add --no-cache make gcompat

RUN rustup toolchain install 1.96.0 --profile minimal --target wasm32-unknown-unknown

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/local/cargo/trunk-target \
    CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=/usr/local/cargo/trunk-target \
    cargo install trunk --locked --version 0.21.14

COPY Cargo.toml Cargo.lock ./
COPY crates/wuwa-tracker-app/Cargo.toml crates/wuwa-tracker-app/Cargo.toml
COPY crates/wuwa-tracker-core/Cargo.toml crates/wuwa-tracker-core/Cargo.toml
COPY crates/wuwa-tracker-types/Cargo.toml crates/wuwa-tracker-types/Cargo.toml
COPY crates/wuwa-tracker-webui/Cargo.toml crates/wuwa-tracker-webui/Cargo.toml

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo fetch --locked && \
    RUSTUP_TOOLCHAIN=1.96.0 cargo fetch --locked --target wasm32-unknown-unknown

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/root/.cache/trunk \
    --mount=type=cache,target=/wuwa-tracker/target \
    make build-webui && \
    cargo build --release -p wuwa-tracker --no-default-features --bin wuwa-tracker-cli && \
    mkdir -p /out && \
    cp target/release/wuwa-tracker-cli /out/wuwa-tracker

FROM alpine AS deploy

WORKDIR /usr/local/bin

COPY --from=build /out/wuwa-tracker wuwa-tracker

ENTRYPOINT ["wuwa-tracker"]
CMD ["serve", "--host", "0.0.0.0"]
