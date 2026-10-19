// EdgeVec Node.js Quick Start Example
// Demonstrates the WASM API with Node.js file persistence.

import { EdgeVec, EdgeVecConfig, initSync } from '@edgevec/core/wasm';
import { readFile, writeFile } from 'node:fs/promises';

async function main() {
    console.log('EdgeVec Node.js Quick Start\n');

    // 1. Initialize the web-target WASM package from disk, then create an index.
    const entry = import.meta.resolve('@edgevec/core/wasm');
    initSync({ module: await readFile(new URL('edgevec_bg.wasm', entry)) });
    const config = new EdgeVecConfig(128);
    config.metric = 'cosine'; // Options: 'l2', 'cosine', 'dot'
    const client = new EdgeVec(config);
    console.log(`✓ Index created (${client.dimensions()}d, cosine metric)\n`);

    // 2. Insert vectors (synchronous)
    console.log('Inserting vectors...');
    const vector1 = new Float32Array(128).fill(0.1);
    const vector2 = new Float32Array(128).fill(0.2);
    const vector3 = new Float32Array(128).fill(0.3);

    const id1 = client.insert(vector1);
    const id2 = client.insert(vector2);
    const id3 = client.insert(vector3);

    console.log(`✓ Inserted 3 vectors with IDs: ${id1}, ${id2}, ${id3}`);
    console.log(`  Total vectors: ${client.liveCount()}\n`);

    // 3. Search for nearest neighbors (synchronous)
    console.log('Searching for nearest neighbors...');
    const query = new Float32Array(128).fill(0.15); // Query closer to vector1
    const results = client.search(query, 2); // Find top 2

    console.log(`✓ Found ${results.length} results:`);
    results.forEach((result, i) => {
        console.log(`  ${i + 1}. ID: ${result.id}, Distance: ${result.score.toFixed(6)}`);
    });
    console.log();

    // 4. Persist the buffer to a file chosen by the application.
    console.log('Saving database...');
    await writeFile('quickstart-db.evwb', client.toBytes());
    console.log('✓ Database saved\n');

    // 5. Load from storage
    console.log('Loading database...');
    const loadedClient = EdgeVec.fromBytes(await readFile('quickstart-db.evwb'));
    console.log(`✓ Database loaded`);
    console.log(`  Vectors in loaded index: ${loadedClient.liveCount()}\n`);

    // 6. Verify loaded database works
    console.log('Verifying loaded database...');
    const verifyQuery = new Float32Array(128).fill(0.25); // Query closer to vector2
    const verifyResults = loadedClient.search(verifyQuery, 2);
    console.log(`✓ Search on loaded database returned ${verifyResults.length} results:`);
    verifyResults.forEach((result, i) => {
        console.log(`  ${i + 1}. ID: ${result.id}, Distance: ${result.score.toFixed(6)}`);
    });

    loadedClient.free();
    client.free();
    config.free();
    console.log('\n✓ Quick start complete!');
}

main().catch(err => {
    console.error('✗ Error:', err.message);
    console.error(err.stack);
    process.exit(1);
});
