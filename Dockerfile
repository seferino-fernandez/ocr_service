ARG TESSERACT_FEATURE=bundled-tesseract

FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
ARG TESSERACT_FEATURE
WORKDIR /app
# Install dependencies for Tesseract Engine
RUN apt update && apt install -y \
    lld \
    clang \
    pkg-config \
    libssl-dev \
    cmake \
    g++ \
    && if [ "$TESSERACT_FEATURE" = "system-tesseract" ]; then \
        apt install -y libtesseract-dev libleptonica-dev; \
    fi \
    && rm -rf /var/lib/apt/lists/*

FROM chef AS planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef AS builder
ARG TESSERACT_FEATURE
COPY --from=planner /app/recipe.json recipe.json
# Build project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json --no-default-features --features "$TESSERACT_FEATURE"
COPY . .
RUN cargo build --release --bin ocr_service --no-default-features --features "$TESSERACT_FEATURE"

FROM gcr.io/distroless/cc-debian13:nonroot AS runtime-bundled-tesseract

FROM debian:trixie-slim AS runtime-system-tesseract
RUN apt-get update \
    && apt-get install -y --no-install-recommends libtesseract5 \
    && rm -rf /var/lib/apt/lists/*

FROM runtime-${TESSERACT_FEATURE} AS runtime

WORKDIR /app

ARG TESSDATA_PATH=tesseract

COPY --from=builder /app/$TESSDATA_PATH $TESSDATA_PATH
COPY --from=builder /app/target/release/ocr_service ocr_service

USER 65532:65532

EXPOSE 8080
ENTRYPOINT ["./ocr_service"]
