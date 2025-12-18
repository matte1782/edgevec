# Week 24 Day 4: UX - Filter Playground

**Date:** TBD
**Focus:** Create interactive filter demo for maximum user experience
**Estimated Duration:** 8 hours

---

## Tasks

### W24.4.1: Filter Playground HTML Scaffold

**Objective:** Create the base HTML/CSS structure for filter playground.

**Acceptance Criteria:**
- [ ] Responsive layout (mobile-first)
- [ ] Dark/light theme toggle
- [ ] Consistent with existing demo style (cyberpunk theme)
- [ ] Accessible (WCAG 2.1 AA)
- [ ] Input area for filter expression
- [ ] Output area for parse result
- [ ] Example buttons section

**Deliverables:**
- `wasm/examples/filter-playground.html`
- `wasm/examples/filter-playground.css`

**Dependencies:** None

**Estimated Duration:** 2.5 hours

**Agent:** WASM_SPECIALIST

**Layout Wireframe:**
```
┌─────────────────────────────────────────────────────────────┐
│  EdgeVec Filter Playground                          [🌙/☀️] │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Enter your filter expression:                              │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ category = "electronics" AND price < 100                ││
│  └─────────────────────────────────────────────────────────┘│
│                                                             │
│  Status: ✅ Valid                              [Parse] [Clear]│
│                                                             │
│  Parsed AST:                                                │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ {                                                       ││
│  │   "type": "And",                                        ││
│  │   "children": [...]                                     ││
│  │ }                                                       ││
│  └─────────────────────────────────────────────────────────┘│
│                                                             │
│  Examples:                                                  │
│  [Simple] [Range] [Boolean] [IN/NOT IN] [Complex] [Nested]  │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  Try with sample data:                                      │
│  [Load Sample Data]  Vectors: 1000  [Run Filtered Search]   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**CSS Requirements:**
- CSS variables for theming
- Smooth transitions
- Focus indicators (accessibility)
- Monospace font for code areas
- Syntax highlighting compatible

---

### W24.4.2: Live Parsing Engine

**Objective:** Implement real-time filter parsing with feedback.

**Acceptance Criteria:**
- [ ] Parse on keystroke (debounced 150ms)
- [ ] Show valid/invalid status immediately
- [ ] Display parsed AST as JSON
- [ ] Highlight syntax errors with position
- [ ] <100ms parse response time

**Deliverables:**
- `wasm/examples/filter-playground.js`

**Dependencies:** W24.4.1

**Estimated Duration:** 2 hours

**Agent:** WASM_SPECIALIST

**Implementation:**
```javascript
// Debounced parsing
let parseTimeout;
filterInput.addEventListener('input', (e) => {
    clearTimeout(parseTimeout);
    parseTimeout = setTimeout(() => {
        try {
            const ast = edgevec.parseFilter(e.target.value);
            showSuccess(ast);
        } catch (error) {
            showError(error);
        }
    }, 150);
});

function showSuccess(ast) {
    statusEl.innerHTML = '✅ Valid';
    statusEl.className = 'status-valid';
    astOutput.textContent = JSON.stringify(ast, null, 2);
}

function showError(error) {
    statusEl.innerHTML = `❌ ${error.message}`;
    statusEl.className = 'status-error';
    // Highlight error position in input
    highlightError(error.position);
}
```

---

### W24.4.3: Example Templates Library

**Objective:** Provide clickable example templates for learning.

**Acceptance Criteria:**
- [ ] 8+ example templates
- [ ] Categories: Simple, Range, Boolean, Complex
- [ ] Click to load into input
- [ ] Description tooltip on hover
- [ ] Progressive complexity

**Deliverables:**
- Examples integrated into `filter-playground.js`

**Dependencies:** W24.4.1

**Estimated Duration:** 1.5 hours

**Agent:** WASM_SPECIALIST

**Example Templates:**
```javascript
const examples = [
    {
        name: "Simple Equals",
        filter: 'category = "electronics"',
        description: "Match exact category value"
    },
    {
        name: "Range",
        filter: 'price >= 10 AND price <= 100',
        description: "Price in range [10, 100]"
    },
    {
        name: "BETWEEN",
        filter: 'price BETWEEN 10 AND 100',
        description: "Equivalent to range, cleaner syntax"
    },
    {
        name: "IN List",
        filter: 'status IN ("active", "pending")',
        description: "Match any value in list"
    },
    {
        name: "Boolean AND",
        filter: 'category = "books" AND price < 20',
        description: "Both conditions must be true"
    },
    {
        name: "Boolean OR",
        filter: 'category = "books" OR category = "movies"',
        description: "Either condition can be true"
    },
    {
        name: "NOT",
        filter: 'NOT (status = "deleted")',
        description: "Negate a condition"
    },
    {
        name: "Complex Nested",
        filter: '(category = "electronics" AND price < 500) OR (category = "books" AND rating > 4)',
        description: "Nested boolean logic"
    }
];
```

---

### W24.4.4: Error Feedback System

**Objective:** Provide helpful error messages with fix suggestions.

**Acceptance Criteria:**
- [ ] Error position highlighted in input
- [ ] Specific error messages (not generic)
- [ ] "Did you mean?" suggestions
- [ ] Link to documentation
- [ ] Keyboard accessible error navigation

**Deliverables:**
- Error handling in `filter-playground.js`

**Dependencies:** W24.4.2

**Estimated Duration:** 2 hours

**Agent:** WASM_SPECIALIST

**Error UX:**
```
Input: category = electronics

Error Display:
┌─────────────────────────────────────────────────────────────┐
│ ❌ Syntax Error at position 12                              │
│                                                             │
│ category = electronics                                      │
│            ^^^^^^^^^^^                                      │
│                                                             │
│ Expected: Quoted string value                               │
│ Did you mean: category = "electronics"                      │
│                                                             │
│ [Copy Fix] [View Docs]                                      │
└─────────────────────────────────────────────────────────────┘
```

**Common Fixes:**
- Unquoted strings → Add quotes
- Missing operator → Suggest AND/OR
- Unclosed parenthesis → Show where to close
- Unknown field → List available fields

---

## Day 4 Checklist

- [ ] W24.4.1: HTML scaffold complete
- [ ] W24.4.2: Live parsing working
- [ ] W24.4.3: Example templates clickable
- [ ] W24.4.4: Error feedback helpful

## Day 4 Exit Criteria

- Filter playground loads in <2s
- All examples work correctly
- Errors show helpful messages
- Mobile responsive
- Theme toggle works
- No console errors

## Design Notes

**Color Palette (Dark Theme):**
- Background: #0a0a0f
- Surface: #1a1a2e
- Primary: #00d9ff (cyan)
- Success: #00ff88
- Error: #ff4466
- Text: #e0e0e0

**Typography:**
- Headers: Inter or system-ui
- Code: JetBrains Mono or monospace
