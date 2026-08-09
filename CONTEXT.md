# Domain Model

The shared vocabulary for OCR Service. Use these terms in code, comments, commits and
issues; if a concept here needs a different name, change it here first.

## Language

A Tesseract language code, such as `eng`, `deu` or `chi_sim`. It is the top-level thing a
caller asks for via the `language` query parameter, and it defaults to
`SERVICE_DEFAULT_LANGUAGE` (`eng`) when omitted.

## Model

The discriminator used when a Language has more than one trained variant — `chi_sim_vert`,
`fast`, `best`. A Model only means something in the context of a Language, so the images
endpoint rejects a `model` given without a `language`.

Languages are laid out on disk as follows:

```
tesseract/
  eng.traineddata              -> Language "eng",     no Model
  chi_sim/
    chi_sim.traineddata        -> Language "chi_sim", Model "chi_sim"
    chi_sim_vert.traineddata   -> Language "chi_sim", Model "chi_sim_vert"
```

A Language whose file sits directly in the tessdata directory has no Model. When a
Language has several Models and the caller names none, the Model-less one wins; if every
variant is named, the caller has to choose.

## Tesseract Model

One `.traineddata` file: a Language, an optional Model, and the paths needed to load it.
This is the Tessdata Catalog's internal record (`src/tessdata/mod.rs`) and is deliberately
**not** what the API returns — see [ADR 0001](docs/adr/0001-languages-response-omits-filesystem-paths.md).

Its `relative_path` is the form Tesseract's `init` expects: the path below the tessdata
directory, without the `.traineddata` extension.

## Tessdata Catalog

The set of Tesseract Models discovered on disk, and the authority on which one answers a
given request (`src/tessdata/`). It owns three things:

- **discovery** — walking the configured `TESSDATA_PATH` for `.traineddata` files
- **resolution** — turning a request's `language` and `model` into exactly one Tesseract
  Model, including the default-language and Model-less fallbacks
- **listing** — the sorted view served by `GET /api/v1/languages`

Because the Catalog owns the default Language, callers never have to thread configuration
through a lookup. It is built once at startup and shared across requests.

## OCR Engine

The component that turns image bytes plus a Tesseract Model into text (`src/ocr/`).
Everything Tesseract-specific lives behind it: accepted content types, image decoding, the
RGB8 pixel layout the C API expects, and the FFI call sequence. It knows nothing about
HTTP — the mapping from `OcrError` onto status codes is an adapter in
`src/models/error.rs`.

## Recognition

A single OCR operation: one image, one Tesseract Model, one string of text out. Recognition
is blocking and CPU-bound, so it runs on the blocking pool rather than on an async worker
thread.
