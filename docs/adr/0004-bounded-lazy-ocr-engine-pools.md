# 4. Retain initialized OCR Engines in bounded, lazy per-Model pools

Date: 2026-08-08

## Status

Accepted

## Context

Every Recognition constructed a `TesseractAPI` and called `init` or `init_2`. Initialization
re-read the selected Tesseract Model's `.traineddata` file, so repeated requests paid the
same disk and setup cost even though the process already had an initialized engine.

An initialized Tesseract handle is stateful and cannot be shared concurrently. Image setup
and text extraction form one mutable sequence. Page segmentation mode, source resolution,
`tessedit_char_whitelist`, and `tessedit_char_blacklist` also persist after Recognition. A
pool that returned handles without resetting them would leak one request's settings into a
later request. OCR Engine mode is more restrictive: `oem` is selected by `init_2` and cannot
be changed after initialization.

Native Recognition can also run pathologically long. The HTTP request timeout can answer a
client while its blocking task continues, but it does not stop that task or release the
native resources it owns.

## Decision

Keep a separate bounded pool for each Tesseract Model's relative path. Pools and engines are
created lazily. A semaphore limits checked-out engines to `TESSERACT_POOL_SIZE`; when the
bound is reached, another Recognition waits instead of constructing beyond it or failing.
The default is the smaller of host available parallelism and four. A value of zero bypasses
all pools and preserves per-Recognition construction as an operational escape hatch.

Each checkout owns its handle exclusively through an RAII lease. Returning the lease clears
the image and adaptive classifier, restores page segmentation mode to `PSM_AUTO`, explicitly
clears both character lists with empty values, and restores source resolution to zero. The
handle returns to the idle pool only if every cleanup step succeeds; otherwise it is
discarded and later demand lazily replaces it. Because the lease owns the semaphore permit,
failures and unwinding also run this cleanup before another Recognition can acquire the
capacity.

Requests with an explicit `oem` bypass pooling. Their handle is initialized with `init_2`
for that Recognition and then dropped. This avoids mixing init-time-incompatible engine
modes while keeping the per-Model pool bound unambiguous.

Wrap pool waiting and blocking-task completion in `TESSERACT_RECOGNITION_TIMEOUT`, which
defaults to ten seconds. On expiry the request fails with a Recognition timeout error.
`tesseract-rs` 0.4.0 exposes no cancellation or interrupt API for an in-progress native
Recognition, so this is containment rather than native execution cancellation: the blocking
task continues, retains its engine lease, and returns the engine only after the call really
finishes and cleanup succeeds.

## Consequences

Repeated default-mode Recognition avoids rereading Model data, and concurrent Recognition
against one Model never shares a native handle. Stateful tuning cannot cross request
boundaries, while explicit OCR Engine modes remain correct at the cost of initialization on
each such request.

Retained memory is approximately `pool size × Model size × Models actually used`. A Model
never requested creates no engines, but large best-quality Models such as `chi_sim` can use
tens of megabytes for every lazily-created engine. Operators must balance pool size against
both expected concurrency and Model size, or select zero when reuse is unsafe in a specific
deployment.

A timeout frees the client and async request path, but cannot free the native blocking
thread. A permanently stuck native call therefore permanently consumes one blocking thread
and one pool slot. The slot is intentionally not made available because doing so would
either exceed the configured bound or expose a still-running handle to another Recognition.
