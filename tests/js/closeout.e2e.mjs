// Exercise the emitted public SDK and actual WASM, never mocks.
import { test, beforeEach } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { IDBFactory } from 'fake-indexeddb';
import { EdgeVecClient } from '../../wasm/dist/index.js';
import { EdgeVecIndex, EdgeVec, EdgeVecConfig, VectorType, JsMetadataValue, JsIndexType } from '../../pkg/index.js';

beforeEach(() => { globalThis.indexedDB = new IDBFactory(); });
const vector = (n) => Float32Array.from({ length: 64 }, (_, j) => (n + j) / 100);

test('client initializes in Node and restores dimensions and live count', async () => {
  const client = await EdgeVecClient.create({ dimensions: 64 });
  assert.equal(client.length, 0);
  const first = client.insert(vector(1));
  client.insert(vector(2));
  assert.equal(client.length, 2);
  client.softDelete(first);
  assert.equal(client.length, 1);
  await client.save('client');
  for (const config of [undefined, { dimensions: 64 }]) {
    const loaded = await EdgeVecClient.load('client', config);
    assert.equal(loaded.dimensions, 64);
    assert.equal(loaded.length, 1);
    assert.equal(loaded.search(vector(2), 1)[0].id, 2);
    assert.equal(loaded.insert(vector(3)), 3);
    assert.equal(loaded.length, 2);
    loaded.compact();
    assert.equal(loaded.length, 2);
    assert.equal(loaded.deletedCount, 0);
  }
  await assert.rejects(EdgeVecClient.load('client', { dimensions: 128 }), /dimensions do not match/);
});

test('high-level wrapper restores dimensions, metadata, filters, and buffers', async () => {
  const empty = new EdgeVecIndex({ dimensions: 64 });
  await empty.save('empty');
  assert.equal(await (await EdgeVecIndex.load('empty')).count('kind = "keep"'), 0);
  assert.throws(() => new EdgeVecIndex({ dimensions: 64, quantized: true }), /not supported/);
  const index = new EdgeVecIndex({ dimensions: 64, m: 8 });
  const first = index.add(vector(1), { kind: 'keep' });
  index.add(vector(2), { kind: 'keep' });
  index.add(vector(3), { kind: 'other' });
  index.delete(first);
  await index.save('wrapper');
  for (const loaded of [await EdgeVecIndex.load('wrapper'), EdgeVecIndex.fromBytes(index.toBytes())]) {
    assert.equal(loaded.dimensions, 64);
    assert.equal(loaded.size, 2);
    assert.equal(await loaded.count('kind = "keep"'), 1);
    assert.equal(loaded.getMetadata(2).kind, 'keep');
    assert.equal((await loaded.search(vector(2), 1))[0].id, 2);
  }
});

test('wrapper sparse and hybrid methods return parsed public result types', () => {
  const index = new EdgeVecIndex({ dimensions: 64 });
  index.add(vector(1));
  index.initSparseStorage();
  assert.equal(index.hasSparseStorage(), true);
  const indices = new Uint32Array([1, 5]);
  const values = new Float32Array([1, 2]);
  const id = index.insertSparse(indices, values, 100);
  assert.equal(index.sparseCount(), 1);
  const sparse = index.searchSparse(indices, values, 100, 1);
  assert.equal(sparse[0].id, id);
  assert.equal(typeof sparse[0].score, 'number');
  const hybrid = index.hybridSearch(vector(1), indices, values, 100, { k: 2 });
  assert.ok(Array.isArray(hybrid));
  assert.ok(hybrid.length > 0);
  assert.equal(typeof hybrid[0].score, 'number');
  assert.equal(hybrid.length, 1);
  assert.equal(hybrid[0].id, 1);
  assert.ok(hybrid[0].dense_rank && hybrid[0].sparse_rank);
});

