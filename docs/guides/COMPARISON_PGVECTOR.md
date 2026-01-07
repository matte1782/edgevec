# EdgeVec vs pgvector: Choosing the Right Vector Database

This guide compares EdgeVec with pgvector to help you choose the right vector database for your use case. Both are excellent tools with different architectural philosophies and deployment models.

## Overview

| Aspect | EdgeVec | pgvector |
|:-------|:--------|:---------|
| **Type** | Embedded vector database | PostgreSQL extension |
| **Language** | Rust/WASM | C |
| **Deployment** | In-process, browser, edge | PostgreSQL server |
| **License** | MIT | PostgreSQL License |

**EdgeVec** is an embedded, WASM-native vector database designed for edge deployment, browser applications, and scenarios requiring zero network latency.

**pgvector** is a PostgreSQL extension that adds vector similarity search capabilities to existing PostgreSQL deployments.

## Feature Comparison

| Feature | EdgeVec | pgvector |
|:--------|:--------|:---------|
| **Index Types** | HNSW, Flat | IVFFlat, HNSW |
| **Quantization** | Binary, Scalar | Half-precision (fp16) |
| **Max Dimensions** | Unlimited | 2,000 (default, configurable) |
| **Filtering** | Integrated expressions | SQL WHERE clauses |
| **Persistence** | File, IndexedDB (browser) | PostgreSQL storage |
| **Transactions** | Single-writer | Full ACID |
| **Concurrency** | Read-many, write-exclusive | Full MVCC |
| **Distance Metrics** | Euclidean, Cosine, Dot, Hamming | Euclidean, Cosine, Inner Product |

### Index Types

**EdgeVec:**
- **HNSW**: Primary index type, optimized for recall/latency trade-off
- **Flat**: Exact search, no indexing overhead

**pgvector:**
- **IVFFlat**: Inverted file with flat quantization, faster build times
- **HNSW**: Hierarchical navigable small world, better recall

### Quantization

**EdgeVec** offers binary quantization (1-bit per dimension) which achieves:
- 32x memory reduction vs float32
- 10-50x faster distance computation via Hamming distance
- 85-95% recall retention (depends on data distribution)

**pgvector** supports half-precision (fp16) storage, providing:
- 2x memory reduction vs float32
- GPU acceleration compatibility
- No recall loss

## Architecture Comparison

### EdgeVec Architecture

```
┌─────────────────────────────────────────┐
│           Your Application              │
│  ┌───────────────────────────────────┐  │
│  │         EdgeVec (in-process)      │  │
│  │  ┌─────────┐  ┌────────────────┐  │  │
│  │  │  HNSW   │  │  Storage Layer │  │  │
│  │  └─────────┘  └────────────────┘  │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

- **Zero network latency**: Direct memory access
- **Single binary**: No external dependencies
- **WASM deployment**: Same code runs in browser and server
- **Offline-first**: Full functionality without connectivity

### pgvector Architecture

```
┌─────────────────────────────────────────┐
│           Your Application              │
└───────────────┬─────────────────────────┘
                │ Network
┌───────────────▼─────────────────────────┐
│           PostgreSQL Server             │
│  ┌───────────────────────────────────┐  │
│  │        pgvector Extension         │  │
│  │  ┌─────────┐  ┌────────────────┐  │  │
│  │  │  Index  │  │  PostgreSQL    │  │  │
│  │  │  (HNSW) │  │  Storage       │  │  │
│  │  └─────────┘  └────────────────┘  │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

- **Client-server**: Requires network round-trip
- **PostgreSQL ecosystem**: Rich SQL, extensions, tools
- **ACID guarantees**: Full transaction support
- **Horizontal scaling**: Read replicas, partitioning

## Performance Characteristics

> **Note**: Fair benchmarks require identical hardware and datasets. These are representative ranges, not direct comparisons.

### Latency

| Operation | EdgeVec (Target) | pgvector (Typical) |
|:----------|:-----------------|:-------------------|
| Single query (100K vectors) | <10ms | 10-50ms |
| Batch query (100 queries) | <100ms | 100-500ms |
| Insert single | <5ms | 1-10ms |
| Bulk insert (10K vectors) | <500ms | 1-5s |

**EdgeVec advantage**: No network latency, optimized for single-query patterns.

**pgvector advantage**: Batch operations can amortize connection overhead.

### Memory

