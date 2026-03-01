import { describe, it, expect } from "vitest";
import { serializeMetadata, deserializeMetadata } from "../src/metadata.js";
import { MetadataSerializationError } from "../src/types.js";

describe("serializeMetadata", () => {
  it("passes through string values unchanged", () => {
    const meta = { name: "test", category: "gpu" };
    const result = serializeMetadata(meta);
    expect(result.name).toBe("test");
    expect(result.category).toBe("gpu");
    expect(result._serializedKeys).toBeUndefined();
  });

  it("passes through number values unchanged", () => {
    const meta = { price: 499.99, count: 42 };
    const result = serializeMetadata(meta);
    expect(result.price).toBe(499.99);
    expect(result.count).toBe(42);
  });

  it("passes through boolean values unchanged", () => {
    const meta = { active: true, deleted: false };
    const result = serializeMetadata(meta);
    expect(result.active).toBe(true);
    expect(result.deleted).toBe(false);
  });

  it("passes through string[] values unchanged", () => {
    const meta = { tags: ["gpu", "nvidia", "rtx"] };
    const result = serializeMetadata(meta);
    expect(result.tags).toEqual(["gpu", "nvidia", "rtx"]);
  });

  it("serializes null to empty string and tracks in __edgevec_nullKeys", () => {
    const meta = { value: null };
    const result = serializeMetadata(meta as Record<string, unknown>);
    expect(result.value).toBe("");
    expect(result.__edgevec_nullKeys).toEqual(["value"]);
  });

  it("serializes undefined to empty string and tracks in __edgevec_nullKeys", () => {
    const meta = { value: undefined };
    const result = serializeMetadata(meta as Record<string, unknown>);
    expect(result.value).toBe("");
    expect(result.__edgevec_nullKeys).toEqual(["value"]);
  });

  it("JSON-stringifies nested objects and tracks in __edgevec_serializedKeys", () => {
    const meta = { nested: { a: 1, b: "two" } };
    const result = serializeMetadata(meta as Record<string, unknown>);
    expect(result.nested).toBe('{"a":1,"b":"two"}');
    expect(result.__edgevec_serializedKeys).toEqual(["nested"]);
  });

  it("JSON-stringifies Date objects and tracks in __edgevec_serializedKeys", () => {
    const date = new Date("2026-03-07T00:00:00.000Z");
    const meta = { created: date };
    const result = serializeMetadata(meta as Record<string, unknown>);
    expect(typeof result.created).toBe("string");
    expect(result.__edgevec_serializedKeys).toEqual(["created"]);
  });

  it("JSON-stringifies number[] and tracks in __edgevec_serializedKeys", () => {
    const meta = { scores: [1, 2, 3] };
    const result = serializeMetadata(meta as Record<string, unknown>);
    expect(result.scores).toBe("[1,2,3]");
    expect(result.__edgevec_serializedKeys).toEqual(["scores"]);
  });

  it("handles mixed types correctly", () => {
    const meta = {
      name: "test",
      price: 100,
      active: true,
      tags: ["a", "b"],
      nested: { x: 1 },
      empty: null,
    };
    const result = serializeMetadata(meta as Record<string, unknown>);
    expect(result.name).toBe("test");
    expect(result.price).toBe(100);
    expect(result.active).toBe(true);
    expect(result.tags).toEqual(["a", "b"]);
    expect(result.nested).toBe('{"x":1}');
    expect(result.empty).toBe("");
    expect(result.__edgevec_serializedKeys).toEqual(["nested"]);
    expect(result.__edgevec_nullKeys).toEqual(["empty"]);
  });

  it("throws MetadataSerializationError on circular reference", () => {
    const obj: Record<string, unknown> = { a: 1 };
    obj.self = obj;
    expect(() => serializeMetadata({ circular: obj })).toThrow(
      MetadataSerializationError
    );
    expect(() => serializeMetadata({ circular: obj })).toThrow(
      /Circular reference detected/
    );
  });

  it("skips reserved keys (__edgevec_ prefixed)", () => {
    const meta = {
      __edgevec_pageContent: "should be skipped",
      __edgevec_id: "should be skipped",
      name: "kept",
    };
    const result = serializeMetadata(meta as Record<string, unknown>);
    expect(result.__edgevec_pageContent).toBeUndefined();
    expect(result.__edgevec_id).toBeUndefined();
    expect(result.name).toBe("kept");
  });

  // [C1 FIX] NaN, Infinity, -Infinity are serialized, not passed through
  it("serializes NaN via _serializedKeys (not pass-through)", () => {
    const meta = { value: NaN };
    const result = serializeMetadata(meta as Record<string, unknown>);
    expect(typeof result.value).toBe("string"); // JSON-stringified
    expect(result.__edgevec_serializedKeys).toContain("value");
  });

  it("serializes Infinity via _serializedKeys", () => {
    const meta = { value: Infinity };
    const result = serializeMetadata(meta as Record<string, unknown>);
    expect(typeof result.value).toBe("string");
    expect(result.__edgevec_serializedKeys).toContain("value");
  });

  it("serializes -Infinity via _serializedKeys", () => {
    const meta = { value: -Infinity };
    const result = serializeMetadata(meta as Record<string, unknown>);
    expect(typeof result.value).toBe("string");
    expect(result.__edgevec_serializedKeys).toContain("value");
  });

  it("serializes -0 via _serializedKeys", () => {
    const meta = { value: -0 };
    const result = serializeMetadata(meta as Record<string, unknown>);
    expect(typeof result.value).toBe("string");
    expect(result.__edgevec_serializedKeys).toContain("value");
  });

  // [C2 FIX] User data with reserved-looking keys is NOT silently dropped
  it("does NOT drop user keys that happen to look internal (no __edgevec_ prefix)", () => {
    const meta = { _serializedKeys: ["user_data"], _nullKeys: ["user_data"], name: "kept" };
    const result = serializeMetadata(meta as Record<string, unknown>);
    // These are NOT __edgevec_ prefixed, so they should be preserved (serialized)
    expect(result.name).toBe("kept");
    // _serializedKeys and _nullKeys without prefix are treated as user data
    // They are string[] so they pass through as native
    expect(result._serializedKeys).toEqual(["user_data"]);
    expect(result._nullKeys).toEqual(["user_data"]);
  });

  // [M2 FIX] Sparse arrays are serialized, not passed through
  it("serializes sparse arrays via _serializedKeys", () => {
    const sparse = new Array(3); // [empty × 3]
    const meta = { arr: sparse };
    const result = serializeMetadata(meta as Record<string, unknown>);
    expect(typeof result.arr).toBe("string");
    expect(result.__edgevec_serializedKeys).toContain("arr");
  });

  // [M3 FIX] DAG structures (shared refs) do NOT throw
  it("handles DAG structures (shared references) without false positive", () => {
    const shared = { x: 1 };
    const meta = { a: shared, b: shared };
    // Should NOT throw — this is a DAG, not circular
    const result = serializeMetadata(meta as Record<string, unknown>);
    expect(result.__edgevec_serializedKeys).toContain("a");
    expect(result.__edgevec_serializedKeys).toContain("b");
  });

  it("handles empty metadata object", () => {
    const result = serializeMetadata({});
    expect(Object.keys(result).length).toBe(0);
  });
});

