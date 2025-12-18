# Week 23 Day 5: TypeScript Wrapper

**Date:** Day 5 of Week 23
**Focus:** TypeScript API for Filter System
**Agent:** WASM_SPECIALIST
**Total Hours:** 12h
**Status:** [PLANNED]

---

## Executive Summary

Day 5 creates the TypeScript wrapper layer that provides an ergonomic API for JavaScript/TypeScript developers. This includes the `Filter` static class, `FilterBuilder` fluent API, and `EdgeVecIndex.searchFiltered()` method.

**Prerequisites:**
- W23.4.1-W23.4.4 (all WASM bindings) COMPLETE
- `wasm-pack build --target web` succeeds
- FILTERING_WASM_API.md specification approved

---

## Tasks Overview

| Task ID | Description | Hours | Priority |
|:--------|:------------|:------|:---------|
| W23.5.1 | Filter static class (eq, ne, lt, etc.) | 3h | P0 |
| W23.5.2 | FilterBuilder fluent API | 4h | P0 |
| W23.5.3 | EdgeVecIndex.searchFiltered() method | 3h | P0 |
| W23.5.4 | TypeScript type definitions (.d.ts) | 2h | P0 |

---

## Task W23.5.1: Filter Static Class

### Description
Implement the `Filter` static class with factory methods for creating filter expressions.

### Hours: 3h

### Specification

**File:** `pkg/filter.ts`

