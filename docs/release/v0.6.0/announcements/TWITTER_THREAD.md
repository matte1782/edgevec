# EdgeVec v0.6.0 Twitter/X Thread

## Tweet 1 (Main)
EdgeVec v0.6.0 is out! A browser-native vector database in Rust/WASM.

New in v0.6.0:
- 32x memory reduction with Binary Quantization
- Metadata filtering
- Memory pressure monitoring

npm install edgevec
cargo add edgevec

Thread /1

---

## Tweet 2 (Binary Quantization)
Binary Quantization compresses 768-dim vectors from 3KB to 96 bytes while maintaining 93%+ recall with rescoring.

Perfect for mobile/edge deployment where memory matters.

Demo: https://github.com/matte1782/edgevec

/2

---

## Tweet 3 (Filtering)
Filter your searches without post-processing:

searchFiltered(query, 10, { filter: "category = 'docs' AND year >= 2024" })

Expression syntax supports =, !=, >, <, AND, OR, IN, CONTAINS.

/3

---

## Tweet 4 (Demo)
Try the interactive demo with cyberpunk aesthetics!

Built with the WASM bindings - everything runs in your browser.

[Attach demo_hero.png screenshot]

/4

---

## Hashtags
#rustlang #wasm #vectordatabase #machinelearning #ai #webdev