test('hybrid document pairing survives deletion, compaction, and new inserts', () => {
  const config = new EdgeVecConfig(64);
  const db = new EdgeVec(config);
  const indices = new Uint32Array([0]);
  const values = n => new Float32Array([n]);
  const query = () => JSON.parse(db.hybridSearch(vector(2), indices, values(1), 10,
    JSON.stringify({ k: 2, dense_k: 2, sparse_k: 2 })));
  try {
    db.insert(vector(1)); db.insert(vector(2));
    assert.equal(db.insertSparse(indices, values(10), 10), 0);
    assert.equal(db.insertSparse(indices, values(1), 10), 1);
    assert.equal(query().length, 2);
    assert.ok(query().every(hit => hit.dense_rank && hit.sparse_rank));
    for (const [alpha, expected] of [[1, 2], [0, 1]]) {
      const hits = JSON.parse(db.hybridSearch(vector(2), indices, values(1), 10,
        JSON.stringify({ k: 1, dense_k: 2, sparse_k: 2, fusion: { type: 'linear', alpha } })));
      assert.equal(hits[0].id, expected);
    }
    db.softDelete(1);
    const live = JSON.parse(db.hybridSearch(vector(2), indices, values(1), 10,
      JSON.stringify({ k: 1, dense_k: 1, sparse_k: 1 })));
    assert.equal(live.length, 1);
    assert.equal(live[0].id, 2);
    assert.equal(live[0].sparse_score, 1);
    assert.ok(live[0].dense_rank && live[0].sparse_rank);
    db.compact().free();
    assert.equal(query().length, 1);
    assert.equal(query()[0].id, 1);
    assert.equal(query()[0].sparse_score, 1);
    const id = db.insert(vector(3));
    db.insertSparseForVector(id, indices, values(2), 10);
    assert.ok(query().every(hit => hit.dense_rank && hit.sparse_rank));
    assert.equal(query().length, 2);
    assert.throws(() => db.insertSparseForVector(id, indices, values(3), 10), /already has/);
  } finally { db.free(); config.free(); }
});

test('hybrid pairing supports sparse-first and explicit out-of-order insertion', () => {
  const indices = new Uint32Array([0]);
  const values = new Float32Array([1]);
  for (const sparseFirst of [true, false]) {
    const config = new EdgeVecConfig(64);
    const db = new EdgeVec(config);
    try {
      if (sparseFirst) {
        db.insertSparse(indices, values, 10); db.insertSparse(indices, values, 10);
        db.insert(vector(1)); db.insert(vector(2));
      } else {
        db.insert(vector(1)); db.insert(vector(2));
        db.insertSparseForVector(2, indices, values, 10);
        db.insertSparse(indices, values, 10); // picks unpaired ID1
        db.insertSparse(indices, values, 10); // reserves future ID3, never duplicates ID2
        assert.throws(() => db.compact(), /pending sparse pairings/);
      }
      const hits = JSON.parse(db.hybridSearch(vector(1), indices, values, 10, '{"k":10}'));
      assert.equal(hits.length, 2);
      assert.deepEqual(hits.map(hit => hit.id).sort(), [1, 2]);
      assert.ok(hits.every(hit => hit.dense_rank && hit.sparse_rank));
      const restored = EdgeVec.fromBytes(db.toBytes());
      try { assert.equal(restored.hasSparseStorage(), false); } finally { restored.free(); }
    } finally { db.free(); config.free(); }
  }
});

test('sparse overflow rejects instead of producing invalid JSON scores', () => {
  const config = new EdgeVecConfig(64);
  const db = new EdgeVec(config);
  const indices = new Uint32Array([0]);
  const values = new Float32Array([3e38]);
  try {
    db.insert(vector(1));
    db.insertSparse(indices, values, 10);
    assert.throws(() => db.searchSparse(indices, values, 10, 1), /non-finite/);
    assert.throws(() => db.hybridSearch(vector(1), indices, values, 10, '{"k":1}'), /non-finite/);
  } finally { db.free(); config.free(); }
});

