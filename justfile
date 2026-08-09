
IMAGE_NAME := "ocr-service"

default:
    @just --list

# The correctness gate. Mirrors .github/workflows/pull-request-validation.yml,
# so a green `just verify` locally means a green CI. Non-zero on any failure.
verify:
    cargo fmt --all --check
    cargo clippy --all-targets --no-default-features --features bundled-tesseract -- -D warnings
    cargo check --locked --no-default-features --features bundled-tesseract
    cargo nextest run --all-targets --no-default-features --features bundled-tesseract
    cargo test --doc --no-default-features --features bundled-tesseract

# Run all bundled-Tesseract tests and doctests.
test:
    cargo nextest run --all-targets --no-default-features --features bundled-tesseract
    cargo test --doc --no-default-features --features bundled-tesseract

# Lint the bundled-Tesseract build with Clippy.
lint:
    cargo clippy --all-targets --no-default-features --features bundled-tesseract -- -D warnings

# Deliberately exercise the system-installed Tesseract build. Requires its development
# packages and may expose recognition differences from the bundled Tesseract version.
verify-system-tesseract:
    cargo clippy --all-targets --no-default-features --features system-tesseract -- -D warnings
    cargo check --locked --no-default-features --features system-tesseract
    cargo nextest run --all-targets --no-default-features --features system-tesseract
    cargo test --doc --no-default-features --features system-tesseract

# Format code with rustfmt and apply Clippy's auto-fixable suggestions
format:
    cargo fmt --all
    cargo clippy --all-targets --no-default-features --features bundled-tesseract --fix --allow-dirty --allow-staged

# Build the project in debug mode
build:
    cargo build

# Build the project in release mode for production
release:
    cargo build --release

# Remove the target directory and all build artifacts
clean:
    cargo clean

# Build Docker image
docker-build:
    docker build -t {{IMAGE_NAME}} .

# Build and smoke-test the Debian runtime that dynamically links system Tesseract.
docker-smoke-system-tesseract:
    scripts/smoke-docker-image.sh system-tesseract

# Build and smoke-test the default distroless runtime with bundled Tesseract.
docker-smoke-bundled-tesseract:
    scripts/smoke-docker-image.sh bundled-tesseract

# Run Docker image
docker-run:
    docker run -e OTEL_SDK_DISABLED=true {{IMAGE_NAME}}

# Run project locally
run:
    OTEL_SDK_DISABLED=true cargo run

# Build with dependency metadata embedded, then scan the binary for known vulnerabilities
audit:
    cargo auditable build --release
    cargo audit bin target/release/ocr_service

# Generate and open the project's API documentation in a browser
docs:
    cargo doc --open

# Generate an HTML coverage report from nextest runs and open it in a browser
coverage:
    cargo llvm-cov nextest --open --html
