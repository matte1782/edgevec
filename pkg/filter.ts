/**
 * EdgeVec Filter System - TypeScript API
 *
 * Provides two ways to create filters:
 * 1. String parsing: Filter.parse('category = "gpu"')
 * 2. Builder methods: Filter.eq('category', 'gpu')
 *
 * @module filter
 * @version 0.5.0
 */

import {
  parse_filter_js,
  validate_filter_js,
  try_parse_filter_js,
  get_filter_info_js,
} from './edgevec.js';

// =============================================================================
// Types
// =============================================================================

/** Supported metadata value types */
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

/** Filter information returned by get_filter_info_js */
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

// =============================================================================
// FilterImpl Class
// =============================================================================

/**
 * Internal filter implementation.
 */
class FilterImpl implements FilterExpression {
  readonly _json: string;
  private _parsed: object | null = null;
  private _info: FilterInfo | null = null;
  private _stringForm: string | null = null;

  constructor(json: string, stringForm?: string) {
    this._json = json;
    this._stringForm = stringForm || null;
  }

  toString(): string {
    if (this._stringForm) {
      return this._stringForm;
    }
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
    const obj = this.toJSON() as { LiteralBool?: boolean };
    return obj.LiteralBool === true;
  }

  get isContradiction(): boolean {
    const obj = this.toJSON() as { LiteralBool?: boolean };
    return obj.LiteralBool === false;
  }

  get complexity(): number {
    return this._getInfo().complexity;
  }

  private _getInfo(): FilterInfo {
    if (!this._info) {
      try {
        const infoJson = get_filter_info_js(this.toString());
        this._info = JSON.parse(infoJson);
      } catch {
        // Fallback if get_filter_info fails
        this._info = {
          nodeCount: 1,
          depth: 1,
          fields: [],
          operators: [],
          complexity: 1,
        };
      }
    }
    return this._info!;
  }

  private _reconstructString(ast: unknown): string {
    const node = ast as Record<string, unknown>;

    // Handle Rust enum-style serialization (e.g., { "Eq": [left, right] })
    const keys = Object.keys(node);
    if (keys.length === 1) {
      const key = keys[0];
      const value = node[key];

      switch (key) {
        case 'Field':
          return String(value);

        case 'LiteralString':
          return `"${this._escapeString(String(value))}"`;

        case 'LiteralInt':
        case 'LiteralFloat':
          return String(value);

        case 'LiteralBool':
          return value ? 'true' : 'false';

        case 'LiteralArray': {
          const items = value as unknown[];
          return '[' + items.map((v) => this._reconstructString(v)).join(', ') + ']';
        }

        case 'Eq': {
          const [left, right] = value as [unknown, unknown];
          return `${this._reconstructString(left)} = ${this._reconstructString(right)}`;
        }

        case 'Ne': {
          const [left, right] = value as [unknown, unknown];
          return `${this._reconstructString(left)} != ${this._reconstructString(right)}`;
        }

        case 'Lt': {
          const [left, right] = value as [unknown, unknown];
          return `${this._reconstructString(left)} < ${this._reconstructString(right)}`;
        }

        case 'Le': {
          const [left, right] = value as [unknown, unknown];
          return `${this._reconstructString(left)} <= ${this._reconstructString(right)}`;
        }

        case 'Gt': {
          const [left, right] = value as [unknown, unknown];
          return `${this._reconstructString(left)} > ${this._reconstructString(right)}`;
        }

        case 'Ge': {
          const [left, right] = value as [unknown, unknown];
          return `${this._reconstructString(left)} >= ${this._reconstructString(right)}`;
        }

        case 'And': {
          const [left, right] = value as [unknown, unknown];
          return `(${this._reconstructString(left)} AND ${this._reconstructString(right)})`;
        }

        case 'Or': {
          const [left, right] = value as [unknown, unknown];
          return `(${this._reconstructString(left)} OR ${this._reconstructString(right)})`;
        }

        case 'Not': {
          return `NOT (${this._reconstructString(value)})`;
        }

        case 'Contains': {
          const [left, right] = value as [unknown, unknown];
          return `${this._reconstructString(left)} CONTAINS ${this._reconstructString(right)}`;
        }

        case 'StartsWith': {
          const [left, right] = value as [unknown, unknown];
          return `${this._reconstructString(left)} STARTS_WITH ${this._reconstructString(right)}`;
        }

        case 'EndsWith': {
          const [left, right] = value as [unknown, unknown];
          return `${this._reconstructString(left)} ENDS_WITH ${this._reconstructString(right)}`;
        }

        case 'Like': {
          const [left, right] = value as [unknown, unknown];
          return `${this._reconstructString(left)} LIKE ${this._reconstructString(right)}`;
        }

        case 'Between': {
          const [field, low, high] = value as [unknown, unknown, unknown];
          return `${this._reconstructString(field)} BETWEEN ${this._reconstructString(low)} AND ${this._reconstructString(high)}`;
        }

        case 'In': {
          const [field, values] = value as [unknown, unknown];
          return `${this._reconstructString(field)} IN ${this._reconstructString(values)}`;
        }

        case 'NotIn': {
          const [field, values] = value as [unknown, unknown];
          return `${this._reconstructString(field)} NOT IN ${this._reconstructString(values)}`;
        }

        case 'Any': {
          const [field, val] = value as [unknown, unknown];
          return `ANY(${this._reconstructString(field)}, ${this._reconstructString(val)})`;
        }

        case 'All': {
          const [field, values] = value as [unknown, unknown];
          return `ALL(${this._reconstructString(field)}, ${this._reconstructString(values)})`;
        }

        case 'None': {
          const [field, values] = value as [unknown, unknown];
          return `NONE(${this._reconstructString(field)}, ${this._reconstructString(values)})`;
        }

        case 'IsNull':
          return `${this._reconstructString(value)} IS NULL`;

        case 'IsNotNull':
          return `${this._reconstructString(value)} IS NOT NULL`;

        default:
          return JSON.stringify(ast);
      }
    }

    return JSON.stringify(ast);
  }

