# Mobile Browser Usage Guide

EdgeVec is fully compatible with mobile browsers. This guide covers
platform-specific considerations for iOS Safari and Android Chrome.

## Browser Support

| Platform | Browser | Minimum Version | Status |
|:---------|:--------|:----------------|:-------|
| iOS | Safari | 15+ | Supported |
| Android | Chrome | 90+ | Supported |
| Android | Firefox | 100+ | Supported |

## Memory Considerations

### iOS Safari
- Maximum WASM memory: ~1GB
- Recommended max vectors: 500,000 (128-dim)
- Configure via `EdgeVecConfig` for explicit control

### Android Chrome
- Maximum WASM memory: varies by device
- Low-end devices: 100,000 vectors max
- High-end devices: 500,000+ vectors

## Performance Tips

1. **Lazy Loading**: Load EdgeVec only when needed
2. **Web Workers**: Use for background indexing
3. **Batch Operations**: Insert vectors in batches
4. **Quantization**: Use SQ8 for 4x memory reduction

## Example: Safe Mobile Initialization

```javascript
async function initEdgeVec() {
  try {
    // Dynamic import for code splitting
    const { default: init, EdgeVec, EdgeVecConfig } = await import('edgevec');
    await init();

    // Conservative config for mobile
    const config = new EdgeVecConfig(128);
    config.ef_construction = 100;
    config.m = 16;

    return new EdgeVec(config);
  } catch (e) {
    console.error('EdgeVec initialization failed:', e);
    // Fallback to server-side search
    return null;
  }
}
```

## Troubleshooting

### "Out of memory" errors
- Reduce the number of vectors stored in the index
- Use quantization (SQ8) for 4x memory reduction
- Implement pagination for large datasets

### Slow initial load
- WASM JIT compilation takes time on first load
- Use `wasm-opt` for smaller bundle size
- Consider preloading WASM in service worker

### Search hangs on iOS
- Background tabs may suspend WASM
- Use `requestIdleCallback` for non-urgent operations
- Handle visibility changes with Page Visibility API

## Testing Your Integration

Use the test page at `tests/mobile/index.html` to verify EdgeVec works
on your target mobile browsers. The test page includes:

1. WASM initialization test
2. Index creation test
3. Vector insert/search tests
4. Full metadata API tests (string, integer, float, boolean, array)
5. Memory stress test (1000 vectors)
6. Performance timing

### Running the Test Page

1. Build the WASM package: `wasm-pack build --target web`
2. Copy `pkg/edgevec.js` and `pkg/edgevec_bg.wasm` to `tests/mobile/`
3. Serve the directory: `python -m http.server 8080`
4. Open `http://localhost:8080/tests/mobile/` on your mobile device

## Known Limitations

### iOS Safari
- WASM memory limited to ~1GB (vs 4GB on desktop)
- No SharedArrayBuffer without cross-origin isolation
- JIT compilation may be slower initially
- Background tabs may have WASM suspended

### Android Chrome
- Memory varies by device (typically 512MB-2GB for WASM)
- Older devices may have slower WASM execution
- Battery saver mode may throttle WASM

## Cross-Origin Isolation

For advanced features requiring `SharedArrayBuffer`:

```html
<!-- Required headers for cross-origin isolation -->
Cross-Origin-Embedder-Policy: require-corp
Cross-Origin-Opener-Policy: same-origin
```

Note: Most EdgeVec features work without cross-origin isolation.
Only parallel WASM execution requires these headers.