```typescript
/**
 * Filter factory for creating filter expressions.
 *
 * Provides two ways to create filters:
 * 1. String parsing: Filter.parse('category = "gpu"')
 * 2. Builder methods: Filter.eq('category', 'gpu')
 *
 * @example
 * // String syntax (familiar to SQL users)
 * const f1 = Filter.parse('price < 500 AND category = "gpu"');
 *
 * // Builder syntax (type-safe)
 * const f2 = Filter.and(
 *   Filter.eq('category', 'gpu'),
 *   Filter.lt('price', 500)
 * );
 */

import { parse_filter_js, validate_filter_js, try_parse_filter_js, get_filter_info_js } from './edgevec_bg.wasm';

// Re-export types
export type MetadataValue = string | number | boolean | string[];

/**
 * Compiled filter expression.
 *
 * Opaque type - use Filter factory methods to create.
 */
export interface FilterExpression {
  /** Internal JSON representation */
  readonly _json: string;

  /** Convert to string representation */
  toString(): string;

  /** Serialize to JSON for debugging */
  toJSON(): object;

  /** Check if filter is always true */
  readonly isTautology: boolean;

  /** Check if filter is always false */
  readonly isContradiction: boolean;

  /** Estimated complexity (1-10) */
  readonly complexity: number;
}

/**
 * Internal filter implementation.
 */
class FilterImpl implements FilterExpression {
  readonly _json: string;
  private _parsed: object | null = null;
  private _info: FilterInfo | null = null;

  constructor(json: string) {
    this._json = json;
  }

  toString(): string {
    // Convert JSON AST back to string syntax
    return this._reconstructString(this.toJSON());
  }

  toJSON(): object {
    if (!this._parsed) {
      this._parsed = JSON.parse(this._json);
    }
    return this._parsed!;
  }

  get isTautology(): boolean {
    const obj = this.toJSON() as any;
    return obj.op === 'literal_bool' && obj.value === true;
  }

  get isContradiction(): boolean {
    const obj = this.toJSON() as any;
    return obj.op === 'literal_bool' && obj.value === false;
  }

  get complexity(): number {
    return this._getInfo().complexity;
  }

  private _getInfo(): FilterInfo {
    if (!this._info) {
      const infoJson = get_filter_info_js(this.toString());
      this._info = JSON.parse(infoJson);
    }
    return this._info!;
  }

  private _reconstructString(ast: any): string {
    // Recursively convert AST to string syntax
    switch (ast.op) {
      case 'field':
        return ast.name || ast.value;
      case 'literal_string':
        return `"${this._escapeString(ast.value)}"`;
      case 'literal_int':
      case 'literal_float':
        return String(ast.value);
      case 'literal_bool':
        return ast.value ? 'true' : 'false';
      case 'eq':
        return `${this._reconstructString(ast.left)} = ${this._reconstructString(ast.right)}`;
      case 'ne':
        return `${this._reconstructString(ast.left)} != ${this._reconstructString(ast.right)}`;
      case 'lt':
        return `${this._reconstructString(ast.left)} < ${this._reconstructString(ast.right)}`;
      case 'le':
        return `${this._reconstructString(ast.left)} <= ${this._reconstructString(ast.right)}`;
      case 'gt':
        return `${this._reconstructString(ast.left)} > ${this._reconstructString(ast.right)}`;
      case 'ge':
        return `${this._reconstructString(ast.left)} >= ${this._reconstructString(ast.right)}`;
      case 'and':
        return `(${this._reconstructString(ast.left)} AND ${this._reconstructString(ast.right)})`;
      case 'or':
        return `(${this._reconstructString(ast.left)} OR ${this._reconstructString(ast.right)})`;
      case 'not':
        return `NOT ${this._reconstructString(ast.inner)}`;
      case 'contains':
        return `${this._reconstructString(ast.left)} CONTAINS ${this._reconstructString(ast.right)}`;
      case 'starts_with':
        return `${this._reconstructString(ast.left)} STARTS_WITH ${this._reconstructString(ast.right)}`;
      case 'ends_with':
        return `${this._reconstructString(ast.left)} ENDS_WITH ${this._reconstructString(ast.right)}`;
      case 'like':
        return `${this._reconstructString(ast.left)} LIKE ${this._reconstructString(ast.right)}`;
      case 'between':
        return `${this._reconstructString(ast.field)} BETWEEN ${this._reconstructString(ast.low)} ${this._reconstructString(ast.high)}`;
      case 'in':
        return `${this._reconstructString(ast.field)} IN ${this._reconstructArray(ast.values)}`;
      case 'not_in':
        return `${this._reconstructString(ast.field)} NOT IN ${this._reconstructArray(ast.values)}`;
      case 'any':
        return `ANY(${this._reconstructString(ast.field)}, ${this._reconstructString(ast.value)})`;
      case 'all':
        return `ALL(${this._reconstructString(ast.field)}, ${this._reconstructArray(ast.values)})`;
      case 'none_of':
        return `NONE(${this._reconstructString(ast.field)}, ${this._reconstructArray(ast.values)})`;
      case 'is_null':
        return `${this._reconstructString(ast.field)} IS NULL`;
      case 'is_not_null':
        return `${this._reconstructString(ast.field)} IS NOT NULL`;
      default:
        return JSON.stringify(ast);
    }
  }

  private _escapeString(s: string): string {
    return s.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
  }

  private _reconstructArray(arr: any[]): string {
    return '[' + arr.map(v => this._reconstructString(v)).join(', ') + ']';
  }
}

interface FilterInfo {
  nodeCount: number;
  depth: number;
  fields: string[];
  operators: string[];
  complexity: number;
}

/**
 * Filter validation result.
 */
export interface FilterValidation {
  valid: boolean;
  errors: FilterValidationError[];
  warnings: FilterValidationWarning[];
  filter?: FilterExpression;
}

export interface FilterValidationError {
  code: string;
  message: string;
  position?: { line: number; column: number; offset: number };
  suggestion?: string;
}

export interface FilterValidationWarning {
  code: string;
  message: string;
  position?: { line: number; column: number; offset: number };
}

/**
 * Filter factory and static methods.
 */
export const Filter = {
  // ═══════════════════════════════════════════════════════════════════════════
  // PARSING
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Parse a filter string into a compiled filter.
   *
   * @param query - Filter string in EdgeVec syntax
   * @returns Compiled filter expression
   * @throws FilterException on syntax error
   *
   * @example
   * const filter = Filter.parse('category = "gpu" AND price < 1000');
   */
  parse(query: string): FilterExpression {
    const json = parse_filter_js(query);
    return new FilterImpl(json);
  },

  /**
   * Try to parse a filter string, returning null on error.
   *
   * @param query - Filter string to parse
   * @returns Compiled filter or null
   *
   * @example
   * const filter = Filter.tryParse(userInput);
   * if (filter) { /* valid */ }
   */
  tryParse(query: string): FilterExpression | null {
    const result = try_parse_filter_js(query);
    if (result === null) return null;
    return new FilterImpl(result as string);
  },

  /**
   * Validate a filter string without compiling.
   *
   * @param query - Filter string to validate
   * @returns Validation result
   *
   * @example
   * const result = Filter.validate('price <');
   * if (!result.valid) console.log(result.errors);
   */
  validate(query: string): FilterValidation {
    const json = validate_filter_js(query);
    const result = JSON.parse(json);
    return {
      valid: result.valid,
      errors: result.errors || [],
      warnings: result.warnings || [],
      filter: result.filter ? new FilterImpl(JSON.stringify(result.filter)) : undefined,
    };
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // COMPARISON OPERATORS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Create equality filter: field = value
   *
   * @example Filter.eq('category', 'gpu')
   */
  eq(field: string, value: MetadataValue): FilterExpression {
    return Filter.parse(`${field} = ${Filter._valueToString(value)}`);
  },

  /**
   * Create inequality filter: field != value
   *
   * @example Filter.ne('status', 'deleted')
   */
  ne(field: string, value: MetadataValue): FilterExpression {
    return Filter.parse(`${field} != ${Filter._valueToString(value)}`);
  },

  /**
   * Create less-than filter: field < value
   *
   * @example Filter.lt('price', 1000)
   */
  lt(field: string, value: number): FilterExpression {
    return Filter.parse(`${field} < ${value}`);
  },

  /**
   * Create less-than-or-equal filter: field <= value
   *
   * @example Filter.le('price', 1000)
   */
  le(field: string, value: number): FilterExpression {
    return Filter.parse(`${field} <= ${value}`);
  },

  /**
   * Create greater-than filter: field > value
   *
   * @example Filter.gt('rating', 4.0)
   */
  gt(field: string, value: number): FilterExpression {
    return Filter.parse(`${field} > ${value}`);
  },

  /**
   * Create greater-than-or-equal filter: field >= value
   *
   * @example Filter.ge('stock', 10)
   */
  ge(field: string, value: number): FilterExpression {
    return Filter.parse(`${field} >= ${value}`);
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // RANGE OPERATORS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Create range filter: low <= field <= high
   *
   * @example Filter.between('price', 100, 500)
   */
  between(field: string, low: number, high: number): FilterExpression {
    return Filter.parse(`${field} BETWEEN ${low} ${high}`);
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // STRING OPERATORS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Create contains filter: field CONTAINS substring
   *
   * @example Filter.contains('title', 'NVIDIA')
   */
  contains(field: string, substring: string): FilterExpression {
    return Filter.parse(`${field} CONTAINS "${Filter._escapeString(substring)}"`);
  },

  /**
   * Create starts-with filter: field STARTS_WITH prefix
   *
   * @example Filter.startsWith('sku', 'GPU-')
   */
  startsWith(field: string, prefix: string): FilterExpression {
    return Filter.parse(`${field} STARTS_WITH "${Filter._escapeString(prefix)}"`);
  },

  /**
   * Create ends-with filter: field ENDS_WITH suffix
   *
   * @example Filter.endsWith('filename', '.pdf')
   */
  endsWith(field: string, suffix: string): FilterExpression {
    return Filter.parse(`${field} ENDS_WITH "${Filter._escapeString(suffix)}"`);
  },

  /**
   * Create pattern match filter: field LIKE pattern
   *
   * @example Filter.like('email', '%@company.com')
   */
  like(field: string, pattern: string): FilterExpression {
    return Filter.parse(`${field} LIKE "${Filter._escapeString(pattern)}"`);
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // SET OPERATORS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Create IN filter: field IN [values]
   *
   * @example Filter.in('category', ['gpu', 'cpu', 'ram'])
   */
  in(field: string, values: MetadataValue[]): FilterExpression {
    return Filter.parse(`${field} IN ${Filter._arrayToString(values)}`);
  },

  /**
   * Create NOT IN filter: field NOT IN [values]
   *
   * @example Filter.notIn('status', ['deleted', 'archived'])
   */
  notIn(field: string, values: MetadataValue[]): FilterExpression {
    return Filter.parse(`${field} NOT IN ${Filter._arrayToString(values)}`);
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // ARRAY OPERATORS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Create ANY filter: ANY(field, value)
   *
   * @example Filter.any('tags', 'nvidia')
   */
  any(field: string, value: MetadataValue): FilterExpression {
    return Filter.parse(`ANY(${field}, ${Filter._valueToString(value)})`);
  },

  /**
   * Create ALL filter: ALL(field, values)
   *
   * @example Filter.all('tags', ['gpu', 'gaming'])
   */
  all(field: string, values: MetadataValue[]): FilterExpression {
    return Filter.parse(`ALL(${field}, ${Filter._arrayToString(values)})`);
  },

  /**
   * Create NONE filter: NONE(field, values)
   *
   * @example Filter.none('tags', ['nsfw', 'spam'])
   */
  none(field: string, values: MetadataValue[]): FilterExpression {
    return Filter.parse(`NONE(${field}, ${Filter._arrayToString(values)})`);
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // NULL OPERATORS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Create IS NULL filter
   *
   * @example Filter.isNull('deletedAt')
   */
  isNull(field: string): FilterExpression {
    return Filter.parse(`${field} IS NULL`);
  },

  /**
   * Create IS NOT NULL filter
   *
   * @example Filter.isNotNull('verifiedAt')
   */
  isNotNull(field: string): FilterExpression {
    return Filter.parse(`${field} IS NOT NULL`);
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // LOGICAL OPERATORS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Create AND combination of filters
   *
   * @example Filter.and(Filter.eq('a', 1), Filter.eq('b', 2))
   */
  and(...filters: FilterExpression[]): FilterExpression {
    if (filters.length === 0) {
      throw new Error('Filter.and() requires at least one filter');
    }
    if (filters.length === 1) {
      return filters[0];
    }

    const combined = filters.map(f => `(${f.toString()})`).join(' AND ');
    return Filter.parse(combined);
  },

  /**
   * Create OR combination of filters
   *
   * @example Filter.or(Filter.eq('a', 1), Filter.eq('b', 2))
   */
  or(...filters: FilterExpression[]): FilterExpression {
    if (filters.length === 0) {
      throw new Error('Filter.or() requires at least one filter');
    }
    if (filters.length === 1) {
      return filters[0];
    }

    const combined = filters.map(f => `(${f.toString()})`).join(' OR ');
    return Filter.parse(combined);
  },

  /**
   * Create NOT (negation) of a filter
   *
   * @example Filter.not(Filter.eq('status', 'deleted'))
   */
  not(filter: FilterExpression): FilterExpression {
    return Filter.parse(`NOT (${filter.toString()})`);
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // SPECIAL FILTERS
  // ═══════════════════════════════════════════════════════════════════════════

  /** Filter that matches all vectors (no filtering) */
  get all(): FilterExpression {
    return new FilterImpl('{"op":"literal_bool","value":true}');
  },

  /** Filter that matches no vectors (empty result) */
  get nothing(): FilterExpression {
    return new FilterImpl('{"op":"literal_bool","value":false}');
  },

  // ═══════════════════════════════════════════════════════════════════════════
  // INTERNAL HELPERS
  // ═══════════════════════════════════════════════════════════════════════════

  _valueToString(value: MetadataValue): string {
    if (typeof value === 'string') {
      return `"${Filter._escapeString(value)}"`;
    }
    if (typeof value === 'boolean') {
      return value ? 'true' : 'false';
    }
    if (Array.isArray(value)) {
      return Filter._arrayToString(value);
    }
    return String(value);
  },

  _arrayToString(values: MetadataValue[]): string {
    const items = values.map(v => Filter._valueToString(v)).join(', ');
    return `[${items}]`;
  },

  _escapeString(s: string): string {
    return s.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
  },
};
```

