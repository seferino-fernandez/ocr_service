FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
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

FROM gcr.io/distroless/cc-debian13:nonroot AS runtime

WORKDIR /app

ARG TESSDATA_PATH=tesseract

COPY --from=builder /app/$TESSDATA_PATH $TESSDATA_PATH
COPY --from=builder /app/target/release/ocr_service ocr_service

EXPOSE 8080
ENTRYPOINT ["./ocr_service"]