  private _escapeString(s: string): string {
    return s.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
  }
}

// =============================================================================
// Filter Factory
// =============================================================================

/**
 * Filter factory and static methods.
 */
export const Filter = {
  // ===========================================================================
  // PARSING
  // ===========================================================================

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
    return new FilterImpl(json, query);
  },

  /**
   * Try to parse a filter string, returning null on error.
   *
   * @param query - Filter string to parse
   * @returns Compiled filter or null
   *
   * @example
   * const filter = Filter.tryParse(userInput);
   * if (filter) { // valid }
   */
  tryParse(query: string): FilterExpression | null {
    const result = try_parse_filter_js(query);
    if (result === null) return null;
    return new FilterImpl(result as string, query);
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
      filter: result.valid ? Filter.tryParse(query) || undefined : undefined,
    };
  },

  // ===========================================================================
  // COMPARISON OPERATORS
  // ===========================================================================

  /**
   * Create equality filter: field = value
   *
   * @example Filter.eq('category', 'gpu')
   */
  eq(field: string, value: MetadataValue): FilterExpression {
    const query = `${field} = ${Filter._valueToString(value)}`;
    return Filter.parse(query);
  },

  /**
   * Create inequality filter: field != value
   *
   * @example Filter.ne('status', 'deleted')
   */
  ne(field: string, value: MetadataValue): FilterExpression {
    const query = `${field} != ${Filter._valueToString(value)}`;
    return Filter.parse(query);
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

  // ===========================================================================
  // RANGE OPERATORS
  // ===========================================================================

  /**
   * Create range filter: low <= field <= high
   *
   * @example Filter.between('price', 100, 500)
   */
  between(field: string, low: number, high: number): FilterExpression {
    return Filter.parse(`${field} BETWEEN ${low} AND ${high}`);
  },

  // ===========================================================================
  // STRING OPERATORS
  // ===========================================================================

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

  // ===========================================================================
  // SET OPERATORS
  // ===========================================================================

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

  // ===========================================================================
  // ARRAY OPERATORS
  // ===========================================================================

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
   * @example Filter.allOf('tags', ['gpu', 'gaming'])
   */
  allOf(field: string, values: MetadataValue[]): FilterExpression {
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

  // ===========================================================================
  // NULL OPERATORS
  // ===========================================================================

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

  // ===========================================================================
  // LOGICAL OPERATORS
  // ===========================================================================

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

    const combined = filters.map((f) => `(${f.toString()})`).join(' AND ');
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

    const combined = filters.map((f) => `(${f.toString()})`).join(' OR ');
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

  // ===========================================================================
  // SPECIAL FILTERS
  // ===========================================================================

  /** Filter that matches all vectors (no filtering) */
  get matchAll(): FilterExpression {
    return new FilterImpl('{"LiteralBool":true}', 'true');
  },

  /** Filter that matches no vectors (empty result) */
  get nothing(): FilterExpression {
    return new FilterImpl('{"LiteralBool":false}', 'false');
  },

  // ===========================================================================
  // INTERNAL HELPERS
  // ===========================================================================

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
    const items = values.map((v) => Filter._valueToString(v)).join(', ');
    return `[${items}]`;
  },

  _escapeString(s: string): string {
    return s.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
  },
};

export default Filter;

// =============================================================================
// Standalone Function Exports (v0.8.0)
// =============================================================================

/**
 * Standalone filter functions for functional composition.
 *
 * These are convenience wrappers around Filter.* methods for cleaner imports:
 * ```typescript
 * import { eq, and, gt } from 'edgevec';
 * const f = and(eq('category', 'books'), gt('price', 10));
 * ```
 */

// Comparison operators
export const eq = Filter.eq.bind(Filter);
export const ne = Filter.ne.bind(Filter);
export const gt = Filter.gt.bind(Filter);
export const lt = Filter.lt.bind(Filter);
export const ge = Filter.ge.bind(Filter);
export const le = Filter.le.bind(Filter);
export const between = Filter.between.bind(Filter);

// String operators
export const contains = Filter.contains.bind(Filter);
export const startsWith = Filter.startsWith.bind(Filter);
export const endsWith = Filter.endsWith.bind(Filter);
export const like = Filter.like.bind(Filter);

// Set/Array operators
export const inArray = Filter.in.bind(Filter);
export const notInArray = Filter.notIn.bind(Filter);
export const any = Filter.any.bind(Filter);
export const all = Filter.allOf.bind(Filter);
export const none = Filter.none.bind(Filter);

// Null operators
export const isNull = Filter.isNull.bind(Filter);
export const isNotNull = Filter.isNotNull.bind(Filter);

// Logical operators
export const and = Filter.and.bind(Filter);
export const or = Filter.or.bind(Filter);
export const not = Filter.not.bind(Filter);

// Special filters
export const matchAll = Filter.matchAll;
export const matchNone = Filter.nothing;

/**
 * Wrapper to convert filter expression to string for search options.
 * Useful when you want to be explicit about the conversion.
 *
 * @example
 * const f = filter(and(eq('a', 1), gt('b', 2)));
 * // f is now a FilterExpression that can be used directly in search
 */
export function filter(expr: FilterExpression): FilterExpression {
  return expr;
}