### Acceptance Criteria
- [ ] All comparison methods work (eq, ne, lt, le, gt, ge)
- [ ] Range method works (between)
- [ ] String methods work (contains, startsWith, endsWith, like)
- [ ] Set methods work (in, notIn)
- [ ] Array methods work (any, all, none)
- [ ] NULL methods work (isNull, isNotNull)
- [ ] Logical methods work (and, or, not)
- [ ] Special filters work (all, nothing)
- [ ] Parse and tryParse work
- [ ] TypeScript compiles without errors

### Test Cases (Jest)
```typescript
import { Filter } from './filter';

describe('Filter', () => {
  describe('parse', () => {
    it('parses simple equality', () => {
      const f = Filter.parse('x = 1');
      expect(f.toString()).toContain('x');
      expect(f.toString()).toContain('1');
    });

    it('throws on invalid input', () => {
      expect(() => Filter.parse('>>>')).toThrow();
    });
  });

  describe('tryParse', () => {
    it('returns filter on valid input', () => {
      const f = Filter.tryParse('x = 1');
      expect(f).not.toBeNull();
    });

    it('returns null on invalid input', () => {
      const f = Filter.tryParse('>>>');
      expect(f).toBeNull();
    });
  });

  describe('eq', () => {
    it('creates equality filter for string', () => {
      const f = Filter.eq('category', 'gpu');
      expect(f.toString()).toBe('category = "gpu"');
    });

    it('creates equality filter for number', () => {
      const f = Filter.eq('count', 42);
      expect(f.toString()).toBe('count = 42');
    });
  });

  describe('and', () => {
    it('combines two filters', () => {
      const f = Filter.and(
        Filter.eq('a', 1),
        Filter.eq('b', 2)
      );
      expect(f.toString()).toContain('AND');
    });

    it('handles single filter', () => {
      const f = Filter.and(Filter.eq('a', 1));
      expect(f.toString()).toContain('a');
    });
  });

  describe('validate', () => {
    it('returns valid for correct filter', () => {
      const result = Filter.validate('x = 1');
      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(0);
    });

    it('returns errors for invalid filter', () => {
      const result = Filter.validate('x <');
      expect(result.valid).toBe(false);
      expect(result.errors.length).toBeGreaterThan(0);
    });
  });
});
```