for (const kind of ['bq', 'binary', 'flat']) {
  test(`${kind} stream owns its snapshot through source mutation and free`, () => {
    const config = new EdgeVecConfig(64);
    if (kind === 'binary') { config.metric = 'hamming'; config.vector_type = VectorType.Binary; }
    if (kind === 'flat') config.indexType = JsIndexType.Flat;
    const db = new EdgeVec(config);
    const iterators = [];
    let freed = false;
    try {
      for (let n = 1; n <= 2; n++) {
        const id = kind === 'bq' ? db.insert(vector(n)) : db.insertBinary(new Uint8Array(8).fill(n));
        const label = JsMetadataValue.fromString(`stream-${id}`);
        try { db.setMetadata(id, 'label', label); } finally { label.free(); }
      }
      if (kind === 'bq') db.enableBQ();
      const expected = db.toBytes();
      for (const size of [0, 1, 63, 64, 77, 4096]) iterators.push({ size, iterator: db.save_stream(size) });
      if (kind === 'flat') db.insertBinary(new Uint8Array(8).fill(3));
      else { db.softDelete(1); db.compact().free(); }
      db.free(); freed = true;
      for (const { size, iterator } of iterators) {
        const chunks = [];
        for (let chunk; (chunk = iterator.next_chunk()) !== undefined;) {
          assert.ok(chunk.length <= Math.max(64, size));
          chunks.push(chunk);
        }
        assert.equal(iterator.next_chunk(), undefined);
        const bytes = Uint8Array.from(chunks.flatMap(chunk => Array.from(chunk)));
        assert.deepEqual(bytes, expected);
        const restored = EdgeVec.fromBytes(bytes);
        try {
          assert.equal(restored.liveCount(), 2);
          assert.equal(restored.getAllMetadata(2).label, 'stream-2');
          if (kind === 'bq') assert.equal(restored.hasBQ(), true);
          else assert.equal(restored.searchBinary(new Uint8Array(8).fill(2), 1)[0].id, 2);
        } finally { restored.free(); }
      }
    } finally {
      if (!freed) db.free();
      config.free();
      for (const { iterator } of iterators) iterator.free();
    }
  });
}

for (const binary of [false, true]) {
  for (const deleted of [0, 1, 3]) {
    test(`${binary ? 'binary' : 'BQ'} compaction preserves storage, metadata, and buffer state (${deleted} deleted)`, () => {
      const config = new EdgeVecConfig(64);
      if (binary) { config.metric = 'hamming'; config.vector_type = VectorType.Binary; }
      const db = new EdgeVec(config);
      try {
        for (const [i, byte] of [0, 0x55, 0xff].entries()) {
          const id = binary ? db.insertBinary(new Uint8Array(8).fill(byte)) : db.insert(vector(i + 1));
          const label = JsMetadataValue.fromString(`original-${id}`);
          try { db.setMetadata(id, 'label', label); } finally { label.free(); }
        }
        if (!binary) db.enableBQ();
        for (let id = 1; id <= deleted; id++) db.softDelete(id);
        const result = db.compact();
        assert.equal(result.new_size, 3 - deleted);
        assert.equal(result.tombstones_removed, deleted);
        result.free();
        const restored = EdgeVec.fromBytes(db.toBytes());
        try {
          for (const index of [db, restored]) {
            assert.equal(index.liveCount(), 3 - deleted);
            assert.equal(index.deletedCount(), 0);
            if (!binary) assert.equal(index.hasBQ(), true);
            for (let id = 1; id <= 3 - deleted; id++) {
              assert.equal(index.getAllMetadata(id).label, `original-${id + deleted}`);
            }
            if (deleted < 3) {
              const hits = binary ? index.searchBinary(new Uint8Array(8).fill(0xff), 1) : index.search(vector(3), 1);
              assert.equal(hits[0].id, 3 - deleted);
            }
            const id = binary ? index.insertBinary(new Uint8Array(8).fill(0xaa)) : index.insert(vector(4));
            assert.equal(id, 4 - deleted);
          }
        } finally { restored.free(); }
      } finally { db.free(); config.free(); }
    });
  }
}

test('CommonJS entry point initializes WASM and exposes the builder', async () => {
  const require = createRequire(import.meta.url);
  const { EdgeVecClient: Client, EdgeVecConfigBuilder } = require('../../wasm/index.cjs');
  const client = await Client.create(new EdgeVecConfigBuilder(64).build());
  const id = client.insert(vector(4));
  assert.equal(client.search(vector(4), 1)[0].id, id);
});
