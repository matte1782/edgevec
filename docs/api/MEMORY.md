# Memory Management API Reference

**Version:** EdgeVec v0.6.0
**Last Updated:** 2025-12-22

EdgeVec provides a comprehensive memory pressure monitoring and management API for WASM environments where memory is limited.

---

## Overview

In browser environments, WASM modules have limited memory (typically 1-4GB max). The Memory Management API helps you:

1. **Monitor** current memory usage
2. **Detect** when memory pressure becomes critical
3. **Control** insert behavior based on available memory
4. **Optimize** memory usage through compaction

---

## API Reference

### getMemoryPressure()

Get current memory usage and pressure level.

```typescript
getMemoryPressure(): MemoryPressure
```

**Returns:**

```typescript
interface MemoryPressure {
  level: 'normal' | 'warning' | 'critical';
  usedBytes: number;
  totalBytes: number;
  usagePercent: number;
}
```

**Pressure Levels:**

| Level | Threshold | Meaning |
|:------|:----------|:--------|
| `normal` | <70% | Safe to insert |
| `warning` | 70-90% | Consider compaction |
| `critical` | >90% | Inserts may be blocked |

**Example:**

```typescript
const pressure = index.getMemoryPressure();

console.log(`Memory: ${pressure.usedBytes} / ${pressure.totalBytes}`);
console.log(`Usage: ${pressure.usagePercent.toFixed(1)}%`);
console.log(`Level: ${pressure.level}`);

// Example output:
// Memory: 52428800 / 268435456
// Usage: 19.5%
// Level: normal
```

---

### canInsert()

Check if inserts are allowed based on memory pressure.

```typescript
canInsert(): boolean
```

**Returns:** `false` when memory is at critical level (>90% by default).

**Example:**

```typescript
function safeInsert(vector: Float32Array, metadata: object): number | null {
    if (!index.canInsert()) {
        console.warn('Memory pressure critical, cannot insert');
        return null;
    }
    return index.insertWithMetadata(vector, metadata);
}
```

---

### setMemoryConfig(config)

Configure memory pressure thresholds and behavior.

```typescript
setMemoryConfig(config: MemoryConfig): void
```

**Parameters:**

```typescript
interface MemoryConfig {
  warningThreshold?: number;        // Default: 0.70 (70%)
  criticalThreshold?: number;       // Default: 0.90 (90%)
  blockInsertsAtCritical?: boolean; // Default: true
}
```

**Example:**

```typescript
// More aggressive thresholds for low-memory devices
index.setMemoryConfig({
    warningThreshold: 0.50,   // Warn at 50%
    criticalThreshold: 0.75, // Critical at 75%
    blockInsertsAtCritical: true
});

// Allow inserts even at critical (use with caution!)
index.setMemoryConfig({
    blockInsertsAtCritical: false
});
```

---

### getMemoryRecommendation()

Get actionable guidance based on current memory state.

```typescript
getMemoryRecommendation(): string
```

**Returns:** Human-readable suggestion.

**Example Outputs:**

| State | Recommendation |
|:------|:---------------|
| Normal, no tombstones | "Memory usage is healthy." |
| Normal, some tombstones | "Consider running compact() to reclaim 15% memory." |
| Warning | "Memory pressure elevated. Run compact() or reduce vector count." |
| Critical | "Memory critical. Immediate action required: compact() or delete vectors." |

**Example:**

```typescript
const recommendation = index.getMemoryRecommendation();
console.log(recommendation);

if (recommendation.includes('compact')) {
    const result = index.compact();
    console.log(`Compaction freed ${result.tombstones_removed} vectors`);
}
```

---

## Memory Optimization Strategies

### 1. Enable Binary Quantization

The most effective way to reduce memory: 32x reduction.

```typescript
// BQ is auto-enabled for dimensions divisible by 8
const index = new EdgeVec({ dimensions: 768 });

// Memory comparison (100k vectors, 768D):
// - F32: ~300 MB
// - BQ:  ~10 MB
```

### 2. Periodic Compaction

Remove deleted vectors to reclaim memory.

```typescript
// Check if compaction would help
if (index.needsCompaction()) {
    const before = index.getMemoryPressure();
    const result = index.compact();
    const after = index.getMemoryPressure();

    console.log(`Freed ${result.tombstones_removed} tombstones`);
    console.log(`Memory: ${before.usagePercent}% -> ${after.usagePercent}%`);
}
```

### 3. Automatic Compaction Policy

