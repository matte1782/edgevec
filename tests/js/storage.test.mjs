// Tests for the IndexedDB shim behind EdgeVec.save()/load() — issue #12.
// Run: npm run test:storage
import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { IDBFactory } from "fake-indexeddb";
import { IndexedDbBackend } from "../../src/js/storage.js";

const TIMEOUT_MS = 2000;

function settles(promise) {
    let timer;
    return Promise.race([
        promise.then((value) => ({ value }), (error) => ({ error })),
        new Promise((_, reject) => {
            timer = setTimeout(() => reject(new Error("promise never settled")), TIMEOUT_MS);
        }),
    ]).finally(() => clearTimeout(timer));
}

function rawOpen(version) {
    return new Promise((resolve, reject) => {
        const req = version === undefined
            ? indexedDB.open("EdgeVecDB")
            : indexedDB.open("EdgeVecDB", version);
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error);
    });
}

// Recreates what EdgeVec <= 0.9.0 read() left behind: a v1 database without
// the "files" object store.
async function createBrokenDb() {
    (await rawOpen()).close();
}

function deleteDb() {
    return new Promise((resolve, reject) => {
        const req = indexedDB.deleteDatabase("EdgeVecDB");
        req.onsuccess = () => resolve("deleted");
        req.onerror = () => reject(req.error);
        req.onblocked = () => resolve("blocked");
    });
}

beforeEach(() => {
    globalThis.indexedDB = new IDBFactory();
});

test("write then read returns the same bytes", async () => {
    const data = Uint8Array.from([69, 86, 87, 66, 1, 2, 3]);
    await IndexedDbBackend.write("db", data);
    const read = await IndexedDbBackend.read("db");
    assert.deepEqual(new Uint8Array(read), data);
});

test("write overwrites an existing entry", async () => {
    await IndexedDbBackend.write("db", Uint8Array.from([1]));
    await IndexedDbBackend.write("db", Uint8Array.from([2, 2]));
    assert.deepEqual(new Uint8Array(await IndexedDbBackend.read("db")), Uint8Array.from([2, 2]));
});

test("read of a missing name rejects with NotFoundError on a fresh browser", async () => {
    const { error } = await settles(IndexedDbBackend.read("missing"));
    assert.equal(error?.name, "NotFoundError");
    assert.match(error.message, /missing/);
});

test("read of a missing name does not leave the database unusable", async () => {
    await settles(IndexedDbBackend.read("missing"));
    await IndexedDbBackend.write("db", Uint8Array.from([7]));
    assert.deepEqual(new Uint8Array(await IndexedDbBackend.read("db")), Uint8Array.from([7]));
});

test("repairs a database broken by EdgeVec <= 0.9.0", async () => {
    await createBrokenDb();
    const missing = await settles(IndexedDbBackend.read("anything"));
    assert.equal(missing.error?.name, "NotFoundError");

    await IndexedDbBackend.write("db", Uint8Array.from([9]));
    assert.deepEqual(new Uint8Array(await IndexedDbBackend.read("db")), Uint8Array.from([9]));
});

test("healthy v1 database written by EdgeVec <= 0.9.0 is read without an upgrade", async () => {
    // Old write() created the store at v1. Keep a connection open, as an old
    // tab would: a version upgrade would now be blocked, so none may happen.
    const setup = indexedDB.open("EdgeVecDB", 1);
    setup.onupgradeneeded = () => setup.result.createObjectStore("files");
    const oldTab = await new Promise((resolve) => { setup.onsuccess = () => resolve(setup.result); });
    await new Promise((resolve) => {
        const tx = oldTab.transaction(["files"], "readwrite");
        tx.objectStore("files").put(Uint8Array.from([4, 2]), "legacy");
        tx.oncomplete = resolve;
    });

    const { value, error } = await settles(IndexedDbBackend.read("legacy"));
    assert.equal(error, undefined);
    assert.deepEqual(new Uint8Array(value), Uint8Array.from([4, 2]));
    oldTab.close();
});

test("repair blocked by another open connection rejects instead of hanging", async () => {
    const oldTab = await rawOpen(); // broken v1 db, connection left open
    try {
        const { error } = await settles(IndexedDbBackend.write("db", Uint8Array.from([1])));
        assert.match(error?.message ?? "", /blocked/);
        const read = await settles(IndexedDbBackend.read("db"));
        assert.equal(read.error, error, "later reads must fail promptly with the same error");
        const write = await settles(IndexedDbBackend.write("db", Uint8Array.from([2])));
        assert.equal(write.error, error, "later writes must fail promptly too");
    } finally {
        oldTab.close();
        // Wait for the uncancellable upgrade's success and module-state cleanup.
        (await rawOpen()).close();
    }
    await IndexedDbBackend.write("db", Uint8Array.from([3]));
    assert.deepEqual(new Uint8Array(await IndexedDbBackend.read("db")), Uint8Array.from([3]));
});

test("connections are closed after every operation", async () => {
    await IndexedDbBackend.write("db", Uint8Array.from([1]));
    await IndexedDbBackend.read("db");
    await settles(IndexedDbBackend.read("missing"));
    assert.equal(await deleteDb(), "deleted");
});

test("concurrent calls during a blocked repair all reject promptly", async () => {
    const oldTab = await rawOpen();
    try {
        const results = await Promise.all([
            IndexedDbBackend.write("a", Uint8Array.from([1])),
            IndexedDbBackend.write("b", Uint8Array.from([2])),
            IndexedDbBackend.read("c"),
        ].map(settles));
        for (const result of results) {
            assert.match(result.error?.message ?? "", /blocked/);
            assert.equal(result.error, results[0].error);
        }
    } finally {
        oldTab.close();
        (await rawOpen()).close();
    }
    await IndexedDbBackend.write("a", Uint8Array.from([3]));
    assert.deepEqual(new Uint8Array(await IndexedDbBackend.read("a")), Uint8Array.from([3]));
});

test("concurrent operations all settle", async () => {
    const ops = [];
    for (let i = 0; i < 10; i++) {
        ops.push(IndexedDbBackend.write(`db${i}`, Uint8Array.from([i])));
        ops.push(settles(IndexedDbBackend.read(`db${i}`)));
    }
    await settles(Promise.all(ops));
    for (let i = 0; i < 10; i++) {
        assert.deepEqual(new Uint8Array(await IndexedDbBackend.read(`db${i}`)), Uint8Array.from([i]));
    }
});

test("rejects clearly when IndexedDB is unavailable", async () => {
    delete globalThis.indexedDB;
    const { error } = await settles(IndexedDbBackend.write("db", Uint8Array.from([1])));
    assert.match(error?.message ?? "", /IndexedDB is not available/);
    const read = await settles(IndexedDbBackend.read("db"));
    assert.match(read.error?.message ?? "", /IndexedDB is not available/);
});