---

## Task W23.5.2: FilterBuilder Fluent API

### Description
Implement the fluent `FilterBuilder` class for building complex filters programmatically.

### Hours: 4h

### Specification

**File:** `pkg/filter-builder.ts`

```typescript
import { Filter, FilterExpression, MetadataValue } from './filter';

/**
 * Fluent builder for complex filters.
 *
 * @example
 * const filter = new FilterBuilder()
 *   .where('category').eq('gpu')
 *   .and('price').lt(1000)
 *   .or('featured').eq(true)
 *   .build();
 */
export class FilterBuilder {
  private parts: FilterPart[] = [];
  private currentField: string | null = null;
  private pendingLogic: 'and' | 'or' | null = null;

  /**
   * Start a new condition on a field.
   *
   * @example builder.where('category').eq('gpu')
   */
  where(field: string): FieldCondition {
    this.currentField = field;
    return new FieldCondition(this, field);
  }

  /**
   * Add an AND condition.
   *
   * @example builder.where('a').eq(1).and('b').lt(10)
   */
  and(field: string): FieldCondition {
    this.pendingLogic = 'and';
    this.currentField = field;
    return new FieldCondition(this, field);
  }

  /**
   * Add an OR condition.
   *
   * @example builder.where('a').eq(1).or('b').eq(2)
   */
  or(field: string): FieldCondition {
    this.pendingLogic = 'or';
    this.currentField = field;
    return new FieldCondition(this, field);
  }

  /**
   * Start a grouped sub-expression.
   *
   * @example builder.where('a').eq(1).andGroup(b => b.where('x').eq(1).or('y').eq(2))
   */
  andGroup(builderFn: (b: FilterBuilder) => FilterBuilder): FilterBuilder {
    const subBuilder = new FilterBuilder();
    builderFn(subBuilder);
    const subFilter = subBuilder.build();

    this.addPart({
      filter: subFilter,
      logic: 'and',
      grouped: true,
    });

    return this;
  }

  /**
   * Start an OR grouped sub-expression.
   */
  orGroup(builderFn: (b: FilterBuilder) => FilterBuilder): FilterBuilder {
    const subBuilder = new FilterBuilder();
    builderFn(subBuilder);
    const subFilter = subBuilder.build();

    this.addPart({
      filter: subFilter,
      logic: 'or',
      grouped: true,
    });

    return this;
  }

  /**
   * Build the final filter expression.
   *
   * @throws Error if builder is empty
   */
  build(): FilterExpression {
    if (this.parts.length === 0) {
      throw new Error('FilterBuilder: no conditions specified');
    }

    if (this.parts.length === 1) {
      return this.parts[0].filter;
    }

    // Build up the expression respecting logic operators
    let result = this.parts[0].filter;

    for (let i = 1; i < this.parts.length; i++) {
      const part = this.parts[i];
      const partExpr = part.grouped
        ? Filter.parse(`(${part.filter.toString()})`)
        : part.filter;

      if (part.logic === 'and') {
        result = Filter.and(result, partExpr);
      } else {
        result = Filter.or(result, partExpr);
      }
    }

    return result;
  }

  /**
   * Get string representation of current state.
   */
  toString(): string {
    try {
      return this.build().toString();
    } catch {
      return '<incomplete filter>';
    }
  }

  /** @internal */
  addPart(part: FilterPart): void {
    // First part doesn't have logic
    if (this.parts.length === 0) {
      part.logic = null;
    } else if (!part.logic && this.pendingLogic) {
      part.logic = this.pendingLogic;
    }

    this.parts.push(part);
    this.pendingLogic = null;
  }
}

interface FilterPart {
  filter: FilterExpression;
  logic: 'and' | 'or' | null;
  grouped?: boolean;
}

/**
 * Field condition builder (returned by where/and/or).
 */
export class FieldCondition {
  constructor(
    private builder: FilterBuilder,
    private field: string
  ) {}

  /** Equal to */
  eq(value: MetadataValue): FilterBuilder {
    this.builder.addPart({
      filter: Filter.eq(this.field, value),
      logic: null,
    });
    return this.builder;
  }

  /** Not equal to */
  ne(value: MetadataValue): FilterBuilder {
    this.builder.addPart({
      filter: Filter.ne(this.field, value),
      logic: null,
    });
    return this.builder;
  }

  /** Less than */
  lt(value: number): FilterBuilder {
    this.builder.addPart({
      filter: Filter.lt(this.field, value),
      logic: null,
    });
    return this.builder;
  }

  /** Less than or equal */
  le(value: number): FilterBuilder {
    this.builder.addPart({
      filter: Filter.le(this.field, value),
      logic: null,
    });
    return this.builder;
  }

  /** Greater than */
  gt(value: number): FilterBuilder {
    this.builder.addPart({
      filter: Filter.gt(this.field, value),
      logic: null,
    });
    return this.builder;
  }

  /** Greater than or equal */
  ge(value: number): FilterBuilder {
    this.builder.addPart({
      filter: Filter.ge(this.field, value),
      logic: null,
    });
    return this.builder;
  }

  /** Between (inclusive) */
  between(low: number, high: number): FilterBuilder {
    this.builder.addPart({
      filter: Filter.between(this.field, low, high),
      logic: null,
    });
    return this.builder;
  }

  /** Contains substring */
  contains(substring: string): FilterBuilder {
    this.builder.addPart({
      filter: Filter.contains(this.field, substring),
      logic: null,
    });
    return this.builder;
  }

  /** Starts with prefix */
  startsWith(prefix: string): FilterBuilder {
    this.builder.addPart({
      filter: Filter.startsWith(this.field, prefix),
      logic: null,
    });
    return this.builder;
  }

  /** Ends with suffix */
  endsWith(suffix: string): FilterBuilder {
    this.builder.addPart({
      filter: Filter.endsWith(this.field, suffix),
      logic: null,
    });
    return this.builder;
  }

  /** LIKE pattern match */
  like(pattern: string): FilterBuilder {
    this.builder.addPart({
      filter: Filter.like(this.field, pattern),
      logic: null,
    });
    return this.builder;
  }

  /** In array of values */
  in(values: MetadataValue[]): FilterBuilder {
    this.builder.addPart({
      filter: Filter.in(this.field, values),
      logic: null,
    });
    return this.builder;
  }

  /** Not in array of values */
  notIn(values: MetadataValue[]): FilterBuilder {
    this.builder.addPart({
      filter: Filter.notIn(this.field, values),
      logic: null,
    });
    return this.builder;
  }

  /** Is null */
  isNull(): FilterBuilder {
    this.builder.addPart({
      filter: Filter.isNull(this.field),
      logic: null,
    });
    return this.builder;
  }

  /** Is not null */
  isNotNull(): FilterBuilder {
    this.builder.addPart({
      filter: Filter.isNotNull(this.field),
      logic: null,
    });
    return this.builder;
  }
}

// Export for convenience
export { Filter, FilterExpression, MetadataValue } from './filter';
```