```typescript
function autoCompact(index: EdgeVec): void {
    const pressure = index.getMemoryPressure();
    const ratio = index.tombstoneRatio();

    // Compact if:
    // 1. Memory pressure is warning+ AND tombstone ratio > 10%
    // 2. Tombstone ratio exceeds 30% regardless of pressure
    if ((pressure.level !== 'normal' && ratio > 0.1) || ratio > 0.3) {
        console.log('Auto-compacting...');
        index.compact();
    }
}

// Call periodically or after batch deletions
autoCompact(index);
```

### 4. Pre-Insert Memory Check

```typescript
async function insertWithMemoryCheck(
    index: EdgeVec,
    vectors: Float32Array[],
    onMemoryWarning: () => void
): Promise<number[]> {
    const ids: number[] = [];

    for (const vector of vectors) {
        // Check before each insert
        if (!index.canInsert()) {
            // Try compaction first
            if (index.needsCompaction()) {
                index.compact();
            }

            // Still can't insert?
            if (!index.canInsert()) {
                onMemoryWarning();
                break;
            }
        }

        ids.push(index.insert(vector));
    }

    return ids;
}
```

---

## Memory Usage Estimation

### Per-Vector Memory

| Component | F32 Mode | BQ Mode |
|:----------|:---------|:--------|
| Vector data | 4 * dim bytes | dim/8 bytes |
| HNSW node | ~64 bytes | ~64 bytes |
| Metadata (avg) | ~100 bytes | ~100 bytes |

**Examples (768D):**

| Mode | Per Vector | 100k Vectors | 1M Vectors |
|:-----|:-----------|:-------------|:-----------|
| F32 only | ~3.2 KB | ~320 MB | ~3.2 GB |
| F32 + BQ | ~3.3 KB | ~330 MB | ~3.3 GB |
| BQ only | ~260 bytes | ~26 MB | ~260 MB |

### Estimating Capacity

```typescript
function estimateCapacity(
    dimensions: number,
    useBQ: boolean,
    maxMemoryMB: number
): number {
    const perVectorBytes = useBQ
        ? (dimensions / 8) + 64 + 100   // BQ + node + metadata
        : (dimensions * 4) + 64 + 100;  // F32 + node + metadata

    const maxBytes = maxMemoryMB * 1024 * 1024;
    const overhead = 0.2; // 20% overhead for WASM heap management

    return Math.floor(maxBytes * (1 - overhead) / perVectorBytes);
}

// Examples:
console.log('F32 capacity (768D, 1GB):', estimateCapacity(768, false, 1024));
// ~312,500 vectors

console.log('BQ capacity (768D, 1GB):', estimateCapacity(768, true, 1024));
// ~3,355,443 vectors
```

---

## Monitoring Dashboard Example

```typescript
function createMemoryDashboard(index: EdgeVec): object {
    const pressure = index.getMemoryPressure();
    const stats = {
        vectorCount: index.vectorCount(),
        liveCount: index.liveCount(),
        deletedCount: index.deletedCount(),
        tombstoneRatio: (index.tombstoneRatio() * 100).toFixed(1) + '%'
    };

    return {
        memory: {
            used: formatBytes(pressure.usedBytes),
            total: formatBytes(pressure.totalBytes),
            percent: pressure.usagePercent.toFixed(1) + '%',
            level: pressure.level
        },
        vectors: stats,
        recommendation: index.getMemoryRecommendation(),
        canInsert: index.canInsert(),
        needsCompaction: index.needsCompaction(),
        hasBQ: index.hasBQ()
    };
}

function formatBytes(bytes: number): string {
    if (bytes < 1024) return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
    if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
    return (bytes / (1024 * 1024 * 1024)).toFixed(1) + ' GB';
}

// Usage
const dashboard = createMemoryDashboard(index);
console.log(JSON.stringify(dashboard, null, 2));
```

---

## Best Practices

1. **Monitor regularly** - Check memory pressure before bulk operations
2. **Compact proactively** - Don't wait for critical pressure
3. **Use BQ when possible** - 32x memory savings with minimal recall loss
4. **Set appropriate thresholds** - Lower on mobile, higher on desktop
5. **Handle insert blocks gracefully** - Provide user feedback

---

## Error Handling

```typescript
try {
    if (!index.canInsert()) {
        throw new Error('Memory pressure too high');
    }
    index.insert(vector);
} catch (e) {
    if (e.message.includes('memory')) {
        // Memory-related error
        console.warn('Memory issue:', e.message);

        // Try to recover
        if (index.needsCompaction()) {
            index.compact();
        }
    } else {
        throw e;
    }
}
```

---

## See Also

- [WASM_INDEX.md](./WASM_INDEX.md) - Complete EdgeVec API
- [Binary Quantization Guide](../guides/BINARY_QUANTIZATION.md)
- [Performance Tuning](../PERFORMANCE_TUNING.md)
