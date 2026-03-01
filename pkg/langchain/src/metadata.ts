/**
 * Metadata Serialization Module for edgevec-langchain
 *
 * Handles conversion between LangChain's flexible metadata format
 * and EdgeVec's `MetadataValue` type (`string | number | boolean | string[]`).
 *
 * Unsupported types (nested objects, null, Date, number[]) are JSON-stringified
 * and tracked in a `_serializedKeys` metadata field for lossless round-tripping.
 *
 * @module metadata
 */

import type { Metadata } from "edgevec/edgevec-wrapper.js";
import { MetadataSerializationError } from "./types.js";

/** Internal prefix — collision-resistant to avoid clashing with user metadata keys [C2 FIX] */
const INTERNAL_PREFIX = "__edgevec_";

/** Metadata key that tracks which keys were JSON-stringified */
const SERIALIZED_KEYS_FIELD = `${INTERNAL_PREFIX}serializedKeys`;

/** Metadata key that tracks which keys were originally null */
const NULL_KEYS_FIELD = `${INTERNAL_PREFIX}nullKeys`;

/** Internal metadata key for storing pageContent */
export const PAGE_CONTENT_KEY = `${INTERNAL_PREFIX}pageContent`;
/** Internal metadata key for storing document ID */
export const ID_KEY = `${INTERNAL_PREFIX}id`;

/** Reserved internal metadata keys — all use __edgevec_ prefix [C2 FIX] */
const RESERVED_KEYS = new Set([
  PAGE_CONTENT_KEY,
  ID_KEY,
  SERIALIZED_KEYS_FIELD,
  NULL_KEYS_FIELD,
]);

/**
 * Check if a value is a native EdgeVec MetadataValue (no conversion needed).
 *
 * [C1 FIX] NaN, Infinity, -Infinity are NOT native — they corrupt on JSON round-trip.
 * [M2 FIX] Sparse arrays and empty arrays are NOT native string[].
 */
function isNativeMetadataValue(
  value: unknown
): value is string | number | boolean | string[] {
  if (typeof value === "string") return true;
  if (typeof value === "number") {
    // [C1 FIX] Reject non-finite numbers — they become null via JSON.stringify
    return Number.isFinite(value) && !Object.is(value, -0);
  }
  if (typeof value === "boolean") return true;
  if (Array.isArray(value)) {
    // [M2 FIX] Must be a non-sparse array where every element is a string
    // .every() vacuously returns true on sparse arrays, so check length matches
    if (value.length === 0) return true; // empty string[] is valid
    let count = 0;
    for (let i = 0; i < value.length; i++) {
      if (!(i in value)) return false; // sparse slot detected
      if (typeof value[i] !== "string") return false;
      count++;
    }
    return count === value.length;
  }
  return false;
}

/**
 * Serialize LangChain metadata into EdgeVec-compatible metadata.
 *
 * - Native types (string, number, boolean, string[]) pass through unchanged.
 * - `null` is coerced to `""` and tracked in `_nullKeys`.
 * - All other types (object, Date, number[], etc.) are JSON-stringified
 *   and tracked in `_serializedKeys`.
 *
 * @param metadata - LangChain Document metadata (Record<string, any>)
 * @returns EdgeVec-compatible metadata with tracking fields
 * @throws {MetadataSerializationError} On circular references
 */
export function serializeMetadata(
  metadata: Record<string, unknown>
): Metadata {
  const result: Metadata = {};
  const serializedKeys: string[] = [];
  const nullKeys: string[] = [];

  for (const [key, value] of Object.entries(metadata)) {
    // Skip reserved internal keys
    if (RESERVED_KEYS.has(key)) continue;

    if (value === null || value === undefined) {
      // Null/undefined → empty string, tracked for round-trip
      result[key] = "";
      nullKeys.push(key);
    } else if (isNativeMetadataValue(value)) {
      // Native types pass through
      result[key] = value;
    } else {
      // Everything else → JSON.stringify with circular ref detection
      result[key] = safeStringify(value, key);
      serializedKeys.push(key);
    }
  }

  // Store tracking arrays (only if non-empty)
  if (serializedKeys.length > 0) {
    result[SERIALIZED_KEYS_FIELD] = serializedKeys;
  }
  if (nullKeys.length > 0) {
    result[NULL_KEYS_FIELD] = nullKeys;
  }

  return result;
}

/**
 * Deserialize EdgeVec metadata back to LangChain format.
 *
 * Reverses the serialization:
 * - Keys in `_serializedKeys` are JSON.parsed back to original type
 * - Keys in `_nullKeys` are restored to `null`
 * - Internal keys (`_pageContent`, `_id`, tracking fields) are stripped
 *
 * @param metadata - EdgeVec metadata (from search results)
 * @returns LangChain-compatible metadata
 */
export function deserializeMetadata(
  metadata: Metadata
): Record<string, unknown> {
  const result: Record<string, unknown> = {};

  // [C3 FIX] Validate tracking fields are actually string[] before trusting them
  const serializedKeys = new Set<string>(
    validateStringArray(metadata[SERIALIZED_KEYS_FIELD])
  );
  const nullKeys = new Set<string>(
    validateStringArray(metadata[NULL_KEYS_FIELD])
  );

  for (const [key, value] of Object.entries(metadata)) {
    // Skip internal tracking fields
    if (RESERVED_KEYS.has(key)) continue;

    if (nullKeys.has(key)) {
      result[key] = null;
    } else if (serializedKeys.has(key) && typeof value === "string") {
      try {
        result[key] = JSON.parse(value);
      } catch {
        // If JSON.parse fails, return raw string
        result[key] = value;
      }
    } else {
      result[key] = value;
    }
  }

  return result;
}

/**
 * Validate that a metadata value is a string[] (for tracking fields).
 * Returns empty array if the value is not a valid string[].
 * [C3 FIX] Prevents type confusion when _serializedKeys/_nullKeys are corrupted.
 */
function validateStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  if (!value.every((item): item is string => typeof item === "string")) return [];
  return value;
}

/**
 * JSON.stringify with circular reference detection.
 *
 * [M3 FIX] Relies on native JSON.stringify circular detection (throws TypeError).
 * This correctly handles DAG structures (shared references) without false positives.
 * The native implementation tracks the serialization stack internally.
 *
 * @param value - Value to stringify
 * @param key - Metadata key (for error messages)
 * @returns JSON string
 * @throws {MetadataSerializationError} On circular references or serialization failure
 */
function safeStringify(value: unknown, key: string): string {
  try {
    return JSON.stringify(value);
  } catch (e) {
    // Native JSON.stringify throws TypeError on circular structures
    // V8: "Converting circular structure to JSON"
    // SpiderMonkey: "cyclic object value"
    if (e instanceof TypeError && /(circular|cyclic)/i.test(e.message)) {
      throw new MetadataSerializationError(
        `Circular reference detected in metadata key "${key}"`
      );
    }
    throw new MetadataSerializationError(
      `Failed to serialize metadata key "${key}": ${e instanceof Error ? e.message : String(e)}`
    );
  }
}
