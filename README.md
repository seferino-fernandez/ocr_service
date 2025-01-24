# OCR Service

This repository contains OCR Service built using [Axum](https://github.com/tokio-rs/axum).

The full list of crates used can be found in the [Cargo.toml](./Cargo.toml) file. However, here are some key ones:

* [axum](https://github.com/tokio-rs/axum) - A user-friendly, modular web framework built with Tokio, Tower, and Hyper.
* [tesseract-rs](https://github.com/cafercangundogdu/tesseract-rs) - A library for OCR using Tesseract Rust bindings.
* [image](https://github.com/image-rs/image) - An image processing library for Rust.
* [Insta](https://insta.rs/) - A library for snapshot testing in Rust.
* [utoipa](https://github.com/juhaku/utoipa) - A library for generating OpenAPI documentation in Rust.
* [opentelemetry-rust](https://github.com/open-telemetry/opentelemetry-rust) - OpenTelemetry for Rust.

## Getting Started

### Configure the Application

#### Environment Variables

You can use the [.env.example](./.env.example) file or [src/config/app_config.rs](./src/config/app_config.rs) to view and configure the application.

#### Download Tesseract Data Files

The OCR Service requires the Tesseract data files to be downloaded into the [tesseract](./tesseract) directory. You can download the files manually via these repositories:

* [tesseract-ocr/tessdata](https://github.com/tesseract-ocr/tessdata)
* [tesseract-ocr/tessdata_fast](https://github.com/tesseract-ocr/tessdata_fast)

Or by running the [scripts/download-tessdata.sh](./scripts/README.md) script.

### Starting the Application

With everything else set up, all you need to do now is:

```shell
cargo run
```

### Running Tests

Run tests:

```sh
cargo test
```

Run Snapshot tests:

```sh
cargo insta test
```

Run and review Snapshot tests:

```sh
cargo insta test --review
```

### Linting and Formatting

Run Clippy:

```sh
cargo clippy
```

Run Rustfmt:

```sh
cargo fmt
```

## Deployment

For building and running the docker image locally:

```sh
docker build -t ocr-service .
docker run -p 8080:8080 ocr-service
```

## API Documentation

The API documentation is available at [http://localhost:8080/api-docs](http://localhost:8080/api-docs) when running locally.

### curl Examples

**Send a file to the `/api/v1/images` endpoint to process the image.**

```bash
curl -X POST -F "image=@./tests/images/tessdoc-introduction.png" \
http://localhost:8080/api/v1/images
```

### Health Check

```bash
curl http://localhost:8080/health
```
