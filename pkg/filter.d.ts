/**
 * EdgeVec Filter System - TypeScript API
 *
 * Provides two ways to create filters:
 * 1. String parsing: Filter.parse('category = "gpu"')
 * 2. Builder methods: Filter.eq('category', 'gpu')
 *
 * @module filter
 * @version 0.9.0
 */
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
    position?: {
        line: number;
        column: number;
        offset: number;
    };
    suggestion?: string;
}
export interface FilterValidationWarning {
    code: string;
    message: string;
    position?: {
        line: number;
        column: number;
        offset: number;
    };
}
/**
 * Filter factory and static methods.
 */
export declare const Filter: {
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
    parse(query: string): FilterExpression;
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
    tryParse(query: string): FilterExpression | null;
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
    validate(query: string): FilterValidation;
    /**
     * Create equality filter: field = value
     *
     * @example Filter.eq('category', 'gpu')
     */
    eq(field: string, value: MetadataValue): FilterExpression;
    /**
     * Create inequality filter: field != value
     *
     * @example Filter.ne('status', 'deleted')
     */
    ne(field: string, value: MetadataValue): FilterExpression;
    /**
     * Create less-than filter: field < value
     *
     * @example Filter.lt('price', 1000)
     */
    lt(field: string, value: number): FilterExpression;
    /**
     * Create less-than-or-equal filter: field <= value
     *
     * @example Filter.le('price', 1000)
     */
    le(field: string, value: number): FilterExpression;
    /**
     * Create greater-than filter: field > value
     *
     * @example Filter.gt('rating', 4.0)
     */
    gt(field: string, value: number): FilterExpression;
    /**
     * Create greater-than-or-equal filter: field >= value
     *
     * @example Filter.ge('stock', 10)
     */
    ge(field: string, value: number): FilterExpression;
    /**
     * Create range filter: low <= field <= high
     *
     * @example Filter.between('price', 100, 500)
     */
    between(field: string, low: number, high: number): FilterExpression;
    /**
     * Create contains filter: field CONTAINS substring
     *
     * @example Filter.contains('title', 'NVIDIA')
     */
    contains(field: string, substring: string): FilterExpression;
    /**
     * Create starts-with filter: field STARTS_WITH prefix
     *
     * @example Filter.startsWith('sku', 'GPU-')
     */
    startsWith(field: string, prefix: string): FilterExpression;
    /**
     * Create ends-with filter: field ENDS_WITH suffix
     *
     * @example Filter.endsWith('filename', '.pdf')
     */
    endsWith(field: string, suffix: string): FilterExpression;
    /**
     * Create pattern match filter: field LIKE pattern
     *
     * @example Filter.like('email', '%@company.com')
     */
    like(field: string, pattern: string): FilterExpression;
    /**
     * Create IN filter: field IN [values]
     *
     * @example Filter.in('category', ['gpu', 'cpu', 'ram'])
     */
    in(field: string, values: MetadataValue[]): FilterExpression;
    /**
     * Create NOT IN filter: field NOT IN [values]
     *
     * @example Filter.notIn('status', ['deleted', 'archived'])
     */
    notIn(field: string, values: MetadataValue[]): FilterExpression;
    /**
     * Create ANY filter: ANY(field, value)
     *
     * @example Filter.any('tags', 'nvidia')
     */
    any(field: string, value: MetadataValue): FilterExpression;
    /**
     * Create ALL filter: ALL(field, values)
     *
     * @example Filter.allOf('tags', ['gpu', 'gaming'])
     */
    allOf(field: string, values: MetadataValue[]): FilterExpression;
    /**
     * Create NONE filter: NONE(field, values)
     *
     * @example Filter.none('tags', ['nsfw', 'spam'])
     */
    none(field: string, values: MetadataValue[]): FilterExpression;
    /**
     * Create IS NULL filter
     *
     * @example Filter.isNull('deletedAt')
     */
    isNull(field: string): FilterExpression;
    /**
     * Create IS NOT NULL filter
     *
     * @example Filter.isNotNull('verifiedAt')
     */
    isNotNull(field: string): FilterExpression;
    /**
     * Create AND combination of filters
     *
     * @example Filter.and(Filter.eq('a', 1), Filter.eq('b', 2))
     */
    and(...filters: FilterExpression[]): FilterExpression;
    /**
     * Create OR combination of filters
     *
     * @example Filter.or(Filter.eq('a', 1), Filter.eq('b', 2))
     */
    or(...filters: FilterExpression[]): FilterExpression;
    /**
     * Create NOT (negation) of a filter
     *
     * @example Filter.not(Filter.eq('status', 'deleted'))
     */
    not(filter: FilterExpression): FilterExpression;
    /** Filter that matches all vectors (no filtering) */
    readonly matchAll: FilterExpression;
    /** Filter that matches no vectors (empty result) */
    readonly nothing: FilterExpression;
    _valueToString(value: MetadataValue): string;
    _arrayToString(values: MetadataValue[]): string;
    _escapeString(s: string): string;
};
export default Filter;
//# sourceMappingURL=filter.d.ts.map