### Acceptance Criteria
- [ ] Fluent API chains correctly
- [ ] where/and/or work as expected
- [ ] All operators available (eq, lt, contains, etc.)
- [ ] Grouped expressions work with andGroup/orGroup
- [ ] build() produces correct filter
- [ ] toString() shows current state
- [ ] TypeScript types are correct

### Test Cases (Jest)
```typescript
import { FilterBuilder } from './filter-builder';

describe('FilterBuilder', () => {
  it('builds simple equality', () => {
    const filter = new FilterBuilder()
      .where('category').eq('gpu')
      .build();

    expect(filter.toString()).toContain('category');
    expect(filter.toString()).toContain('gpu');
  });

  it('builds AND condition', () => {
    const filter = new FilterBuilder()
      .where('category').eq('gpu')
      .and('price').lt(1000)
      .build();

    expect(filter.toString()).toContain('AND');
  });

  it('builds OR condition', () => {
    const filter = new FilterBuilder()
      .where('category').eq('gpu')
      .or('category').eq('cpu')
      .build();

    expect(filter.toString()).toContain('OR');
  });

  it('builds complex nested expression', () => {
    const filter = new FilterBuilder()
      .where('category').eq('electronics')
      .andGroup(b => b
        .where('price').lt(500)
        .or('featured').eq(true)
      )
      .build();

    const str = filter.toString();
    expect(str).toContain('electronics');
    expect(str).toContain('500');
    expect(str).toContain('featured');
  });

  it('throws on empty build', () => {
    expect(() => new FilterBuilder().build()).toThrow();
  });

  it('handles between', () => {
    const filter = new FilterBuilder()
      .where('price').between(100, 500)
      .build();

    expect(filter.toString()).toContain('BETWEEN');
  });
});
```

---

## Task W23.5.3: EdgeVecIndex.searchFiltered() Method

### Description
Add the `searchFiltered()` method to the EdgeVecIndex TypeScript class.

### Hours: 3h

### Specification

**File:** `pkg/edgevec.ts` (modifications to existing class)

