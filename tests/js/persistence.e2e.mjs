// Exercises the actual wasm-bindgen glue, not just the standalone IDB shim.
// Build a web package, then run: node tests/js/persistence.e2e.mjs <build-dir>
import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { IDBFactory } from "fake-indexeddb";

const build = path.resolve(process.argv[2] ?? "target/persistence-e2e");
const ev = await import(pathToFileURL(path.join(build, "edgevec.js")).href);
ev.initSync({ module: await readFile(path.join(build, "edgevec_bg.wasm")) });
const ids = (hits) => hits.map((hit) => hit.id);
const vector = (i) => Float32Array.from({ length: 64 }, (_, j) => ((i * 31 + j * 17) % 97) / 97 - 0.5);

function rawOpen() {
    return new Promise((resolve, reject) => {
        const req = indexedDB.open("EdgeVecDB");
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error);
    });
}

beforeEach(() => { globalThis.indexedDB = new IDBFactory(); });

test("issue #12: empty 768-d cosine index saves and loads", { timeout: 5000 }, async () => {
    const config = new ev.EdgeVecConfig(768);
    config.metric = "cosine";
    const db = new ev.EdgeVec(config);
    try {
        await db.save("test");
        const loaded = await ev.EdgeVec.load("test");
        assert.equal(loaded.dimensions(), 768);
        assert.equal(loaded.liveCount(), 0);
        loaded.free();
        await assert.rejects(ev.EdgeVec.load("missing"), { name: "NotFoundError" });
    } finally { db.free(); config.free(); }
});

test("blocked repair rejects repeated WASM calls and recovers", { timeout: 5000 }, async () => {
    const blocker = await rawOpen(); // raw v1 without the files store
    const config = new ev.EdgeVecConfig(64);
    const db = new ev.EdgeVec(config);
    try {
        await Promise.all([
            assert.rejects(db.save("repair"), /blocked/),
            assert.rejects(ev.EdgeVec.load("repair"), /blocked/),
            assert.rejects(db.save("other"), /blocked/),
        ]);
    } finally {
        blocker.close();
        (await rawOpen()).close(); // wait for the pending upgrade to complete
    }
    try {
        db.insert(vector(1));
        await db.save("repair");
        const loaded = await ev.EdgeVec.load("repair");
        assert.equal(loaded.liveCount(), 1);
        loaded.free();
    } finally { db.free(); config.free(); }
});

test("WASM IndexedDB and buffer roundtrips preserve HNSW state", { timeout: 10000 }, async () => {
    const config = new ev.EdgeVecConfig(64);
    config.metric = "cosine";
    const db = new ev.EdgeVec(config);
    try {
        for (let i = 0; i < 100; i++) {
            db.insertWithMetadata(vector(i), { title: `doc-${i}`, n: i, tags: ["a", "b"], ok: true, score: 0.25 });
        }
        db.softDelete(5);
        db.enableBQ();
        await db.save("full");
        const loaded = await ev.EdgeVec.load("full");
        const fromBuffer = ev.EdgeVec.fromBytes(db.toBytes());
        try {
            for (const restored of [loaded, fromBuffer]) {
                assert.equal(restored.liveCount(), 99);
                assert.equal(restored.dimensions(), 64);
                assert.equal(restored.isDeleted(5), true);
                assert.equal(restored.hasBQ(), true);
                assert.deepEqual(restored.getAllMetadata(42), db.getAllMetadata(42));
                assert.deepEqual(ids(restored.search(vector(31), 10)), ids(db.search(vector(31), 10)));
                assert.deepEqual(ids(restored.searchBQ(vector(31), 10)), ids(db.searchBQ(vector(31), 10)));
                assert.equal(restored.insert(vector(101)), 101);
            }
        } finally { loaded.free(); fromBuffer.free(); }
    } finally { db.free(); config.free(); }
});

test("WASM IndexedDB roundtrip preserves binary Flat search", { timeout: 5000 }, async () => {
    const config = new ev.EdgeVecConfig(64);
    config.indexType = ev.JsIndexType.Flat;
    const db = new ev.EdgeVec(config);
    try {
        for (let i = 0; i < 20; i++) db.insertBinary(new Uint8Array(8).fill(i));
        await db.save("flat");
        const restored = await ev.EdgeVec.load("flat");
        assert.deepEqual(ids(restored.searchBinary(new Uint8Array(8).fill(7), 5)), ids(db.searchBinary(new Uint8Array(8).fill(7), 5)));
        restored.free();
    } finally { db.free(); config.free(); }
});

test("WASM save/load reject clearly without IndexedDB", { timeout: 5000 }, async () => {
    delete globalThis.indexedDB;
    const config = new ev.EdgeVecConfig(64);
    const db = new ev.EdgeVec(config);
    try {
        await assert.rejects(db.save("x"), /IndexedDB is not available/);
        await assert.rejects(ev.EdgeVec.load("x"), /IndexedDB is not available/);
    } finally { db.free(); config.free(); }
});
