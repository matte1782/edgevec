/**
 * EdgeVec TypeScript Type Definitions
 *
 * @version 0.5.0
 * @module edgevec
 */

// =============================================================================
// METADATA TYPES
// =============================================================================

/** Supported metadata value types */
export type MetadataValue = string | number | boolean | string[];

/** Metadata record for a vector */
export type Metadata = Record<string, MetadataValue>;

// =============================================================================
// FILTER TYPES
// =============================================================================

/** Compiled filter expression */
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

/** Filter validation result */
export interface FilterValidation {
  /** Whether the filter is valid */
  valid: boolean;
  /** List of errors (empty if valid) */
  errors: FilterValidationError[];
  /** List of warnings */
  warnings: FilterValidationWarning[];
  /** Compiled filter (if valid) */
  filter?: FilterExpression;
}

/** Filter validation error */
export interface FilterValidationError {
  /** Error code (e.g., "E001") */
  code: string;
  /** Human-readable error message */
  message: string;
  /** Position in source string */
  position?: SourcePosition;
  /** Suggestion for fixing the error */
  suggestion?: string;
}

/** Filter validation warning */
export interface FilterValidationWarning {
  /** Warning code */
  code: string;
  /** Human-readable warning message */
  message: string;
  /** Position in source string */
  position?: SourcePosition;
}

/** Source position in filter string */
export interface SourcePosition {
  /** Line number (1-indexed) */
  line: number;
  /** Column number (1-indexed) */
  column: number;
  /** Byte offset (0-indexed) */
  offset: number;
}

// =============================================================================
// FILTER STATIC INTERFACE
// =============================================================================

/** Filter factory interface */
export interface FilterStatic {
  // Parsing
  /**
   * Parse a filter string into a compiled filter.
   * @throws FilterException on syntax error
   */
  parse(query: string): FilterExpression;

  /**
   * Try to parse a filter string, returning null on error.
   */
  tryParse(query: string): FilterExpression | null;

  /**
   * Validate a filter string without compiling.
   */
  validate(query: string): FilterValidation;

  // Comparison operators
  /** Create equality filter: field = value */
  eq(field: string, value: MetadataValue): FilterExpression;
  /** Create inequality filter: field != value */
  ne(field: string, value: MetadataValue): FilterExpression;
  /** Create less-than filter: field < value */
  lt(field: string, value: number): FilterExpression;
  /** Create less-than-or-equal filter: field <= value */
  le(field: string, value: number): FilterExpression;
  /** Create greater-than filter: field > value */
  gt(field: string, value: number): FilterExpression;
  /** Create greater-than-or-equal filter: field >= value */
  ge(field: string, value: number): FilterExpression;

  // Range operators
  /** Create range filter: low <= field <= high */
  between(field: string, low: number, high: number): FilterExpression;

  // String operators
  /** Create contains filter: field CONTAINS substring */
  contains(field: string, substring: string): FilterExpression;
  /** Create starts-with filter: field STARTS_WITH prefix */
  startsWith(field: string, prefix: string): FilterExpression;
  /** Create ends-with filter: field ENDS_WITH suffix */
  endsWith(field: string, suffix: string): FilterExpression;
  /** Create pattern match filter: field LIKE pattern */
  like(field: string, pattern: string): FilterExpression;

  // Set operators
  /** Create IN filter: field IN [values] */
  in(field: string, values: MetadataValue[]): FilterExpression;
  /** Create NOT IN filter: field NOT IN [values] */
  notIn(field: string, values: MetadataValue[]): FilterExpression;

  // Array operators
  /** Create ANY filter: ANY(field, value) */
  any(field: string, value: MetadataValue): FilterExpression;
  /** Create ALL filter: ALL(field, values) */
  allOf(field: string, values: MetadataValue[]): FilterExpression;
  /** Create NONE filter: NONE(field, values) */
  none(field: string, values: MetadataValue[]): FilterExpression;

  // NULL operators
  /** Create IS NULL filter */
  isNull(field: string): FilterExpression;
  /** Create IS NOT NULL filter */
  isNotNull(field: string): FilterExpression;

  // Logical operators
  /** Create AND combination of filters */
  and(...filters: FilterExpression[]): FilterExpression;
  /** Create OR combination of filters */
  or(...filters: FilterExpression[]): FilterExpression;
  /** Create NOT (negation) of a filter */
  not(filter: FilterExpression): FilterExpression;

  // Special filters
  /** Filter that matches all vectors (no filtering) */
  readonly matchAll: FilterExpression;
  /** Filter that matches no vectors (empty result) */
  readonly nothing: FilterExpression;
}

/** Filter factory */
export const Filter: FilterStatic;

// =============================================================================
// FILTER BUILDER
// =============================================================================

/** Field condition builder (returned by where/and/or) */
export interface FieldCondition {
  /** Equal to */
  eq(value: MetadataValue): FilterBuilder;
  /** Not equal to */
  ne(value: MetadataValue): FilterBuilder;
  /** Less than */
  lt(value: number): FilterBuilder;
  /** Less than or equal */
  le(value: number): FilterBuilder;
  /** Greater than */
  gt(value: number): FilterBuilder;
  /** Greater than or equal */
  ge(value: number): FilterBuilder;
  /** Between (inclusive) */
  between(low: number, high: number): FilterBuilder;
  /** Contains substring */
  contains(substring: string): FilterBuilder;
  /** Starts with prefix */
  startsWith(prefix: string): FilterBuilder;
  /** Ends with suffix */
  endsWith(suffix: string): FilterBuilder;
  /** LIKE pattern match */
  like(pattern: string): FilterBuilder;
  /** In array of values */
  in(values: MetadataValue[]): FilterBuilder;
  /** Not in array of values */
  notIn(values: MetadataValue[]): FilterBuilder;
  /** ANY - array field contains value */
  any(value: MetadataValue): FilterBuilder;
  /** ALL - array field contains all values */
  all(values: MetadataValue[]): FilterBuilder;
  /** NONE - array field contains none of values */
  none(values: MetadataValue[]): FilterBuilder;
  /** Is null */
  isNull(): FilterBuilder;
  /** Is not null */
  isNotNull(): FilterBuilder;
}

