FROM lukemathwalker/cargo-chef:0.1.70-rust-bookworm AS chef
WORKDIR /app
# Install dependencies for Tesseract Engine
RUN apt update && apt install -y \
    lld \
    clang \
    pkg-config \
    libssl-dev \
    cmake \
    g++

FROM chef AS planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin ocr_service

FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update -y \
    && apt-get install -y --no-install-recommends ca-certificates \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/tesseract /app/tesseract
COPY --from=builder /app/target/release/ocr_service ocr_service

EXPOSE 8080
ENTRYPOINT ["./ocr_service"]
