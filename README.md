# OCR Service

This repository contains OCR Service built using [Axum](https://github.com/tokio-rs/axum).

The full list of crates used can be found in the [Cargo.toml](./Cargo.toml) file. However, here are some key ones:

- [axum](https://github.com/tokio-rs/axum) - A user-friendly, modular web framework built with Tokio, Tower, and Hyper.
- [tesseract-rs](https://github.com/cafercangundogdu/tesseract-rs) - A library for OCR using Tesseract Rust bindings.
- [image](https://github.com/image-rs/image) - An image processing library for Rust.
- [Insta](https://insta.rs/) - A library for snapshot testing in Rust.
- [utoipa](https://github.com/juhaku/utoipa) - A library for generating OpenAPI documentation in Rust.
- [opentelemetry-rust](https://github.com/open-telemetry/opentelemetry-rust) - OpenTelemetry for Rust.

## Architecture

The domain vocabulary is defined in [CONTEXT.md](./CONTEXT.md); architectural decisions are
recorded in [docs/adr](./docs/adr).

Two modules carry the service's behaviour:

- **[`src/tessdata`](./src/tessdata)** — the Tessdata Catalog. Discovers the `.traineddata`
  files under `TESSDATA_PATH`, resolves a request's `language`/`model` to exactly one
  Tesseract Model, and serves the sorted listing.
- **[`src/ocr`](./src/ocr)** — the OCR Engine. Owns accepted content types, image decoding
  and the Tesseract FFI, and runs recognition on the blocking pool.

Both are built once at startup into `AppState::from_config`, leaving `router()` as pure
assembly. Handlers extract and delegate.

## Getting Started

### Configure the Application

#### Environment Variables

You can use the [.env.example](./.env.example) file or [src/config/app_config.rs](./src/config/app_config.rs) to view and configure the application.

The `TESSDATA_PATH` environment variable allows you to specify where the Tesseract data files are stored. By default, it's set to `tesseract`. If you change this path, make sure to:

1. Update the environment variable in your `.env` file
2. If using Docker or Docker Compose, pass the same path as a build argument to ensure your data gets copied into the container correctly

#### Download Tesseract Data Files

The OCR Service requires the Tesseract data files to be downloaded into the [tesseract](./tesseract) directory. You can download the files manually via these repositories:

- [tesseract-ocr/tessdata](https://github.com/tesseract-ocr/tessdata)
- [tesseract-ocr/tessdata_fast](https://github.com/tesseract-ocr/tessdata_fast)

Or by running the [scripts/download-tessdata.sh](./scripts/README.md) script.

### Starting the Application

With everything else set up, all you need to do now is:

```shell
cargo run
```

### OCR Engine Build Modes

By default, OCR Service compiles the bundled Tesseract 5.5.2 and Leptonica sources. This
requires no system Tesseract installation and remains the path used by `cargo build` and
CI.

For faster local builds, opt into a system-installed Tesseract instead:

```shell
# macOS
brew install tesseract

# Debian or Ubuntu
sudo apt install libtesseract-dev libleptonica-dev

cargo run --no-default-features --features system-tesseract
```

The system path links the Tesseract version provided by your package manager, which may
differ from the bundled 5.5.2. Recognition output can therefore vary slightly between
the two paths.

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
# If using the default TESSDATA_PATH (tesseract)
docker build -t ocr-service .
docker run -p 8080:8080 ocr-service

# If using a custom TESSDATA_PATH, specify it as both a build argument and environment variable
docker build --build-arg TESSDATA_PATH=your/custom/path -t ocr-service .
docker run -p 8080:8080 -e TESSDATA_PATH=your/custom/path ocr-service

# To link the image against the distro's Tesseract instead of the bundled 5.5.2 sources
docker build --build-arg TESSERACT_FEATURE=system-tesseract -t ocr-service .
```

If using Docker Compose, you can specify the custom path in the compose file:

```yaml
services:
    ocr_service:
        build:
            context: ../
            args:
                - TESSDATA_PATH=your/custom/path
                - TESSERACT_FEATURE=${TESSERACT_FEATURE:-bundled-tesseract}
            dockerfile: Dockerfile
```

Make sure the `TESSDATA_PATH` build argument matches the environment variable you've set in your `.env` file.
Set `TESSERACT_FEATURE=system-tesseract` before starting Compose to use the distro library.

## API Documentation

The API documentation is available at [http://localhost:8080/api-docs](http://localhost:8080/api-docs) when running locally.

### curl Examples

**Send a file to the `/api/v1/images` endpoint to process the image.**

```bash
curl -X POST -F "image=@./tests/images/tessdoc-introduction.png" \
"http://localhost:8080/api/v1/images?language=eng"
```

**Send a file to the `/api/v1/images` endpoint to process the image with a specific model.**

```bash
curl -X POST -F "image=@./tests/images/chinese-simplified-sign.jpg" \
"http://localhost:8080/api/v1/images?language=chi_sim&model=chi_sim"
```

Recognition can be tuned per request with optional query parameters:

- `psm`: Tesseract page segmentation mode (`0` through `13`).
- `oem`: Tesseract OCR Engine mode (`0` through `3`).
- `dpi`: positive source resolution in pixels per inch.
- `whitelist`: characters to restrict Recognition to.
- `blacklist`: characters to exclude from Recognition.
- `format`: output representation (`text`, `hocr`, `alto`, `tsv`, or `box`; defaults to
  `text`).

Omitting the five tuning parameters preserves Tesseract's default Recognition behavior;
`format` changes only the output representation. For example, this request treats the
image as a single text line at 300 DPI and recognizes digits only:

```bash
curl -X POST -F "image=@./tests/images/tessdoc-introduction.png" \
"http://localhost:8080/api/v1/images?language=eng&psm=7&dpi=300&whitelist=0123456789"
```

The default `text` representation is JSON and includes Tesseract's mean word confidence:

```json
{
  "text": "Recognized text...",
  "confidence": 91
}
```

The document formats are returned directly rather than JSON-escaped. For example, this
writes native hOCR to a file:

```bash
curl -X POST -F "image=@./tests/images/tessdoc-introduction.png" \
"http://localhost:8080/api/v1/images?language=eng&format=hocr" --output page.hocr
```

**Get all available languages and models.**

```bash
curl http://localhost:8080/api/v1/languages
```

```json
{
  "languages": [
    {
      "language": "chi_sim",
      "model": "chi_sim",
      "description": null,
      "is_default": false
    },
    {
      "language": "chi_sim",
      "model": "chi_sim_vert",
      "description": null,
      "is_default": false
    },
    {
      "language": "eng",
      "model": null,
      "description": null,
      "is_default": false
    }
  ]
}
```

The bundled `tesseract/` tree has no `models.toml`, so every bundled entry has a `null`
description and `is_default: false`. A deployment can annotate and select a default Model
with a sidecar, in which case an entry can instead look like:

```json
{
  "language": "sme",
  "model": "sme_bible",
  "description": "Trained on the 1895 North Sami bible",
  "is_default": true
}
```

A `model` of `null` means the language has a single trained model, so requests need only
the `language` parameter. Where a language has several models, pass both — see
[ADR 0001](./docs/adr/0001-languages-response-omits-filesystem-paths.md) for why the
response carries no filesystem paths.

#### Model metadata

For a Language with several named Models, add an optional `models.toml` to that Language's
directory under `TESSDATA_PATH`. It can describe Models for API consumers and nominate the
one used when an image request gives the Language but omits `model`:

```toml
default = "sme_bible"

[models.sme_bible]
description = "Trained on the 1895 North Sami bible"

[models.sme_dict]
description = "Trained on dictionary scans, 400dpi"
```

The `default` must name a `.traineddata` file in the same Language directory, otherwise
the service refuses to start. Without `models.toml`, Model resolution keeps its existing
behaviour and `description` is `null` while `is_default` is `false`.

### Health Check

```bash
curl http://localhost:8080/system/health
```

### Client Configuration

Get the upload limit and OCR Engine content types before selecting a file, so a browser can
reject a file that is too large before beginning an upload.

```bash
curl http://localhost:8080/system/config
```
