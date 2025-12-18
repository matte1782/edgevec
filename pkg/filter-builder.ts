/**
 * EdgeVec Filter Builder - Fluent API
 *
 * Provides a chainable interface for building complex filters.
 *
 * @module filter-builder
 * @version 0.5.0
 *
 * @example
 * const filter = new FilterBuilder()
 *   .where('category').eq('gpu')
 *   .and('price').lt(1000)
 *   .or('featured').eq(true)
 *   .build();
 */

import { Filter, FilterExpression, MetadataValue } from './filter.js';

// =============================================================================
// Types
// =============================================================================

interface FilterPart {
  filter: FilterExpression;
  logic: 'and' | 'or' | null;
  grouped?: boolean;
}

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
  private parts: FilterPart[] = [];
  private pendingLogic: 'and' | 'or' | null = null;

  /**
   * Start a new condition on a field.
   *
   * @example builder.where('category').eq('gpu')
   */
  where(field: string): FieldCondition {
    return new FieldCondition(this, field);
  }

  /**
   * Add an AND condition.
   *
   * @example builder.where('a').eq(1).and('b').lt(10)
   */
  and(field: string): FieldCondition {
    this.pendingLogic = 'and';
    return new FieldCondition(this, field);
  }

  /**
   * Add an OR condition.
   *
   * @example builder.where('a').eq(1).or('b').eq(2)
   */
  or(field: string): FieldCondition {
    this.pendingLogic = 'or';
    return new FieldCondition(this, field);
  }

  /**
   * Start a grouped sub-expression with AND.
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
   * Start a grouped sub-expression with OR.
   *
   * @example builder.where('a').eq(1).orGroup(b => b.where('x').eq(1).and('y').eq(2))
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
   * Add an existing filter expression with AND.
   *
   * @example builder.where('a').eq(1).andFilter(existingFilter)
   */
  andFilter(filter: FilterExpression): FilterBuilder {
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
  orFilter(filter: FilterExpression): FilterBuilder {
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

  /**
   * Check if the builder has any conditions.
   */
  isEmpty(): boolean {
    return this.parts.length === 0;
  }

  /**
   * Reset the builder to empty state.
   */
  reset(): FilterBuilder {
    this.parts = [];
    this.pendingLogic = null;
    return this;
  }

  /** @internal */
  addPart(part: FilterPart): void {
    // First part doesn't have logic
    if (this.parts.length === 0) {
      part.logic = null;
    } else if (!part.logic && this.pendingLogic) {
      part.logic = this.pendingLogic;
    } else if (!part.logic && !this.pendingLogic) {
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

  /** ANY - array field contains value */
  any(value: MetadataValue): FilterBuilder {
    this.builder.addPart({
      filter: Filter.any(this.field, value),
      logic: null,
    });
    return this.builder;
  }

  /** ALL - array field contains all values */
  all(values: MetadataValue[]): FilterBuilder {
    this.builder.addPart({
      filter: Filter.allOf(this.field, values),
      logic: null,
    });
    return this.builder;
  }

  /** NONE - array field contains none of values */
  none(values: MetadataValue[]): FilterBuilder {
    this.builder.addPart({
      filter: Filter.none(this.field, values),
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

// Re-export for convenience
export { Filter, FilterExpression, MetadataValue } from './filter.js';

export default FilterBuilder;