```typescript
import { search_filtered_js } from './edgevec_bg.wasm';
import { Filter, FilterExpression } from './filter';

/**
 * Filter strategy for search.
 */
export type FilterStrategy = 'auto' | 'pre' | 'post' | 'hybrid';

/**
 * Options for filtered search.
 */
export interface SearchOptions {
  /**
   * Filter expression (string or Filter object).
   */
  filter?: string | FilterExpression;

  /**
   * Filter strategy.
   * @default 'auto'
   */
  strategy?: FilterStrategy;

  /**
   * Oversample factor for post/hybrid strategy.
   * @default 3.0
   */
  oversampleFactor?: number;

  /**
   * Include metadata in results.
   * @default false
   */
  includeMetadata?: boolean;

  /**
   * Include vectors in results.
   * @default false
   */
  includeVectors?: boolean;

  /**
   * Override ef_search for this query.
   */
  efSearch?: number;
}

/**
 * Single search result.
 */
export interface SearchResult {
  /** Vector ID */
  id: number;
  /** Similarity score */
  score: number;
  /** Metadata (if includeMetadata=true) */
  metadata?: Record<string, any>;
  /** Vector data (if includeVectors=true) */
  vector?: Float32Array;
}

/**
 * Filtered search result with diagnostics.
 */
export interface FilteredSearchResult {
  /** Search results */
  results: SearchResult[];
  /** Whether k results were found */
  complete: boolean;
  /** Observed selectivity */
  observedSelectivity: number;
  /** Strategy used */
  strategyUsed: FilterStrategy;
  /** Vectors evaluated */
  vectorsEvaluated: number;
  /** Filter evaluation time (ms) */
  filterTimeMs: number;
  /** Total search time (ms) */
  totalTimeMs: number;
}

/**
 * EdgeVec index (existing class, add these methods).
 */
export class EdgeVecIndex {
  private ptr: number;

  // ... existing methods ...

  /**
   * Search with optional filter.
   *
   * @param query - Query vector
   * @param k - Number of results
   * @param options - Search options (filter, strategy, etc.)
   * @returns Search results
   *
   * @example
   * const results = await index.search(query, 10, {
   *   filter: 'category = "gpu" AND price < 500'
   * });
   */
  async search(
    query: Float32Array | number[],
    k: number,
    options?: SearchOptions
  ): Promise<SearchResult[]> {
    const result = await this.searchFiltered(query, k, options);
    return result.results;
  }

  /**
   * Search with filter and full diagnostics.
   *
   * @param query - Query vector
   * @param k - Number of results
   * @param options - Search options
   * @returns Filtered search result with diagnostics
   *
   * @example
   * const result = await index.searchFiltered(query, 10, {
   *   filter: Filter.eq('category', 'gpu'),
   *   strategy: 'auto'
   * });
   * console.log('Strategy used:', result.strategyUsed);
   * console.log('Selectivity:', result.observedSelectivity);
   */
  async searchFiltered(
    query: Float32Array | number[],
    k: number,
    options?: SearchOptions
  ): Promise<FilteredSearchResult> {
    // Convert query to JSON array
    const queryArray = Array.from(query);
    const queryJson = JSON.stringify(queryArray);

    // Build options JSON
    const optionsJson = this.buildOptionsJson(options);

    try {
      // Call WASM
      const resultJson = search_filtered_js(
        this.ptr,
        queryJson,
        k,
        optionsJson
      );

      // Parse result
      const result = JSON.parse(resultJson);

      // Convert results
      return {
        results: result.results.map((r: any) => ({
          id: r.id,
          score: r.score,
          metadata: r.metadata ?? undefined,
          vector: r.vector ? new Float32Array(r.vector) : undefined,
        })),
        complete: result.complete,
        observedSelectivity: result.observedSelectivity,
        strategyUsed: result.strategyUsed as FilterStrategy,
        vectorsEvaluated: result.vectorsEvaluated,
        filterTimeMs: result.filterTimeMs,
        totalTimeMs: result.totalTimeMs,
      };
    } catch (e) {
      throw this.wrapError(e);
    }
  }

  /**
   * Count vectors matching a filter.
   *
   * @param filter - Filter expression
   * @returns Count of matching vectors
   */
  async count(filter?: string | FilterExpression): Promise<number> {
    if (!filter) {
      return this.size;
    }

    // Use pre-filter strategy to count all matches
    const result = await this.searchFiltered(
      new Float32Array(this.dimensions).fill(0),
      this.size,
      { filter, strategy: 'pre' }
    );

    return result.results.length;
  }

  private buildOptionsJson(options?: SearchOptions): string {
    if (!options) {
      return '{}';
    }

    const opts: any = {};

    // Handle filter (string or FilterExpression)
    if (options.filter) {
      if (typeof options.filter === 'string') {
        opts.filter = options.filter;
      } else {
        opts.filter = options.filter.toString();
      }
    }

    if (options.strategy) {
      opts.strategy = options.strategy;
    }

    if (options.oversampleFactor !== undefined) {
      opts.oversampleFactor = options.oversampleFactor;
    }

    if (options.includeMetadata !== undefined) {
      opts.includeMetadata = options.includeMetadata;
    }

    if (options.includeVectors !== undefined) {
      opts.includeVectors = options.includeVectors;
    }

    if (options.efSearch !== undefined) {
      opts.efSearch = options.efSearch;
    }

    return JSON.stringify(opts);
  }

  private wrapError(e: unknown): Error {
    if (typeof e === 'string') {
      try {
        const parsed = JSON.parse(e);
        const error = new FilterException(
          parsed.code,
          parsed.message,
          parsed.position,
          parsed.suggestion
        );
        return error;
      } catch {
        return new Error(e);
      }
    }
    return e instanceof Error ? e : new Error(String(e));
  }
}

/**
 * Filter exception with rich error information.
 */
export class FilterException extends Error {
  readonly code: string;
  readonly position?: { line: number; column: number; offset: number };
  readonly suggestion?: string;
  readonly filterString?: string;

  constructor(
    code: string,
    message: string,
    position?: { line: number; column: number; offset: number },
    suggestion?: string
  ) {
    super(message);
    this.name = 'FilterException';
    this.code = code;
    this.position = position;
    this.suggestion = suggestion;
  }

  /**
   * Format error with source context.
   */
  format(): string {
    let output = `FilterException [${this.code}]: ${this.message}`;

    if (this.position) {
      output += `\n  at line ${this.position.line}, column ${this.position.column}`;
    }

    if (this.suggestion) {
      output += `\n  Suggestion: ${this.suggestion}`;
    }

    return output;
  }
}
```

### Acceptance Criteria
- [ ] search() accepts filter option
- [ ] searchFiltered() returns full diagnostics
- [ ] String and Filter object filters work
- [ ] All strategies work (auto, pre, post, hybrid)
- [ ] includeMetadata and includeVectors work
- [ ] FilterException provides rich error info
- [ ] count() with filter works

