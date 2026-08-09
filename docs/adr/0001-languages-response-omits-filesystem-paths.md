# 1. The languages response omits filesystem paths

Date: 2026-08-08

## Status

Accepted

## Context

`GET /api/v1/languages` used to serialise the Tessdata Catalog's internal record straight
onto the wire. A single struct, `TesseractModel`, played three roles at once: the record
produced by walking the tessdata directory, the value passed to Tesseract's `init`, and the
API response body.

The consequence was visible in the committed snapshot:

```yaml
languages:
  - full_path: tesseract/chi_sim/chi_sim.traineddata
    language: chi_sim
    model: chi_sim
    relative_path: chi_sim/chi_sim
```

Two problems follow from that.

1. **It discloses server layout.** Every response told callers where the service keeps its
   files, including whatever `TESSDATA_PATH` is set to in that deployment.
2. **It froze the internal representation.** With no seam between disk and wire, any change
   to how models are stored — absolute paths, a different directory convention, file
   handles, memory-mapped data — was automatically a breaking API change. The fields were
   also the wrong shape internally: both were `Option<String>` that discovery always
   populated, so the OCR path carried an `unwrap_or_default()` that would silently hand
   Tesseract an empty language if the invariant ever broke.

## Decision

Split the type in two.

- `crate::tessdata::TesseractModel` stays internal to the Tessdata Catalog. It carries no
  serde derives, and its path fields are now `PathBuf` and `String` rather than `Option`s.
- `crate::models::languages::LanguageSummary` is the wire type, carrying `language` and
  `model` only.

`TessdataCatalog::list()` maps one onto the other and owns the sort order.

## Consequences

This is a **breaking change** to the `/api/v1/languages` response. `full_path` and
`relative_path` are gone:

```yaml
languages:
  - language: chi_sim
    model: chi_sim
  - language: chi_sim
    model: chi_sim_vert
  - language: eng
    model: ~
```

Anything relying on those fields must switch to `language` + `model`, which is all the
images endpoint has ever accepted anyway — the paths were never usable as request input.

In exchange, the Catalog's on-disk representation is now free to change without touching
the API contract, and the OpenAPI schema describes only fields callers can act on.

If a future need arises to expose which file backed a model — for a diagnostics endpoint,
say — add it to a dedicated response type rather than reinstating it here.
