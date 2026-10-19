// Release WASM fromBytes benchmark. Run:
// node tests/js/persistence.bench.mjs target/persistence-release
// --reuse reuses the generated dataset. For first-call timings in a fresh Node:
// node tests/js/persistence.bench.mjs <build-dir> --measure-only [--bq]
import assert from "node:assert/strict";
import { readFile, writeFile, stat } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { performance } from "node:perf_hooks";
import { pathToFileURL } from "node:url";
import { gzipSync } from "node:zlib";

const build = path.resolve(process.argv[2] ?? "target/persistence-release");
const ev = await import(pathToFileURL(path.join(build, "edgevec.js")).href);
ev.initSync({ module: await readFile(path.join(build, "edgevec_bg.wasm")) });
const count = 100_000;
const dimensions = 128;
const reuse = process.argv.includes("--reuse");
const cacheFile = path.join(build, "benchmark-100k.evwb");
if (process.argv.includes("--measure-only")) {
    const bq = process.argv.includes("--bq");
    const bytes = await readFile(bq ? path.join(build, "benchmark-100k-bq.evwb") : cacheFile);
    console.log(JSON.stringify({
        node: process.version, platform: `${os.platform()} ${os.release()} ${os.arch()}`,
        cpu: os.cpus()[0]?.model, count, dimensions, metric: "l2", seed: 42,
        ...measure(bytes, bq),
    }, null, 2));
    process.exit(0);
}
const config = new ev.EdgeVecConfig(dimensions); // default HNSW parameters, L2
const db = reuse ? ev.EdgeVec.fromBytes(await readFile(cacheFile)) : new ev.EdgeVec(config);
let state = 42;
const vector = new Float32Array(dimensions);
const started = performance.now();
for (let i = 0; !reuse && i < count; i++) {
    for (let j = 0; j < dimensions; j++) {
        state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
        vector[j] = state / 2 ** 31 - 1;
    }
    db.insert(vector);
    if ((i + 1) % 25_000 === 0) console.error(`Built ${i + 1}/${count} vectors`);
}
if (!reuse) await writeFile(cacheFile, db.toBytes());

function measure(bytes, bq) {
    const times = [];
    let firstLoadMs;
    for (let i = 0; i < 8; i++) {
        const start = performance.now();
        const restored = ev.EdgeVec.fromBytes(bytes);
        const elapsed = performance.now() - start;
        assert.equal(restored.liveCount(), count);
        assert.equal(restored.dimensions(), dimensions);
        assert.equal(restored.hasBQ(), bq);
        restored.free();
        if (i === 0) firstLoadMs = elapsed;
        else times.push(elapsed);
    }
    const ordered = [...times].sort((a, b) => a - b);
    return { bq, bufferBytes: bytes.length, firstLoadMs, samplesMs: times, medianMs: ordered[3], maxMs: ordered[6] };
}

const plain = measure(db.toBytes(), false);
db.enableBQ();
const bqBytes = db.toBytes();
const withBQ = measure(bqBytes, true);
await writeFile(path.join(build, "benchmark-100k-bq.evwb"), bqBytes);
const wasmBytes = (await stat(path.join(build, "edgevec_bg.wasm"))).size;
const jsBytes = (await stat(path.join(build, "edgevec.js"))).size;
const wasmGzipBytes = gzipSync(await readFile(path.join(build, "edgevec_bg.wasm"))).length;
const jsGzipBytes = gzipSync(await readFile(path.join(build, "edgevec.js"))).length;
console.log(JSON.stringify({
    node: process.version,
    platform: `${os.platform()} ${os.release()} ${os.arch()}`,
    cpu: os.cpus()[0]?.model,
    count, dimensions, metric: "l2", seed: 42,
    totalElapsedMs: performance.now() - started,
    wasmBytes, jsBytes, combinedBytes: wasmBytes + jsBytes,
    wasmGzipBytes, jsGzipBytes, combinedGzipBytes: wasmGzipBytes + jsGzipBytes,
    plain, withBQ,
}, null, 2));
db.free();
config.free();