### Test Cases (Jest)
```typescript
import { EdgeVecIndex, Filter, FilterException } from './edgevec';

describe('EdgeVecIndex.searchFiltered', () => {
  let index: EdgeVecIndex;

  beforeEach(async () => {
    index = new EdgeVecIndex({ dimensions: 384 });
    // Add test vectors with metadata
    for (let i = 0; i < 100; i++) {
      await index.add(randomVector(384), {
        category: i % 2 === 0 ? 'gpu' : 'cpu',
        price: i * 10,
      });
    }
  });

  it('searches with string filter', async () => {
    const result = await index.searchFiltered(
      randomVector(384),
      10,
      { filter: 'category = "gpu"' }
    );

    expect(result.results.length).toBeLessThanOrEqual(10);
    expect(result.complete).toBeDefined();
    expect(result.strategyUsed).toBeDefined();
  });

  it('searches with Filter object', async () => {
    const filter = Filter.eq('category', 'gpu');
    const result = await index.searchFiltered(
      randomVector(384),
      10,
      { filter }
    );

    expect(result.results.length).toBeGreaterThan(0);
  });

  it('includes metadata when requested', async () => {
    const result = await index.searchFiltered(
      randomVector(384),
      10,
      { includeMetadata: true }
    );

    for (const r of result.results) {
      expect(r.metadata).toBeDefined();
    }
  });

  it('respects strategy override', async () => {
    const result = await index.searchFiltered(
      randomVector(384),
      10,
      { filter: 'category = "gpu"', strategy: 'pre' }
    );

    expect(result.strategyUsed).toBe('pre');
  });

  it('throws FilterException on invalid filter', async () => {
    await expect(
      index.searchFiltered(randomVector(384), 10, { filter: '>>>' })
    ).rejects.toBeInstanceOf(FilterException);
  });
});
```

---

## Task W23.5.4: TypeScript Type Definitions

### Description
Create comprehensive TypeScript type definitions file.

### Hours: 2h

### Specification

**File:** `pkg/edgevec.d.ts`

