// Generates golden persistence fixtures from REAL EdgeVec builds.
//
// Usage (see README.md in this directory):
//   node generate.mjs <label> <path-to-wasm-pack-web-build> [--current]
//
// Old releases only expose IndexedDB save(), so bytes are captured from
// fake-indexeddb. With --current, toBytes() is used and BQ/Flat fixtures are
// added. Each run writes <label>_*.bin and merges into expected.json.
import { IDBFactory } from "fake-indexeddb";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const [label, pkgDir, flag] = process.argv.slice(2);
if (!label || !pkgDir) {
  console.error("usage: node generate.mjs <label> <pkg-dir> [--current]");
  process.exit(2);
}
const current = flag === "--current";
const here = path.dirname(fileURLToPath(import.meta.url));
const ev = await import(pathToFileURL(path.resolve(pkgDir, "edgevec.js")).href);
ev.initSync({ module: fs.readFileSync(path.resolve(pkgDir, "edgevec_bg.wasm")) });
globalThis.indexedDB = new IDBFactory();

let seed = 7;
const rnd = () => ((seed = (seed * 1103515245 + 12345) >>> 0) / 2 ** 32) * 2 - 1;
const DIMS = 16;
const N = 40;

async function captureBytes(db, name) {
  if (current) return db.toBytes();
  await db.save(name);
  return new Promise((resolve, reject) => {
    const req = indexedDB.open("EdgeVecDB");
    req.onerror = () => reject(req.error);
    req.onsuccess = () => {
      const conn = req.result;
      const get = conn.transaction(["files"], "readonly").objectStore("files").get(name);
      get.onsuccess = () => { conn.close(); resolve(new Uint8Array(get.result)); };
      get.onerror = () => reject(get.error);
    };
  });
}

const ids = (hits) => hits.map((h) => h.id);
const sorted = (o) => Object.fromEntries(Object.entries(o ?? {}).sort(([a], [b]) => a.localeCompare(b)));

async function hnswFixture(name, metric, withBq) {
  const config = new ev.EdgeVecConfig(DIMS);
  config.metric = metric;
  const db = new ev.EdgeVec(config);
  const vectors = [];
  for (let i = 0; i < N; i++) {
    const v = Float32Array.from({ length: DIMS }, rnd);
    vectors.push(v);
    if (i < 6) {
      db.insertWithMetadata(v, { title: `doc-${i}`, n: i, score: i / 4, ok: i % 2 === 0, tags: ["x", `t${i}`] });
    } else {
      db.insert(v);
    }
  }
  db.softDelete(5);
  db.softDelete(17);
  if (withBq) db.enableBQ();
  const queries = [0, 9, 33].map((i) => Array.from(vectors[i]));
  return {
    bytes: await captureBytes(db, name),
    expected: {
      kind: "hnsw",
      metric,
      dims: DIMS,
      liveCount: db.liveCount(),
      deleted: [5, 17],
      bq: withBq,
      queries,
      results: queries.map((q) => ids(db.search(Float32Array.from(q), 10))),
      metadata: Object.fromEntries([1, 2, 3, 6, 7].map((id) => [id, sorted(db.getAllMetadata(id))])),
    },
  };
}

async function flatFixture(name) {
  const config = new ev.EdgeVecConfig(64);
  config.indexType = ev.JsIndexType.Flat;
  const db = new ev.EdgeVec(config);
  const vectors = [];
  for (let i = 0; i < 20; i++) {
    const v = Uint8Array.from({ length: 8 }, () => (rnd() * 255) & 0xff);
    vectors.push(v);
    db.insertBinary(v);
  }
  const queries = [2, 11].map((i) => Array.from(vectors[i]));
  return {
    bytes: await captureBytes(db, name),
    expected: {
      kind: "flat",
      dims: 64,
      queries,
      results: queries.map((q) => ids(db.searchBinary(Uint8Array.from(q), 5))),
    },
  };
}

const fixtures = { [`${label}_hnsw_l2`]: await hnswFixture("l2", "l2", false) };
if (ev.JsIndexType) fixtures[`${label}_flat`] = await flatFixture("flat");
if (current) fixtures[`${label}_hnsw_cosine_bq`] = await hnswFixture("cos", "cosine", true);

const expectedPath = path.join(here, "expected.json");
const expected = fs.existsSync(expectedPath) ? JSON.parse(fs.readFileSync(expectedPath, "utf8")) : {};
for (const [name, f] of Object.entries(fixtures)) {
  fs.writeFileSync(path.join(here, `${name}.bin`), f.bytes);
  expected[name] = f.expected;
  console.log(`wrote ${name}.bin (${f.bytes.length} bytes)`);
}
fs.writeFileSync(expectedPath, JSON.stringify(expected, null, 2) + "\n");
