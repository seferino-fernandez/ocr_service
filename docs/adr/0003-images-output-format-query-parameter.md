# 3. Images output format is selected by query parameter

Date: 2026-08-08

## Status

Accepted

## Context

`POST /api/v1/images` originally returned only a JSON object containing flat recognized
text. That representation is useful for reading the text, but document digitisation also
needs layout, word positions, and per-word confidence. Tesseract can produce hOCR, ALTO
XML, TSV, and training box output in addition to plain text.

HTTP content negotiation through `Accept` could select among those representations. In
this API, however, output format is part of the Recognition request alongside `language`,
`model`, and the Tesseract tuning parameters. Callers commonly construct these requests as
pipeline jobs and persist their URLs or query options. An explicit `format` value keeps
that selection visible in the request and gives invalid values a direct validation error
that lists the supported formats. It also preserves the existing behaviour for every
caller that omits the new parameter.

Wrapping document formats in JSON would give every success response the same outer media
type, but XML, HTML, TSV, and box consumers would then have to parse JSON and unescape a
string before passing the document to the next tool.

## Decision

`POST /api/v1/images` accepts a `format` query parameter with `text`, `hocr`, `alto`,
`tsv`, and `box` as its values. Omitting it is equivalent to `format=text`.

The `text` representation remains JSON and adds Tesseract's mean word confidence to the
recognized text. Each document representation is returned as the response body under its
native content type: `text/vnd.hocr+html`, `application/xml`,
`text/tab-separated-values`, or `text/plain`.

The query parameter is the sole format-selection mechanism. The endpoint does not vary
its representation based on the `Accept` header.

## Consequences

Existing clients continue to receive JSON without changing their requests, with the
additive `confidence` field. Digitisation and training pipelines can stream native output
straight into format-aware tools without a JSON decoding step.

Clients that request an unsupported format receive a `400` response naming `format` and
listing the accepted values. A client that wants a different representation must change
the query parameter rather than only its `Accept` header.
