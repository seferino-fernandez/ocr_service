#!/usr/bin/env bash

# Build and run one Docker runtime variant, then prove HTTP readiness and Recognition.
set -Eeuo pipefail

if [[ $# -ne 1 ]]; then
    echo "usage: $0 <bundled-tesseract|system-tesseract>" >&2
    exit 2
fi

readonly tesseract_feature="$1"

case "$tesseract_feature" in
    bundled-tesseract|system-tesseract) ;;
    *)
        echo "unsupported Tesseract feature: $tesseract_feature" >&2
        exit 2
        ;;
esac

readonly image_name="ocr-service-smoke-${tesseract_feature}"
readonly container_name="ocr-service-smoke-${tesseract_feature}-$$"
readonly health_path="/system/health"
readonly recognition_path="/api/v1/images?language=eng"
readonly recognition_image="${SMOKE_RECOGNITION_IMAGE:-tests/images/tessdoc-introduction.png}"
readonly recognition_reference="${SMOKE_RECOGNITION_REFERENCE:-tests/api/snapshots/api__images__images_endpoint_tessdoc_introduction.snap}"
response_file="$(mktemp)"
buildx_config_dir="$(mktemp -d)"
container_started=false

cleanup() {
    local exit_status=$?

    if [[ "$container_started" == "true" ]]; then
        if [[ $exit_status -ne 0 ]]; then
            echo "Container logs for $container_name:" >&2
            docker logs "$container_name" >&2 || true
        fi
        docker rm --force "$container_name" >/dev/null || true
    fi
    rm -f "$response_file"
    rm -rf "$buildx_config_dir"
}

trap cleanup EXIT

if [[ ! -f "$recognition_image" ]]; then
    echo "Recognition image does not exist: $recognition_image" >&2
    exit 2
fi

if [[ ! -f "$recognition_reference" ]]; then
    echo "Recognition reference does not exist: $recognition_reference" >&2
    exit 2
fi

echo "Building $image_name with $tesseract_feature"
BUILDX_CONFIG="$buildx_config_dir" docker buildx build \
    --load \
    --build-arg "TESSERACT_FEATURE=$tesseract_feature" \
    --tag "$image_name" \
    .

echo "Starting $container_name"
docker run \
    --detach \
    --name "$container_name" \
    --publish 127.0.0.1::8080 \
    --env OTEL_SDK_DISABLED=true \
    "$image_name" >/dev/null
container_started=true

if [[ "$tesseract_feature" == "system-tesseract" ]]; then
    echo "Dynamic library resolution:"
    docker exec "$container_name" ldd /app/ocr_service | grep -E 'libtesseract|libleptonica'
    echo "Installed Tesseract and Leptonica packages:"
    docker exec "$container_name" dpkg-query -W -f='${binary:Package}\t${Version}\n' libtesseract5 libleptonica6
fi

readonly published_address="$(docker port "$container_name" 8080/tcp)"
readonly health_url="http://${published_address}${health_path}"
ready=false

for attempt in {1..30}; do
    if [[ "$(docker inspect --format '{{.State.Running}}' "$container_name")" != "true" ]]; then
        echo "Container exited before responding to $health_path" >&2
        exit 1
    fi

    if curl --fail --silent --show-error --max-time 2 --output "$response_file" "$health_url"; then
        ready=true
        break
    fi

    sleep 1
done

if [[ "$ready" != "true" ]]; then
    echo "Timed out waiting for $health_url" >&2
    exit 1
fi

echo "GET $health_path response:"
cat "$response_file"
echo

readonly recognition_url="http://${published_address}${recognition_path}"
echo "POST $recognition_path using $recognition_image"

if ! recognition_status="$(curl \
    --silent \
    --show-error \
    --max-time 30 \
    --output "$response_file" \
    --write-out '%{http_code}' \
    --request POST \
    --form "image=@${recognition_image};type=image/png" \
    "$recognition_url")"; then
    echo "POST $recognition_path failed before receiving an HTTP response" >&2
    exit 1
fi

if [[ "$recognition_status" != "200" ]]; then
    echo "POST $recognition_path returned HTTP $recognition_status (expected 200):" >&2
    cat "$response_file" >&2
    exit 1
fi

python3 scripts/compare-recognition.py "$recognition_reference" "$response_file"
