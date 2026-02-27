/**
 * EdgeVec Filter Builder - Fluent API
 *
 * Provides a chainable interface for building complex filters.
 *
 * @module filter-builder
 * @version 0.9.0
 *
 * @example
 * const filter = new FilterBuilder()
 *   .where('category').eq('gpu')
 *   .and('price').lt(1000)
 *   .or('featured').eq(true)
 *   .build();
 */
import { FilterExpression, MetadataValue } from './filter.js';
interface FilterPart {
    filter: FilterExpression;
    logic: 'and' | 'or' | null;
    grouped?: boolean;
}
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
export declare class FilterBuilder {
    private parts;
    private pendingLogic;
    /**
     * Start a new condition on a field.
     *
     * @example builder.where('category').eq('gpu')
     */
    where(field: string): FieldCondition;
    /**
     * Add an AND condition.
     *
     * @example builder.where('a').eq(1).and('b').lt(10)
     */
    and(field: string): FieldCondition;
    /**
     * Add an OR condition.
     *
     * @example builder.where('a').eq(1).or('b').eq(2)
     */
    or(field: string): FieldCondition;
    /**
     * Start a grouped sub-expression with AND.
     *
     * @example builder.where('a').eq(1).andGroup(b => b.where('x').eq(1).or('y').eq(2))
     */
    andGroup(builderFn: (b: FilterBuilder) => FilterBuilder): FilterBuilder;
    /**
     * Start a grouped sub-expression with OR.
     *
     * @example builder.where('a').eq(1).orGroup(b => b.where('x').eq(1).and('y').eq(2))
     */
    orGroup(builderFn: (b: FilterBuilder) => FilterBuilder): FilterBuilder;
    /**
     * Add an existing filter expression with AND.
     *
     * @example builder.where('a').eq(1).andFilter(existingFilter)
     */
    andFilter(filter: FilterExpression): FilterBuilder;
    /**
     * Add an existing filter expression with OR.
     *
     * @example builder.where('a').eq(1).orFilter(existingFilter)
     */
    orFilter(filter: FilterExpression): FilterBuilder;
    /**
     * Build the final filter expression.
     *
     * @throws Error if builder is empty
     */
    build(): FilterExpression;
    /**
     * Get string representation of current state.
     */
    toString(): string;
    /**
     * Check if the builder has any conditions.
     */
    isEmpty(): boolean;
    /**
     * Reset the builder to empty state.
     */
    reset(): FilterBuilder;
    /** @internal */
    addPart(part: FilterPart): void;
}
/**
 * Field condition builder (returned by where/and/or).
 */
export declare class FieldCondition {
    private builder;
    private field;
    constructor(builder: FilterBuilder, field: string);
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
export { Filter, FilterExpression, MetadataValue } from './filter.js';
export default FilterBuilder;
//# sourceMappingURL=filter-builder.d.ts.map