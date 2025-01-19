# Running OCR Service with OpenObserve

See [openobserve.ai - Docs - Docker](https://openobserve.ai/docs/quickstart/#__tabbed_1_3) for more information.

## Prerequisites

- Docker
- Docker Compose

## Configuring OpenObserve and the OCR Service

Add the following environment variables to the `openobserve` service in the `compose.yaml` file:

- `ZO_ROOT_USER_EMAIL`: The email address of the root user.
- `ZO_ROOT_USER_PASSWORD`: The password of the root user.

Add the following environment variables to the `ocr_service` service in the `compose.yaml` file:

- `OTEL_PROVIDER_AUTH_TOKEN`: The authentication token for the OpenTelemetry provider. It's a base64 encoded string of the email and password separated by a colon.

## Building the OCR Service

```bash
docker compose build
```

## Running the OCR Service

```bash
docker compose up -d
```

## Stopping the OCR Service

```bash
docker compose down
```

## OpenObserve UI

Open the OpenObserve UI at [http://localhost:5080](http://localhost:5080).
