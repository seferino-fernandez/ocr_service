# 2. Tesseract Model metadata uses an optional language sidecar

Date: 2026-08-08

## Status

Accepted

## Context

One Language can have several Tesseract Models. Before this decision, callers could see
only each Model's name through `GET /api/v1/languages`. The names are enough to select a
Model, but do not explain the material, script, or training purpose it represents.

The Tessdata Catalog already resolves a request without a `model` by choosing a Language's
only Model, then its unnamed Model. When a Language has several named Models, the request
otherwise fails because the Catalog has no authority to choose one.

Giellatekno at UiT maintain several North Sámi (`sme`) Models and need to describe those
variants and nominate the Model that is appropriate by default without changing the OCR
Engine or publishing filesystem paths.

## Decision

An optional `models.toml` may appear in a Language directory under `TESSDATA_PATH`, beside
that Language's named `.traineddata` files. It has a top-level `default` Model name and a
`models` table keyed by Model name:

```toml
default = "sme_bible"

[models.sme_bible]
description = "Trained on the 1895 North Sami bible"
```

At startup, the Tessdata Catalog reads the real sidecar files. Malformed TOML fails startup
with the sidecar path and parser error. A configured default must name a Model discovered in
the same Language directory; otherwise startup fails with the path and missing Model name.

The Catalog uses that default after its existing single-Model and unnamed-Model fallbacks.
`GET /api/v1/languages` exposes each Model's nullable `description` and boolean
`is_default`. The wire response continues to carry no filesystem detail, as required by
ADR 0001.

The sidecar is optional: without it, discovery and resolution retain their previous
behaviour, and the added response fields are `null` and `false`. It is per Language
directory, not per Model file, because Model names only have meaning within a Language and
one small file can state both the relationship among the Models and their common default.

## Consequences

Language maintainers can document purpose and make an intentional default selection without
changing server configuration or the OCR Engine. They must keep the configured default in
sync with the `.traineddata` files or the service will not start.

The languages response gains two fields. API consumers can distinguish an undocumented
Model from one whose description was supplied, and can present the default without guessing
from the filesystem. Deployments that do not add a sidecar retain the former resolution
rules.
