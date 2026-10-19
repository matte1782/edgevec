import { execFileSync } from 'node:child_process';
import { readFile, readdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';
import { generateDtsBundle } from 'dts-bundle-generator';

const root = fileURLToPath(new URL('../', import.meta.url));
const tsc = path.join(root, 'node_modules/typescript/bin/tsc');
const disposableLib = '/// <reference lib="esnext.disposable" />\n';
const rawTypesPath = path.join(root, 'pkg/edgevec.d.ts');
const rawTypes = await readFile(rawTypesPath, 'utf8');
if (!rawTypes.includes('reference lib="esnext.disposable"')) {
  await writeFile(rawTypesPath, disposableLib + rawTypes);
}
for (const project of ['pkg/tsconfig.json', 'wasm/tsconfig.build.json']) {
  execFileSync(process.execPath, [tsc, '--project', project], { cwd: root, stdio: 'inherit' });
}
const entryPath = path.join(root, 'pkg/index.js');
await writeFile(entryPath, (await readFile(entryPath, 'utf8')).replace(/[ \t]+$/gm, ''));

// Source clients live in wasm/, while emitted clients live in wasm/dist/.
// Keep the shared raw runtime at pkg/ rather than shipping a second WASM copy.
const output = path.join(root, 'wasm/dist');
for (const name of await readdir(output)) {
  if (!name.endsWith('.js') && !name.endsWith('.d.ts')) continue;
  const file = path.join(output, name);
  const source = await readFile(file, 'utf8');
  await writeFile(file, source.replace(/(['"])\.\.\/pkg\//g, '$1../../pkg/'));
}

// Bundle the existing CommonJS entry point, including the generated JS glue.
// Node 16 cannot require the ESM distribution directly.
await build({
  entryPoints: [path.join(root, 'wasm/index.ts')],
  outfile: path.join(root, 'wasm/index.cjs'),
  bundle: true,
  platform: 'node',
  format: 'cjs',
  target: 'node16',
  define: { 'import.meta.url': '__edgevecModuleUrl' },
  banner: { js: "const __edgevecModuleUrl = require('node:url').pathToFileURL(__filename).href;" },
});
const [commonJsTypes] = generateDtsBundle([
  { filePath: path.join(root, 'wasm/index.ts'), output: { noBanner: true } },
], { preferredConfigPath: path.join(root, 'wasm/tsconfig.build.json') });
await writeFile(path.join(root, 'wasm/index.d.cts'), disposableLib + commonJsTypes);

// wasm-pack regenerates this manifest, so restore the complete public package
// after every build. Snippets are required by the generated WASM glue.
const manifestPath = path.join(root, 'pkg/package.json');
const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
manifest.main = 'index.js';
manifest.types = 'index.d.ts';
manifest.exports = {
  '.': { types: './index.d.ts', import: './index.js' },
  './wasm': { types: './edgevec.d.ts', import: './edgevec.js' },
  './react': { types: './react/index.d.ts', import: './react/index.js' },
  './vue': { types: './vue/index.d.ts', import: './vue/index.js' },
  './*': './*',
};
manifest.files = ['*.js', '*.d.ts', 'edgevec_bg.wasm', 'snippets', 'react', 'vue', 'LICENSE-APACHE', 'LICENSE-MIT', 'README.md'];
manifest.peerDependencies = { react: '>=18', vue: '>=3.3' };
manifest.peerDependenciesMeta = { react: { optional: true }, vue: { optional: true } };
await writeFile(manifestPath, JSON.stringify(manifest, null, 2) + '\n');
// wasm-pack writes a '*' .gitignore. npm uses it for nested directories too,
// which otherwise strips pkg/ from the parent @edgevec/core tarball.
await writeFile(path.join(root, 'pkg/.npmignore'), '# Package contents are controlled by package.json files.\n');