| Metric | EdgeVec | pgvector |
|:-------|:--------|:---------|
| Memory per vector (f32, 768d) | ~100 bytes (indexed) | ~3KB+ (with row overhead) |
| Memory per vector (binary quantized, 768d) | ~100 bytes | N/A |
| Index overhead | ~50-100 bytes/vector | ~64 bytes/vector |

### Scalability

| Scale | EdgeVec | pgvector |
|:------|:--------|:---------|
| 10K vectors | Excellent | Excellent |
| 100K vectors | Excellent | Excellent |
| 1M vectors | Good | Good |
| 10M+ vectors | Limited by RAM | Disk-based, slower |

## When to Choose EdgeVec

Choose EdgeVec when you need:

1. **Browser/Edge deployment**
   - PWAs with vector search
   - Client-side semantic search
   - Offline-capable applications

2. **Embedded use cases**
   - Mobile applications (via WASM)
   - IoT devices with limited resources
   - Desktop applications without server dependency

3. **Ultra-low latency**
   - Real-time recommendations (<10ms)
   - Interactive search experiences
   - Gaming/VR applications

4. **Simplified deployment**
   - No database server to manage
   - Single binary distribution
   - Reduced operational complexity

5. **Privacy-sensitive applications**
   - Data stays on device
   - No network exposure
   - GDPR/compliance considerations

## When to Choose pgvector

Choose pgvector when you need:

1. **Existing PostgreSQL infrastructure**
   - Already running PostgreSQL
   - Familiar with PostgreSQL operations
   - Existing backup/monitoring in place

2. **ACID transactions**
   - Vectors tied to relational data
   - Multi-table consistency required
   - Rollback capabilities

3. **Complex SQL queries**
   - JOINs with vector results
   - Aggregations over vector matches
   - Complex filtering logic

4. **Team expertise**
   - SQL-based team
   - PostgreSQL administration skills
   - Existing PostgreSQL tooling

5. **Large-scale deployments**
   - Read replicas for scaling
   - PostgreSQL partitioning
   - Managed database services (RDS, Cloud SQL)

## Hybrid Architectures

Consider using both for different tiers:

```
┌─────────────────────────────────────────────────────────────┐
│                     Client Application                       │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  EdgeVec (in-browser)                               │    │
│  │  - Recent/frequent queries cached                   │    │
│  │  - Offline search capability                        │    │
│  │  - <10ms latency                                    │    │
│  └───────────────────────┬─────────────────────────────┘    │
└──────────────────────────┼──────────────────────────────────┘
                           │ Cache miss / Sync
┌──────────────────────────▼──────────────────────────────────┐
│                     Backend Server                           │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  pgvector (PostgreSQL)                              │    │
│  │  - Full corpus                                      │    │
│  │  - ACID transactions                                │    │
│  │  - Complex queries                                  │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## Migration Considerations

### From pgvector to EdgeVec

1. **Export vectors**:
   ```sql
   COPY (SELECT id, embedding FROM items) TO '/tmp/vectors.csv' WITH CSV;
   ```

2. **Convert to EdgeVec format**:
   ```rust
   let mut index = HnswIndex::new(HnswConfig::default());
   for (id, embedding) in csv_reader {
       index.add(id, &embedding)?;
   }
   index.save("vectors.edgevec")?;
   ```

3. **Migrate filters**: Convert SQL WHERE clauses to EdgeVec filter expressions.

### From EdgeVec to pgvector

1. **Export from EdgeVec**:
   ```rust
   let vectors = index.export_all()?;
   ```

2. **Import to pgvector**:
   ```sql
   CREATE TABLE items (id bigint, embedding vector(768));
   INSERT INTO items (id, embedding) VALUES ...;
   CREATE INDEX ON items USING hnsw (embedding vector_cosine_ops);
   ```

## Summary

| Criteria | Winner |
|:---------|:-------|
| Browser deployment | EdgeVec |
| Latency-critical | EdgeVec |
| PostgreSQL integration | pgvector |
| ACID transactions | pgvector |
| Memory efficiency | EdgeVec (with binary quantization) |
| SQL ecosystem | pgvector |
| Offline capability | EdgeVec |
| Operational simplicity | EdgeVec |

Both tools excel in their respective niches. EdgeVec is the clear choice for edge/embedded deployments, while pgvector is ideal for PostgreSQL-centric architectures.

## Further Reading

- [EdgeVec Binary Quantization Guide](./BINARY_QUANTIZATION.md)
- [EdgeVec Filter Examples](./FILTER_EXAMPLES.md)
- [pgvector GitHub](https://github.com/pgvector/pgvector)
- [pgvector Documentation](https://github.com/pgvector/pgvector#readme)
