// Install the actual tarballs outside the checkout to detect omitted files.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtemp, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';
import ts from 'typescript';

const root = fileURLToPath(new URL('../', import.meta.url));
await build({ entryPoints: [path.join(root, 'wasm/dist/index.js')], bundle: true, platform: 'browser', format: 'esm', write: false });
const consumer = await mkdtemp(path.join(os.tmpdir(), 'edgevec-package-'));
const npm = process.env.npm_execpath;
assert.ok(npm, 'Run through npm run test:package');
const run = (args, cwd) => execFileSync(process.execPath, [npm, ...args], { cwd, encoding: 'utf8' });
const packs = [];
for (const directory of [root, path.join(root, 'pkg')]) {
  const [pack] = JSON.parse(run(['pack', '--ignore-scripts', '--json', '--pack-destination', consumer], directory));
  assert.ok(pack.files.some(file => file.path.endsWith('storage.js')), 'IndexedDB runtime snippet is included');
  packs.push(path.join(consumer, pack.filename));
}
await writeFile(path.join(consumer, 'package.json'), JSON.stringify({ private: true, type: 'module' }));
run(['install', '--offline', '--legacy-peer-deps', '--ignore-scripts', '--no-audit', '--no-fund', ...packs], consumer);
await writeFile(path.join(consumer, 'consumer.cts'), `
import EdgeVec = require('@edgevec/core');
const config: EdgeVec.EdgeVecClientConfig = { dimensions: 64 };
export const client: Promise<EdgeVec.EdgeVecClient> = EdgeVec.EdgeVecClient.create(config);
`);
await writeFile(path.join(consumer, 'consumer.mts'), `
import { EdgeVecClient } from '@edgevec/core';
import { EdgeVecIndex, type HybridSearchOptions } from 'edgevec';
export const options: HybridSearchOptions = { k: 10 };
export const client = EdgeVecClient.create({ dimensions: 64 });
export const Index = EdgeVecIndex;
`);
const program = ts.createProgram(['consumer.cts', 'consumer.mts'].map(name => path.join(consumer, name)), {
  noEmit: true, strict: true, skipLibCheck: false,
  target: ts.ScriptTarget.ES2020,
  module: ts.ModuleKind.Node16, moduleResolution: ts.ModuleResolutionKind.Node16,
  types: ['node'], typeRoots: [path.join(root, 'node_modules/@types')],
});
const diagnostics = ts.getPreEmitDiagnostics(program);
assert.equal(diagnostics.length, 0, ts.formatDiagnosticsWithColorAndContext(diagnostics, {
  getCurrentDirectory: () => consumer, getCanonicalFileName: name => name, getNewLine: () => '\n',
}));
await writeFile(path.join(consumer, 'smoke.mjs'), `
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { EdgeVecClient } from '@edgevec/core';
import { EdgeVecIndex, initSync, Filter, VectorType } from 'edgevec';
const vector = new Float32Array(64).fill(0.5);
const client = await EdgeVecClient.create({ dimensions: 64 });
const id = client.insert(vector);
assert.equal(client.search(vector, 1)[0].id, id);
initSync({ module: await readFile(new URL('./node_modules/edgevec/edgevec_bg.wasm', import.meta.url)) });
const index = new EdgeVecIndex({ dimensions: 64 });
index.add(vector, { label: 'packed' });
assert.equal((await index.search(vector, 1, { filter: Filter.eq('label', 'packed') })).length, 1);
assert.equal(EdgeVecIndex.fromBytes(index.toBytes()).dimensions, 64);
assert.equal(typeof VectorType.Binary, 'number');
const { EdgeVecClient: CommonJSClient } = createRequire(import.meta.url)('@edgevec/core');
const cjs = await CommonJSClient.create({ dimensions: 64 });
assert.equal(cjs.insert(vector), 1);
console.log('Packed ESM and CommonJS consumers passed');
`);
execFileSync(process.execPath, [path.join(consumer, 'smoke.mjs')], { cwd: consumer, stdio: 'inherit' });
console.log(`Validated tarballs in ${consumer}`);