describe("deserializeMetadata", () => {
  it("passes through native values unchanged", () => {
    const meta = { name: "test", price: 100, active: true, tags: ["a"] };
    const result = deserializeMetadata(meta);
    expect(result).toEqual({ name: "test", price: 100, active: true, tags: ["a"] });
  });

  it("restores null from __edgevec_nullKeys tracking", () => {
    const meta = { value: "", __edgevec_nullKeys: ["value"] };
    const result = deserializeMetadata(meta);
    expect(result.value).toBeNull();
    expect(result.__edgevec_nullKeys).toBeUndefined();
  });

  it("JSON-parses values in __edgevec_serializedKeys", () => {
    const meta = {
      nested: '{"a":1,"b":"two"}',
      __edgevec_serializedKeys: ["nested"],
    };
    const result = deserializeMetadata(meta);
    expect(result.nested).toEqual({ a: 1, b: "two" });
    expect(result.__edgevec_serializedKeys).toBeUndefined();
  });

  it("strips reserved internal keys (__edgevec_ prefixed) from output", () => {
    const meta = {
      __edgevec_pageContent: "hello",
      __edgevec_id: "abc",
      __edgevec_serializedKeys: [] as string[],
      __edgevec_nullKeys: [] as string[],
      name: "kept",
    };
    const result = deserializeMetadata(meta);
    expect(result.__edgevec_pageContent).toBeUndefined();
    expect(result.__edgevec_id).toBeUndefined();
    expect(result.__edgevec_serializedKeys).toBeUndefined();
    expect(result.__edgevec_nullKeys).toBeUndefined();
    expect(result.name).toBe("kept");
  });

  // [C3 FIX] Corrupted _serializedKeys is handled gracefully
  it("handles corrupted _serializedKeys (not an array) gracefully", () => {
    const meta = {
      __edgevec_serializedKeys: "not-an-array" as unknown as string[],
      name: "test",
    };
    const result = deserializeMetadata(meta);
    expect(result.name).toBe("test");
    // Should not crash, just treat as no serialized keys
  });

  it("handles corrupted _nullKeys (number) gracefully", () => {
    const meta = {
      __edgevec_nullKeys: 42 as unknown as string[],
      name: "test",
      value: "",
    };
    const result = deserializeMetadata(meta);
    expect(result.name).toBe("test");
    expect(result.value).toBe(""); // Not restored to null since _nullKeys is invalid
  });

  it("handles corrupted JSON gracefully (returns raw string)", () => {
    const meta = {
      broken: "not-valid-json{",
      __edgevec_serializedKeys: ["broken"],
    };
    const result = deserializeMetadata(meta);
    expect(result.broken).toBe("not-valid-json{");
  });
});

