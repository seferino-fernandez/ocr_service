
IMAGE_NAME := "ocr-service"

default:
    @just --list

# The correctness gate. Mirrors .github/workflows/pull-request-validation.yml,
# so a green `just verify` locally means a green CI. Non-zero on any failure.
verify:
    cargo fmt --all --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo check --locked
    cargo nextest run --all-targets --all-features
    cargo test --doc

# Run all tests: full feature matrix, default features, and doctests
test:
	cargo nextest run --all-targets --all-features
	cargo nextest run
	cargo test --doc

# Lint the code with Clippy
lint:
    cargo clippy

# Format code with rustfmt and apply Clippy's auto-fixable suggestions
format:
    cargo fmt --all
    cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged

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