/** Fluent filter builder */
export declare class FilterBuilder {
  constructor();

  /** Start a new condition on a field */
  where(field: string): FieldCondition;
  /** Add an AND condition */
  and(field: string): FieldCondition;
  /** Add an OR condition */
  or(field: string): FieldCondition;
  /** Start a grouped sub-expression with AND */
  andGroup(builderFn: (b: FilterBuilder) => FilterBuilder): FilterBuilder;
  /** Start a grouped sub-expression with OR */
  orGroup(builderFn: (b: FilterBuilder) => FilterBuilder): FilterBuilder;
  /** Add an existing filter expression with AND */
  andFilter(filter: FilterExpression): FilterBuilder;
  /** Add an existing filter expression with OR */
  orFilter(filter: FilterExpression): FilterBuilder;
  /** Build the final filter expression */
  build(): FilterExpression;
  /** Get string representation of current state */
  toString(): string;
  /** Check if the builder has any conditions */
  isEmpty(): boolean;
  /** Reset the builder to empty state */
  reset(): FilterBuilder;
}

// =============================================================================
// SEARCH TYPES
// =============================================================================

/** Filter strategy for search */
export type FilterStrategy = 'auto' | 'pre' | 'post' | 'hybrid';

/** Search options */
export interface SearchOptions {
  /** Filter expression (string or Filter object) */
  filter?: string | FilterExpression;
  /** Filter strategy */
  strategy?: FilterStrategy;
  /** Oversample factor for post/hybrid strategy */
  oversampleFactor?: number;
  /** Include metadata in results */
  includeMetadata?: boolean;
  /** Include vectors in results */
  includeVectors?: boolean;
  /** Override ef_search for this query */
  efSearch?: number;
}

/** Search result */
export interface SearchResult {
  /** Vector ID */
  id: number;
  /** Similarity score (lower is more similar for distance metrics) */
  score: number;
  /** Metadata (if includeMetadata=true) */
  metadata?: Metadata;
  /** Vector data (if includeVectors=true) */
  vector?: Float32Array;
}

/** Filtered search result with diagnostics */
export interface FilteredSearchResult {
  /** Search results */
  results: SearchResult[];
  /** Whether k results were found */
  complete: boolean;
  /** Observed selectivity (0.0 - 1.0, lower means more selective) */
  observedSelectivity: number;
  /** Strategy used */
  strategyUsed: FilterStrategy;
  /** Number of vectors evaluated */
  vectorsEvaluated: number;
  /** Filter evaluation time (ms) */
  filterTimeMs: number;
  /** Total search time (ms) */
  totalTimeMs: number;
}

// =============================================================================
// INDEX TYPES
// =============================================================================

/** Index configuration */
export interface IndexConfig {
  /** Vector dimensions */
  dimensions: number;
  /** HNSW M parameter (connections per node) */
  m?: number;
  /** HNSW ef_construction parameter */
  efConstruction?: number;
  /** Enable quantization for smaller memory footprint */
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
  add(vector: Float32Array | number[], metadata?: Metadata): number;

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

  /** Set metadata for a vector */
  setMetadata(id: number, key: string, value: MetadataValue): void;

  /** Delete a vector (soft delete) */
  delete(id: number): boolean;

  /** Save index to IndexedDB */
  save(name: string): Promise<void>;

  /** Load index from IndexedDB */
  static load(name: string): Promise<EdgeVecIndex>;
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/** Filter error codes */
export enum FilterErrorCode {
  // Syntax errors (E001-E007)
  SYNTAX_ERROR = 'E001',
  UNEXPECTED_EOF = 'E002',
  INVALID_CHAR = 'E003',
  UNCLOSED_STRING = 'E004',
  UNCLOSED_PAREN = 'E005',
  INVALID_ESCAPE = 'E006',
  INVALID_NUMBER = 'E007',

  // Type errors (E101-E104)
  TYPE_MISMATCH = 'E101',
  UNKNOWN_FIELD = 'E102',
  INCOMPATIBLE_TYPES = 'E103',
  INVALID_OPERATOR = 'E104',

  // Runtime errors (E201-E204)
  DIVISION_BY_ZERO = 'E201',
  NULL_VALUE = 'E202',
  INDEX_OUT_OF_BOUNDS = 'E203',
  INVALID_EXPRESSION = 'E204',

  // Limit errors (E301-E304)
  NESTING_TOO_DEEP = 'E301',
  INPUT_TOO_LONG = 'E302',
  EXPRESSION_TOO_COMPLEX = 'E303',
  ARRAY_TOO_LARGE = 'E304',

  // Strategy errors (E401)
  INVALID_STRATEGY = 'E401',
}

/** Filter exception */
export declare class FilterException extends Error {
  /** Error code */
  readonly code: string;
  /** Position in source string */
  readonly position?: SourcePosition;
  /** Suggestion for fixing the error */
  readonly suggestion?: string;
  /** Original filter string */
  readonly filterString?: string;

  constructor(
    code: string,
    message: string,
    position?: SourcePosition,
    suggestion?: string,
    filterString?: string
  );

  /** Format error with source context */
  format(): string;

  /** Create from JSON error response */
  static fromJson(json: string, filterString?: string): FilterException;
}
