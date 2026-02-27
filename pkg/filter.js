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
import { parse_filter_js, validate_filter_js, try_parse_filter_js, get_filter_info_js, } from './edgevec.js';
// =============================================================================
// FilterImpl Class
// =============================================================================
/**
 * Internal filter implementation.
 */
class FilterImpl {
    constructor(json, stringForm) {
        this._parsed = null;
        this._info = null;
        this._stringForm = null;
        this._json = json;
        this._stringForm = stringForm || null;
    }
    toString() {
        if (this._stringForm) {
            return this._stringForm;
        }
        // Convert JSON AST back to string syntax
        return this._reconstructString(this.toJSON());
    }
    toJSON() {
        if (!this._parsed) {
            this._parsed = JSON.parse(this._json);
        }
        return this._parsed;
    }
    get isTautology() {
        const obj = this.toJSON();
        return obj.LiteralBool === true;
    }
    get isContradiction() {
        const obj = this.toJSON();
        return obj.LiteralBool === false;
    }
    get complexity() {
        return this._getInfo().complexity;
    }
    _getInfo() {
        if (!this._info) {
            try {
                const infoJson = get_filter_info_js(this.toString());
                this._info = JSON.parse(infoJson);
            }
            catch {
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
        return this._info;
    }
    _reconstructString(ast) {
        const node = ast;
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
                    const items = value;
                    return '[' + items.map((v) => this._reconstructString(v)).join(', ') + ']';
                }
                case 'Eq': {
                    const [left, right] = value;
                    return `${this._reconstructString(left)} = ${this._reconstructString(right)}`;
                }
                case 'Ne': {
                    const [left, right] = value;
                    return `${this._reconstructString(left)} != ${this._reconstructString(right)}`;
                }
                case 'Lt': {
                    const [left, right] = value;
                    return `${this._reconstructString(left)} < ${this._reconstructString(right)}`;
                }
                case 'Le': {
                    const [left, right] = value;
                    return `${this._reconstructString(left)} <= ${this._reconstructString(right)}`;
                }
                case 'Gt': {
                    const [left, right] = value;
                    return `${this._reconstructString(left)} > ${this._reconstructString(right)}`;
                }
                case 'Ge': {
                    const [left, right] = value;
                    return `${this._reconstructString(left)} >= ${this._reconstructString(right)}`;
                }
                case 'And': {
                    const [left, right] = value;
                    return `(${this._reconstructString(left)} AND ${this._reconstructString(right)})`;
                }
                case 'Or': {
                    const [left, right] = value;
                    return `(${this._reconstructString(left)} OR ${this._reconstructString(right)})`;
                }
                case 'Not': {
                    return `NOT (${this._reconstructString(value)})`;
                }
                case 'Contains': {
                    const [left, right] = value;
                    return `${this._reconstructString(left)} CONTAINS ${this._reconstructString(right)}`;
                }
                case 'StartsWith': {
                    const [left, right] = value;
                    return `${this._reconstructString(left)} STARTS_WITH ${this._reconstructString(right)}`;
                }
                case 'EndsWith': {
                    const [left, right] = value;
                    return `${this._reconstructString(left)} ENDS_WITH ${this._reconstructString(right)}`;
                }
                case 'Like': {
                    const [left, right] = value;
                    return `${this._reconstructString(left)} LIKE ${this._reconstructString(right)}`;
                }
                case 'Between': {
                    const [field, low, high] = value;
                    return `${this._reconstructString(field)} BETWEEN ${this._reconstructString(low)} AND ${this._reconstructString(high)}`;
                }
                case 'In': {
                    const [field, values] = value;
                    return `${this._reconstructString(field)} IN ${this._reconstructString(values)}`;
                }
                case 'NotIn': {
                    const [field, values] = value;
                    return `${this._reconstructString(field)} NOT IN ${this._reconstructString(values)}`;
                }
                case 'Any': {
                    const [field, val] = value;
                    return `ANY(${this._reconstructString(field)}, ${this._reconstructString(val)})`;
                }
                case 'All': {
                    const [field, values] = value;
                    return `ALL(${this._reconstructString(field)}, ${this._reconstructString(values)})`;
                }
                case 'None': {
                    const [field, values] = value;
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
    _escapeString(s) {
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
    parse(query) {
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
    tryParse(query) {
        const result = try_parse_filter_js(query);
        if (result === null)
            return null;
        return new FilterImpl(result, query);
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
    validate(query) {
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
    eq(field, value) {
        const query = `${field} = ${Filter._valueToString(value)}`;
        return Filter.parse(query);
    },
    /**
     * Create inequality filter: field != value
     *
     * @example Filter.ne('status', 'deleted')
     */
    ne(field, value) {
        const query = `${field} != ${Filter._valueToString(value)}`;
        return Filter.parse(query);
    },
    /**
     * Create less-than filter: field < value
     *
     * @example Filter.lt('price', 1000)
     */
    lt(field, value) {
        return Filter.parse(`${field} < ${value}`);
    },
    /**
     * Create less-than-or-equal filter: field <= value
     *
     * @example Filter.le('price', 1000)
     */
    le(field, value) {
        return Filter.parse(`${field} <= ${value}`);
    },
    /**
     * Create greater-than filter: field > value
     *
     * @example Filter.gt('rating', 4.0)
     */
    gt(field, value) {
        return Filter.parse(`${field} > ${value}`);
    },
    /**
     * Create greater-than-or-equal filter: field >= value
     *
     * @example Filter.ge('stock', 10)
     */
    ge(field, value) {
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
    between(field, low, high) {
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
    contains(field, substring) {
        return Filter.parse(`${field} CONTAINS "${Filter._escapeString(substring)}"`);
    },
    /**
     * Create starts-with filter: field STARTS_WITH prefix
     *
     * @example Filter.startsWith('sku', 'GPU-')
     */
    startsWith(field, prefix) {
        return Filter.parse(`${field} STARTS_WITH "${Filter._escapeString(prefix)}"`);
    },
    /**
     * Create ends-with filter: field ENDS_WITH suffix
     *
     * @example Filter.endsWith('filename', '.pdf')
     */
    endsWith(field, suffix) {
        return Filter.parse(`${field} ENDS_WITH "${Filter._escapeString(suffix)}"`);
    },
    /**
     * Create pattern match filter: field LIKE pattern
     *
     * @example Filter.like('email', '%@company.com')
     */
    like(field, pattern) {
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
    in(field, values) {
        return Filter.parse(`${field} IN ${Filter._arrayToString(values)}`);
    },
    /**
     * Create NOT IN filter: field NOT IN [values]
     *
     * @example Filter.notIn('status', ['deleted', 'archived'])
     */
    notIn(field, values) {
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
    any(field, value) {
        return Filter.parse(`ANY(${field}, ${Filter._valueToString(value)})`);
    },
    /**
     * Create ALL filter: ALL(field, values)
     *
     * @example Filter.allOf('tags', ['gpu', 'gaming'])
     */
    allOf(field, values) {
        return Filter.parse(`ALL(${field}, ${Filter._arrayToString(values)})`);
    },
    /**
     * Create NONE filter: NONE(field, values)
     *
     * @example Filter.none('tags', ['nsfw', 'spam'])
     */
    none(field, values) {
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
    isNull(field) {
        return Filter.parse(`${field} IS NULL`);
    },
    /**
     * Create IS NOT NULL filter
     *
     * @example Filter.isNotNull('verifiedAt')
     */
    isNotNull(field) {
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
    and(...filters) {
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
    or(...filters) {
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
    not(filter) {
        return Filter.parse(`NOT (${filter.toString()})`);
    },
    // ===========================================================================
    // SPECIAL FILTERS
    // ===========================================================================
    /** Filter that matches all vectors (no filtering) */
    get matchAll() {
        return new FilterImpl('{"LiteralBool":true}', 'true');
    },
    /** Filter that matches no vectors (empty result) */
    get nothing() {
        return new FilterImpl('{"LiteralBool":false}', 'false');
    },
    // ===========================================================================
    // INTERNAL HELPERS
    // ===========================================================================
    _valueToString(value) {
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
    _arrayToString(values) {
        const items = values.map((v) => Filter._valueToString(v)).join(', ');
        return `[${items}]`;
    },
    _escapeString(s) {
        return s.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
    },
};
export default Filter;
//# sourceMappingURL=filter.js.map