```typescript
// EdgeVec TypeScript Type Definitions
// Version: 0.5.0

// ═══════════════════════════════════════════════════════════════════════════
// METADATA TYPES
// ═══════════════════════════════════════════════════════════════════════════

/** Supported metadata value types */
export type MetadataValue = string | number | boolean | string[];

/** Metadata record for a vector */
export type Metadata = Record<string, MetadataValue>;

// ═══════════════════════════════════════════════════════════════════════════
// FILTER TYPES
// ═══════════════════════════════════════════════════════════════════════════

/** Compiled filter expression */
export interface FilterExpression {
  readonly _json: string;
  toString(): string;
  toJSON(): object;
  readonly isTautology: boolean;
  readonly isContradiction: boolean;
  readonly complexity: number;
}

/** Filter validation result */
export interface FilterValidation {
  valid: boolean;
  errors: FilterValidationError[];
  warnings: FilterValidationWarning[];
  filter?: FilterExpression;
}

/** Filter validation error */
export interface FilterValidationError {
  code: string;
  message: string;
  position?: SourcePosition;
  suggestion?: string;
}

/** Filter validation warning */
export interface FilterValidationWarning {
  code: string;
  message: string;
  position?: SourcePosition;
}

/** Source position in filter string */
export interface SourcePosition {
  line: number;
  column: number;
  offset: number;
}

/** Filter factory */
export interface FilterStatic {
  // Parsing
  parse(query: string): FilterExpression;
  tryParse(query: string): FilterExpression | null;
  validate(query: string): FilterValidation;

  // Comparison
  eq(field: string, value: MetadataValue): FilterExpression;
  ne(field: string, value: MetadataValue): FilterExpression;
  lt(field: string, value: number): FilterExpression;
  le(field: string, value: number): FilterExpression;
  gt(field: string, value: number): FilterExpression;
  ge(field: string, value: number): FilterExpression;

  // Range
  between(field: string, low: number, high: number): FilterExpression;

  // String
  contains(field: string, substring: string): FilterExpression;
  startsWith(field: string, prefix: string): FilterExpression;
  endsWith(field: string, suffix: string): FilterExpression;
  like(field: string, pattern: string): FilterExpression;

  // Set
  in(field: string, values: MetadataValue[]): FilterExpression;
  notIn(field: string, values: MetadataValue[]): FilterExpression;

  // Array
  any(field: string, value: MetadataValue): FilterExpression;
  all(field: string, values: MetadataValue[]): FilterExpression;
  none(field: string, values: MetadataValue[]): FilterExpression;

  // NULL
  isNull(field: string): FilterExpression;
  isNotNull(field: string): FilterExpression;

  // Logical
  and(...filters: FilterExpression[]): FilterExpression;
  or(...filters: FilterExpression[]): FilterExpression;
  not(filter: FilterExpression): FilterExpression;

  // Special
  readonly all: FilterExpression;
  readonly nothing: FilterExpression;
}

export const Filter: FilterStatic;

// ═══════════════════════════════════════════════════════════════════════════
// FILTER BUILDER
// ═══════════════════════════════════════════════════════════════════════════

/** Field condition builder */
export interface FieldCondition {
  eq(value: MetadataValue): FilterBuilder;
  ne(value: MetadataValue): FilterBuilder;
  lt(value: number): FilterBuilder;
  le(value: number): FilterBuilder;
  gt(value: number): FilterBuilder;
  ge(value: number): FilterBuilder;
  between(low: number, high: number): FilterBuilder;
  contains(substring: string): FilterBuilder;
  startsWith(prefix: string): FilterBuilder;
  endsWith(suffix: string): FilterBuilder;
  like(pattern: string): FilterBuilder;
  in(values: MetadataValue[]): FilterBuilder;
  notIn(values: MetadataValue[]): FilterBuilder;
  isNull(): FilterBuilder;
  isNotNull(): FilterBuilder;
}

/** Fluent filter builder */
export declare class FilterBuilder {
  constructor();
  where(field: string): FieldCondition;
  and(field: string): FieldCondition;
  or(field: string): FieldCondition;
  andGroup(builderFn: (b: FilterBuilder) => FilterBuilder): FilterBuilder;
  orGroup(builderFn: (b: FilterBuilder) => FilterBuilder): FilterBuilder;
  build(): FilterExpression;
  toString(): string;
}

// ═══════════════════════════════════════════════════════════════════════════
// SEARCH TYPES
// ═══════════════════════════════════════════════════════════════════════════

/** Filter strategy */
export type FilterStrategy = 'auto' | 'pre' | 'post' | 'hybrid';

/** Search options */
export interface SearchOptions {
  filter?: string | FilterExpression;
  strategy?: FilterStrategy;
  oversampleFactor?: number;
  includeMetadata?: boolean;
  includeVectors?: boolean;
  efSearch?: number;
}

/** Search result */
export interface SearchResult {
  id: number;
  score: number;
  metadata?: Metadata;
  vector?: Float32Array;
}

/** Filtered search result with diagnostics */
export interface FilteredSearchResult {
  results: SearchResult[];
  complete: boolean;
  observedSelectivity: number;
  strategyUsed: FilterStrategy;
  vectorsEvaluated: number;
  filterTimeMs: number;
  totalTimeMs: number;
}

// ═══════════════════════════════════════════════════════════════════════════
// INDEX
// ═══════════════════════════════════════════════════════════════════════════

/** Index configuration */
export interface IndexConfig {
  dimensions: number;
  m?: number;
  efConstruction?: number;
  quantized?: boolean;
}

/** EdgeVec index */
export declare class EdgeVecIndex {
  constructor(config: IndexConfig);

  /** Number of vectors in index */
  readonly size: number;

  /** Vector dimensions */
  readonly dimensions: number;

  /** Add vector with optional metadata */
  add(vector: Float32Array | number[], metadata?: Metadata): Promise<number>;

  /** Search for similar vectors */
  search(
    query: Float32Array | number[],
    k: number,
    options?: SearchOptions
  ): Promise<SearchResult[]>;

  /** Search with full diagnostics */
  searchFiltered(
    query: Float32Array | number[],
    k: number,
    options?: SearchOptions
  ): Promise<FilteredSearchResult>;

  /** Count vectors matching filter */
  count(filter?: string | FilterExpression): Promise<number>;

  /** Get metadata for a vector */
  getMetadata(id: number): Metadata | undefined;

  /** Get vector data */
  getVector(id: number): Float32Array | undefined;

  /** Serialize index to bytes */
  serialize(): Uint8Array;

  /** Load index from bytes */
  static deserialize(data: Uint8Array): EdgeVecIndex;
}

// ═══════════════════════════════════════════════════════════════════════════
// ERRORS
// ═══════════════════════════════════════════════════════════════════════════

/** Filter error codes */
export enum FilterErrorCode {
  SYNTAX_ERROR = 'E100',
  UNEXPECTED_EOF = 'E101',
  INVALID_CHAR = 'E102',
  UNCLOSED_STRING = 'E103',
  UNCLOSED_PAREN = 'E104',
  INVALID_NUMBER = 'E105',
  INVALID_ESCAPE = 'E106',
  TYPE_MISMATCH = 'E200',
  UNKNOWN_FIELD = 'E201',
  OVERFLOW = 'E202',
  NOT_AN_ARRAY = 'E203',
  EMPTY_ARRAY = 'E204',
  TOO_COMPLEX = 'E300',
  TOO_DEEP = 'E301',
  STRING_TOO_LONG = 'E302',
  ARRAY_TOO_LARGE = 'E303',
  EVALUATION_ERROR = 'E400',
  METADATA_ERROR = 'E401',
  MEMORY_ERROR = 'E402',
}

/** Filter exception */
export declare class FilterException extends Error {
  readonly code: string;
  readonly position?: SourcePosition;
  readonly suggestion?: string;
  readonly filterString?: string;

  constructor(
    code: string,
    message: string,
    position?: SourcePosition,
    suggestion?: string
  );

  format(): string;
}
```

### Acceptance Criteria
- [ ] All public types exported
- [ ] JSDoc comments on all types
- [ ] Filter factory interface complete
- [ ] FilterBuilder types correct
- [ ] SearchOptions complete
- [ ] Error types complete
- [ ] TypeScript compiler accepts all types

---

## Deliverables Checklist

| Artifact | Path | Status |
|:---------|:-----|:-------|
| Filter static class | `pkg/filter.ts` | [ ] |
| FilterBuilder | `pkg/filter-builder.ts` | [ ] |
| EdgeVecIndex methods | `pkg/edgevec.ts` | [ ] |
| Type definitions | `pkg/edgevec.d.ts` | [ ] |
| Jest tests | `pkg/__tests__/` | [ ] |

---

## End of Day 5 Gate

**Pass Criteria:**
- [ ] All 4 tasks complete
- [ ] `tsc --noEmit` passes (no type errors)
- [ ] `npm test` passes (Jest tests)
- [ ] All examples from FILTERING_WASM_API.md work
- [ ] Bundle size < 350KB

**Handoff:**
```
[WASM_SPECIALIST]: Day 5 Complete

Artifacts generated:
- pkg/filter.ts (Filter static class)
- pkg/filter-builder.ts (FilterBuilder fluent API)
- pkg/edgevec.ts (searchFiltered method)
- pkg/edgevec.d.ts (TypeScript definitions)

Status: READY_FOR_DAY_6

Next: W23.6.1 (Parser unit tests - 344 tests)
```

---

**Day 5 Total: 12 hours | 4 tasks | TypeScript API complete**