describe("round-trip: serialize → deserialize", () => {
  it("round-trips string values", () => {
    const original = { name: "test" };
    const result = deserializeMetadata(serializeMetadata(original));
    expect(result).toEqual(original);
  });

  it("round-trips number values", () => {
    const original = { price: 499.99 };
    const result = deserializeMetadata(serializeMetadata(original));
    expect(result).toEqual(original);
  });

  it("round-trips boolean values", () => {
    const original = { active: true, deleted: false };
    const result = deserializeMetadata(serializeMetadata(original));
    expect(result).toEqual(original);
  });

  it("round-trips string[] values", () => {
    const original = { tags: ["gpu", "nvidia"] };
    const result = deserializeMetadata(serializeMetadata(original));
    expect(result).toEqual(original);
  });

  it("round-trips null values", () => {
    const original = { value: null };
    const result = deserializeMetadata(
      serializeMetadata(original as Record<string, unknown>)
    );
    expect(result).toEqual(original);
  });

  it("round-trips nested objects", () => {
    const original = { nested: { a: 1, b: "two", c: [1, 2] } };
    const result = deserializeMetadata(
      serializeMetadata(original as Record<string, unknown>)
    );
    expect(result).toEqual(original);
  });

  it("round-trips number arrays", () => {
    const original = { scores: [1, 2, 3] };
    const result = deserializeMetadata(
      serializeMetadata(original as Record<string, unknown>)
    );
    expect(result).toEqual(original);
  });

  it("round-trips complex mixed metadata", () => {
    const original = {
      name: "GPU Test",
      price: 499,
      available: true,
      tags: ["gpu", "nvidia"],
      specs: { cores: 8192, vram: "16GB" },
      ratings: [4.5, 4.8, 4.2],
      deprecated: null,
    };
    const result = deserializeMetadata(
      serializeMetadata(original as Record<string, unknown>)
    );
    expect(result).toEqual(original);
  });

  it("round-trips unicode metadata", () => {
    const original = {
      title: "日本語テスト 🎯",
      description: "Ñoño café résumé naïve",
      emoji: "🚀🔥💯",
    };
    const result = deserializeMetadata(serializeMetadata(original));
    expect(result).toEqual(original);
  });

  it("round-trips empty metadata", () => {
    const original = {};
    const result = deserializeMetadata(serializeMetadata(original));
    expect(result).toEqual(original);
  });

  // [M4 FIX] Document that undefined is coerced to null on round-trip
  it("coerces undefined to null on round-trip (documented behavior)", () => {
    const original = { value: undefined };
    const result = deserializeMetadata(
      serializeMetadata(original as Record<string, unknown>)
    );
    // undefined -> null is an intentional coercion (LangChain metadata is Record<string, any>)
    expect(result.value).toBeNull();
    expect(result.value).not.toBeUndefined();
  });

  // [C1 FIX] NaN round-trips via JSON serialization
  it("round-trips NaN through serialization (restored as null via JSON)", () => {
    const original = { value: NaN };
    const result = deserializeMetadata(
      serializeMetadata(original as Record<string, unknown>)
    );
    // NaN -> JSON.stringify -> "null" -> JSON.parse -> null
    expect(result.value).toBeNull();
  });

  // [M3 FIX] DAG round-trips correctly
  it("round-trips DAG structure (shared refs) correctly", () => {
    const shared = { x: 1 };
    const original = { a: shared, b: shared };
    const result = deserializeMetadata(
      serializeMetadata(original as Record<string, unknown>)
    );
    expect(result.a).toEqual({ x: 1 });
    expect(result.b).toEqual({ x: 1 });
  });
});
