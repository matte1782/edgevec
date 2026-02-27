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
import { Filter } from './filter.js';
// =============================================================================
// FilterBuilder Class
// =============================================================================
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
    constructor() {
        this.parts = [];
        this.pendingLogic = null;
    }
    /**
     * Start a new condition on a field.
     *
     * @example builder.where('category').eq('gpu')
     */
    where(field) {
        return new FieldCondition(this, field);
    }
    /**
     * Add an AND condition.
     *
     * @example builder.where('a').eq(1).and('b').lt(10)
     */
    and(field) {
        this.pendingLogic = 'and';
        return new FieldCondition(this, field);
    }
    /**
     * Add an OR condition.
     *
     * @example builder.where('a').eq(1).or('b').eq(2)
     */
    or(field) {
        this.pendingLogic = 'or';
        return new FieldCondition(this, field);
    }
    /**
     * Start a grouped sub-expression with AND.
     *
     * @example builder.where('a').eq(1).andGroup(b => b.where('x').eq(1).or('y').eq(2))
     */
    andGroup(builderFn) {
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
     * Start a grouped sub-expression with OR.
     *
     * @example builder.where('a').eq(1).orGroup(b => b.where('x').eq(1).and('y').eq(2))
     */
    orGroup(builderFn) {
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
     * Add an existing filter expression with AND.
     *
     * @example builder.where('a').eq(1).andFilter(existingFilter)
     */
    andFilter(filter) {
        this.addPart({
            filter,
            logic: 'and',
            grouped: false,
        });
        return this;
    }
    /**
     * Add an existing filter expression with OR.
     *
     * @example builder.where('a').eq(1).orFilter(existingFilter)
     */
    orFilter(filter) {
        this.addPart({
            filter,
            logic: 'or',
            grouped: false,
        });
        return this;
    }
    /**
     * Build the final filter expression.
     *
     * @throws Error if builder is empty
     */
    build() {
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
            }
            else {
                result = Filter.or(result, partExpr);
            }
        }
        return result;
    }
    /**
     * Get string representation of current state.
     */
    toString() {
        try {
            return this.build().toString();
        }
        catch {
            return '<incomplete filter>';
        }
    }
    /**
     * Check if the builder has any conditions.
     */
    isEmpty() {
        return this.parts.length === 0;
    }
    /**
     * Reset the builder to empty state.
     */
    reset() {
        this.parts = [];
        this.pendingLogic = null;
        return this;
    }
    /** @internal */
    addPart(part) {
        // First part doesn't have logic
        if (this.parts.length === 0) {
            part.logic = null;
        }
        else if (!part.logic && this.pendingLogic) {
            part.logic = this.pendingLogic;
        }
        else if (!part.logic && !this.pendingLogic) {
            // Default to AND if no logic specified
            part.logic = 'and';
        }
        this.parts.push(part);
        this.pendingLogic = null;
    }
}
// =============================================================================
// FieldCondition Class
// =============================================================================
/**
 * Field condition builder (returned by where/and/or).
 */
export class FieldCondition {
    constructor(builder, field) {
        this.builder = builder;
        this.field = field;
    }
    /** Equal to */
    eq(value) {
        this.builder.addPart({
            filter: Filter.eq(this.field, value),
            logic: null,
        });
        return this.builder;
    }
    /** Not equal to */
    ne(value) {
        this.builder.addPart({
            filter: Filter.ne(this.field, value),
            logic: null,
        });
        return this.builder;
    }
    /** Less than */
    lt(value) {
        this.builder.addPart({
            filter: Filter.lt(this.field, value),
            logic: null,
        });
        return this.builder;
    }
    /** Less than or equal */
    le(value) {
        this.builder.addPart({
            filter: Filter.le(this.field, value),
            logic: null,
        });
        return this.builder;
    }
    /** Greater than */
    gt(value) {
        this.builder.addPart({
            filter: Filter.gt(this.field, value),
            logic: null,
        });
        return this.builder;
    }
    /** Greater than or equal */
    ge(value) {
        this.builder.addPart({
            filter: Filter.ge(this.field, value),
            logic: null,
        });
        return this.builder;
    }
    /** Between (inclusive) */
    between(low, high) {
        this.builder.addPart({
            filter: Filter.between(this.field, low, high),
            logic: null,
        });
        return this.builder;
    }
    /** Contains substring */
    contains(substring) {
        this.builder.addPart({
            filter: Filter.contains(this.field, substring),
            logic: null,
        });
        return this.builder;
    }
    /** Starts with prefix */
    startsWith(prefix) {
        this.builder.addPart({
            filter: Filter.startsWith(this.field, prefix),
            logic: null,
        });
        return this.builder;
    }
    /** Ends with suffix */
    endsWith(suffix) {
        this.builder.addPart({
            filter: Filter.endsWith(this.field, suffix),
            logic: null,
        });
        return this.builder;
    }
    /** LIKE pattern match */
    like(pattern) {
        this.builder.addPart({
            filter: Filter.like(this.field, pattern),
            logic: null,
        });
        return this.builder;
    }
    /** In array of values */
    in(values) {
        this.builder.addPart({
            filter: Filter.in(this.field, values),
            logic: null,
        });
        return this.builder;
    }
    /** Not in array of values */
    notIn(values) {
        this.builder.addPart({
            filter: Filter.notIn(this.field, values),
            logic: null,
        });
        return this.builder;
    }
    /** ANY - array field contains value */
    any(value) {
        this.builder.addPart({
            filter: Filter.any(this.field, value),
            logic: null,
        });
        return this.builder;
    }
    /** ALL - array field contains all values */
    all(values) {
        this.builder.addPart({
            filter: Filter.allOf(this.field, values),
            logic: null,
        });
        return this.builder;
    }
    /** NONE - array field contains none of values */
    none(values) {
        this.builder.addPart({
            filter: Filter.none(this.field, values),
            logic: null,
        });
        return this.builder;
    }
    /** Is null */
    isNull() {
        this.builder.addPart({
            filter: Filter.isNull(this.field),
            logic: null,
        });
        return this.builder;
    }
    /** Is not null */
    isNotNull() {
        this.builder.addPart({
            filter: Filter.isNotNull(this.field),
            logic: null,
        });
        return this.builder;
    }
}
// Re-export for convenience
export { Filter } from './filter.js';
export default FilterBuilder;
//# sourceMappingURL=filter-builder.js.map