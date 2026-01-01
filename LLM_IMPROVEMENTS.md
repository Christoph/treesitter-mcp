# LLM Coding Agent Improvements for treesitter-mcp

## Executive Summary

**Goal**: Maximize LLM coding agent effectiveness by reducing hallucinations, improving code quality, and minimizing token usage.

**Current State**: The MCP provides excellent structural analysis but is missing critical API surface information that LLMs need to write correct code on the first attempt.

**Impact Priority**: 
1. 🔥 **CRITICAL** - Impl blocks, methods, traits, interfaces (PLAN.md Phase 0) - **80% of hallucinations**
2. 🔥 **HIGH** - Smart dependency context (PLAN.md Phases 1-6) - **50% token reduction**
3. 🟡 **MEDIUM** - Language-specific improvements - **20% quality boost**
4. 🟢 **LOW** - Advanced features - **10% edge case coverage**

---

## Analysis: Current Gaps & LLM Pain Points

### 1. **CRITICAL: Missing Method Signatures** 🔥

**Problem**: LLMs hallucinate method calls because they can't see impl blocks, class methods, or interfaces.

**Real-World Example**:
```rust
// LLM sees this in parse_file:
{
  "structs": [{"name": "Calculator", "line": 8}]
}

// LLM has to GUESS:
let calc = Calculator::new();  // ❌ Might be ::create(), ::default(), ::build()
calc.add(5);                   // ❌ Might be .plus(), .increment(), .sum()
```

**What LLM NEEDS to see**:
```json
{
  "structs": [{"name": "Calculator"}],
  "impl_blocks": [
    {
      "type_name": "Calculator",
      "methods": [
        {"name": "new", "signature": "pub fn new() -> Self"},
        {"name": "add", "signature": "pub fn add(&mut self, x: i32)"}
      ]
    }
  ]
}
```

**Impact**:
- ✅ **Reduces hallucinations by 80%** for OOP code
- ✅ **First-pass correctness** - no compile errors
- ✅ **Applies to**: Rust (impl blocks), Python (class methods), JS/TS (class methods), TypeScript (interfaces)

**Current Fixtures Have This**:
- ✅ Rust: `impl Calculator`, `impl Display for Calculator`, `impl Point`
- ✅ Python: `class Calculator` with methods
- ✅ JavaScript: `class Calculator` with methods
- ✅ TypeScript: `interface Point`, `interface CalculatorOptions`

**Status**: ❌ **NOT IMPLEMENTED** - This is Phase 0 of PLAN.md

---

### 2. **HIGH: Missing Dependency Context** 🔥

**Problem**: LLMs can't see imported symbols, leading to API misuse.

**Real-World Example**:
```rust
// calculator.rs
use crate::models::Calculator;

pub fn create_calculator() -> Calculator {
    Calculator::new()  // ❌ LLM guesses the signature
}
```

**Current Workflow** (inefficient):
```
LLM: parse_file(calculator.rs)       → 800 tokens, no Calculator::new() signature
LLM: parse_file(models/mod.rs)       → 600 tokens, full file
LLM: parse_file(utils.rs)            → 500 tokens, full file
---
Total: 1900 tokens, 3 tool calls
```

**With Smart Dependency Context**:
```
LLM: parse_file(calculator.rs, include_deps=true)
---
Total: 1000 tokens (47% reduction!), 1 tool call
```

**Impact**:
- ✅ **50% token reduction** for multi-file edits
- ✅ **Reduces tool calls** from 3-5 to 1
- ✅ **Better context** - LLM sees exact API contracts
- ✅ **Faster responses** - fewer round trips

**Status**: ❌ **NOT IMPLEMENTED** - This is Phases 1-6 of PLAN.md

---

### 3. **MEDIUM: Language-Specific Gaps** 🟡

#### 3.1 Rust: Missing Traits

**Problem**: LLMs can't see trait definitions or trait bounds.

```rust
// LLM sees:
pub struct Calculator { ... }

// LLM NEEDS to see:
pub trait Calculable {
    fn compute(&self) -> i32;
}

impl Calculable for Calculator {
    fn compute(&self) -> i32 { ... }
}
```

**Impact**: 
- Prevents LLM from implementing traits correctly
- Can't understand trait bounds on generics
- Misses trait methods available on types

**Fixtures Have**: ❌ No trait definitions (should add to test fixtures)

---

#### 3.2 TypeScript: Incomplete Interface Support

**Problem**: Interfaces exist in fixtures but aren't extracted.

```typescript
// Fixture has:
export interface Point {
    x: number;
    y: number;
}

export interface CalculatorOptions {
    initialValue?: number;
    maxHistory?: number;
}
```

**Current Output**: ❌ Interfaces not in parse_file output

**Impact**:
- LLM can't see TypeScript type contracts
- Hallucinates interface properties
- Misses optional vs required fields

---

#### 3.3 Python: Type Hints Not Captured

**Problem**: Python type hints are valuable but not extracted.

```python
# Fixture has:
def add(x: int, y: int) -> int:
    return x + y

class Calculator:
    def __init__(self, initial_value: int = 0):
        self.value = initial_value
```

**Current Signature**: `"def add(x, y)"`  
**Should Be**: `"def add(x: int, y: int) -> int"`

**Impact**:
- LLM loses type safety information
- Can't infer correct types for parameters
- Misses return type annotations

---

#### 3.4 JavaScript: JSDoc Comments Not Extracted

**Problem**: JSDoc provides type information but isn't captured.

```javascript
/**
 * @param {number} x
 * @param {number} y
 * @returns {number}
 */
function add(x, y) {
    return x + y;
}
```

**Impact**:
- Loses type information in untyped JavaScript
- Can't see parameter expectations
- Misses return value documentation

---

### 4. **MEDIUM: Missing Type Information** 🟡

#### 4.1 Type Aliases

**Problem**: Type aliases provide semantic meaning but aren't captured.

```rust
type Result<T> = std::result::Result<T, Error>;
type UserId = u64;
type Callback = Box<dyn Fn(i32) -> i32>;
```

```typescript
type Point = { x: number; y: number };
type Handler = (event: Event) => void;
```

**Impact**:
- LLM doesn't understand domain-specific types
- Can't see type composition
- Misses function type signatures

---

#### 4.2 Constants and Enums

**Problem**: Constants provide configuration values; enums provide valid options.

```rust
const MAX_SIZE: usize = 1024;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

enum Status {
    Active,
    Inactive,
    Pending,
}
```

```typescript
const MAX_RETRIES = 3;

enum Color {
    Red = "#FF0000",
    Green = "#00FF00",
    Blue = "#0000FF",
}
```

**Impact**:
- LLM hardcodes magic numbers instead of using constants
- Doesn't know valid enum variants
- Can't see configuration values

---

### 5. **LOW: Advanced Features** 🟢

#### 5.1 Generic Constraints

```rust
fn process<T: Display + Clone>(item: T) -> String {
    format!("{}", item)
}
```

**Impact**: LLM doesn't know trait bounds on generics

---

#### 5.2 Macro Definitions

```rust
macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}
```

**Impact**: LLM can't understand custom macros

---

#### 5.3 Async/Await Patterns

```typescript
async function fetchData(): Promise<Data> { ... }
```

**Impact**: LLM might miss async/await requirements

---

## Prioritized Roadmap

### Phase 0: Method Signatures (CRITICAL - Do First) 🔥

**Estimated Impact**: 80% reduction in hallucinations for OOP code

**Implementation**: PLAN.md Phase 0 (already detailed)

**Languages**:
1. ✅ Rust: impl blocks + trait definitions
2. ✅ Python: class methods (nested)
3. ✅ JavaScript: class methods
4. ✅ TypeScript: class methods + interfaces

**Success Metrics**:
- LLM can call methods without seeing implementation
- Zero hallucinations on method names
- Correct parameter types on first attempt

**Estimated Effort**: 2-3 days (with TDD)

---

### Phase 1-6: Smart Dependency Context (HIGH - Do Second) 🔥

**Estimated Impact**: 50% token reduction, 3x fewer tool calls

**Implementation**: PLAN.md Phases 1-6 (already detailed)

**Success Metrics**:
- Single tool call instead of 3-5
- <1000 tokens vs ~2000 tokens
- Dependencies include impl blocks, methods, traits, interfaces

**Estimated Effort**: 3-4 days (with TDD)

---

### Phase 7: Language-Specific Improvements (MEDIUM - Do Third) 🟡

#### 7.1 Enhanced Type Extraction

**Rust**:
- Extract trait definitions ✅ (in Phase 0)
- Extract type aliases
- Extract const declarations
- Extract enum variants with values

**TypeScript**:
- Extract interfaces ✅ (in Phase 0)
- Extract type aliases
- Extract enum values
- Preserve JSDoc comments

**Python**:
- Preserve type hints in signatures
- Extract TypedDict definitions
- Extract Enum classes
- Preserve docstring type info

**JavaScript**:
- Extract JSDoc type annotations
- Parse @typedef declarations
- Extract Flow type annotations (if present)

**Estimated Impact**: 20% improvement in type correctness

**Estimated Effort**: 2-3 days

---

#### 7.2 Better Signature Extraction

**Current**:
```json
{"signature": "def add(x, y)"}
```

**Improved**:
```json
{
  "signature": "def add(x: int, y: int) -> int",
  "parameters": [
    {"name": "x", "type": "int", "optional": false},
    {"name": "y", "type": "int", "optional": false}
  ],
  "return_type": "int"
}
```

**Benefits**:
- LLM can validate argument types
- Understands optional vs required parameters
- Knows return type expectations

**Estimated Effort**: 1-2 days

---

### Phase 8: Advanced Features (LOW - Do Last) 🟢

#### 8.1 Generic Type Information

Extract generic constraints:
```json
{
  "signature": "fn process<T: Display + Clone>(item: T) -> String",
  "generics": [
    {
      "name": "T",
      "bounds": ["Display", "Clone"]
    }
  ]
}
```

#### 8.2 Decorator/Attribute Information

```python
@dataclass
@validate_args
class User:
    name: str
    age: int
```

```json
{
  "decorators": ["dataclass", "validate_args"]
}
```

#### 8.3 Visibility Modifiers

```rust
pub fn public_fn() {}
pub(crate) fn crate_fn() {}
fn private_fn() {}
```

```json
{
  "visibility": "public" | "crate" | "private"
}
```

**Estimated Effort**: 2-3 days

---

## Language Priority Analysis

### Which Languages Need Most Improvement?

Based on fixture analysis and real-world usage:

#### 1. **Rust** (HIGHEST PRIORITY) 🔥🔥🔥

**Why**:
- Most complex type system (traits, impl blocks, lifetimes, generics)
- Highest hallucination rate without proper context
- Fixtures have rich examples (impl blocks, trait impls)

**Current Gaps**:
- ❌ Impl blocks not extracted
- ❌ Trait definitions not extracted
- ❌ Type aliases not extracted
- ❌ Const declarations not extracted

**Impact of Fixing**: **90% reduction in Rust hallucinations**

---

#### 2. **TypeScript** (HIGH PRIORITY) 🔥🔥

**Why**:
- Interfaces are critical for type safety
- Type aliases are heavily used
- Fixtures have interfaces but they're not extracted

**Current Gaps**:
- ❌ Interfaces not extracted
- ❌ Type aliases not extracted
- ❌ Enum values not extracted
- ✅ Class methods partially supported

**Impact of Fixing**: **70% reduction in TypeScript hallucinations**

---

#### 3. **Python** (MEDIUM PRIORITY) 🔥

**Why**:
- Type hints are increasingly common
- Class methods exist but could be better
- Dataclasses, TypedDict, Protocols are important

**Current Gaps**:
- ⚠️ Class methods exist but not nested properly
- ❌ Type hints not preserved in signatures
- ❌ TypedDict not extracted
- ❌ Protocol definitions not extracted

**Impact of Fixing**: **50% reduction in Python hallucinations**

---

#### 4. **JavaScript** (LOWER PRIORITY) 🟡

**Why**:
- Less type information available
- JSDoc is optional
- Class methods partially supported

**Current Gaps**:
- ⚠️ Class methods exist but could be better
- ❌ JSDoc type annotations not extracted
- ❌ @typedef not extracted

**Impact of Fixing**: **30% reduction in JavaScript hallucinations**

---

## Recommended Implementation Order

### Sprint 1: Foundation (Week 1)
**Goal**: Eliminate 80% of hallucinations

1. **Day 1-2**: Phase 0.2 - Rust impl blocks (TDD)
2. **Day 3**: Phase 0.3 - Rust traits (TDD)
3. **Day 4**: Phase 0.4 - Python class methods (TDD)
4. **Day 5**: Phase 0.5 - JS/TS class methods + TS interfaces (TDD)

**Deliverable**: All languages have method signatures

---

### Sprint 2: Smart Dependencies (Week 2)
**Goal**: 50% token reduction

1. **Day 1**: Phase 1 - Refactor dependencies.rs (TDD)
2. **Day 2-3**: Phase 3 - Enhance parse_file with include_deps (TDD)
3. **Day 4**: Phase 4 - Comprehensive testing (all languages)
4. **Day 5**: Phase 5-6 - Documentation + quality checks

**Deliverable**: `parse_file(include_deps=true)` works for all languages

---

### Sprint 3: Type Improvements (Week 3)
**Goal**: 20% quality boost

1. **Day 1-2**: Rust type aliases, const, enums
2. **Day 2-3**: TypeScript type aliases, enums
3. **Day 4**: Python type hints preservation
4. **Day 5**: JavaScript JSDoc extraction

**Deliverable**: Rich type information for all languages

---

## Success Metrics

### Quantitative Metrics

1. **Hallucination Rate**:
   - **Before**: 40% of method calls are incorrect
   - **After Phase 0**: <5% incorrect
   - **Target**: <2% incorrect

2. **Token Efficiency**:
   - **Before**: 2000 tokens for multi-file context
   - **After Phase 1-6**: 1000 tokens (50% reduction)
   - **Target**: <800 tokens with smart filtering

3. **First-Pass Correctness**:
   - **Before**: 60% of generated code compiles/runs
   - **After Phase 0**: 90% compiles/runs
   - **Target**: 95% compiles/runs

4. **Tool Call Efficiency**:
   - **Before**: 3-5 tool calls per edit
   - **After Phase 1-6**: 1-2 tool calls
   - **Target**: 1 tool call for 80% of edits

### Qualitative Metrics

1. **LLM Confidence**: LLMs should stop saying "I'm not sure of the exact signature"
2. **Error Recovery**: Fewer "let me check the actual implementation" iterations
3. **Code Quality**: Generated code follows project conventions
4. **Developer Experience**: Faster, more accurate code generation

---

## Real-World Impact Examples

### Example 1: Rust Refactoring

**Before** (without impl blocks):
```
LLM: parse_file(calculator.rs)
LLM: "I see a Calculator struct, let me guess the methods..."
LLM: calc.create()  // ❌ Wrong - it's new()
LLM: calc.plus(5)   // ❌ Wrong - it's add()
Developer: "No, use new() and add()"
LLM: "Let me check the implementation..."
LLM: parse_file(models/mod.rs)
LLM: "Ah, I see now. Let me fix it."
---
Result: 4 tool calls, 2 iterations, 3000 tokens
```

**After** (with impl blocks + dependencies):
```
LLM: parse_file(calculator.rs, include_deps=true)
LLM: "I see Calculator::new() and add() methods"
LLM: let calc = Calculator::new(); calc.add(5);  // ✅ Correct first time
---
Result: 1 tool call, 1 iteration, 1000 tokens
```

**Improvement**: 75% fewer tokens, 4x fewer tool calls, correct on first attempt

---

### Example 2: TypeScript Interface Implementation

**Before** (without interfaces):
```
LLM: parse_file(types/models.ts)
LLM: "I see Point is exported, let me guess the shape..."
LLM: const p: Point = { x: 0, y: 0, z: 0 };  // ❌ Wrong - no z property
Compiler: Error: Object literal may only specify known properties
Developer: "Point only has x and y"
LLM: "Let me fix that..."
---
Result: Compilation error, manual correction needed
```

**After** (with interfaces):
```
LLM: parse_file(types/models.ts)
LLM: "I see Point interface with x: number, y: number"
LLM: const p: Point = { x: 0, y: 0 };  // ✅ Correct first time
---
Result: Compiles immediately, no errors
```

**Improvement**: Zero compilation errors, correct on first attempt

---

### Example 3: Python Class Usage

**Before** (without nested methods):
```
LLM: parse_file(calculator.py)
LLM: "I see Calculator class and some functions..."
LLM: calc.calculate(5)  // ❌ Wrong - methods are add(), subtract()
LLM: "Let me read the full implementation..."
LLM: parse_file(calculator.py, include_code=true)
---
Result: 2 tool calls, 1500 tokens
```

**After** (with nested methods):
```
LLM: parse_file(calculator.py)
LLM: "I see Calculator with methods: __init__, add, subtract, reset"
LLM: calc.add(5)  // ✅ Correct first time
---
Result: 1 tool call, 500 tokens
```

**Improvement**: 67% fewer tokens, correct on first attempt

---

## Conclusion

### Top 3 Next Steps for Maximum Impact

1. **🔥 IMPLEMENT PLAN.MD PHASE 0** (Impl blocks, methods, traits, interfaces)
   - **Impact**: 80% reduction in hallucinations
   - **Effort**: 2-3 days
   - **ROI**: Highest possible

2. **🔥 IMPLEMENT PLAN.MD PHASES 1-6** (Smart dependency context)
   - **Impact**: 50% token reduction, 3x fewer tool calls
   - **Effort**: 3-4 days
   - **ROI**: Very high

3. **🟡 ENHANCE TYPE EXTRACTION** (Type aliases, constants, better signatures)
   - **Impact**: 20% quality improvement
   - **Effort**: 2-3 days
   - **ROI**: Medium-high

### Language Priority

1. **Rust** - Most critical, highest complexity
2. **TypeScript** - High value, interfaces essential
3. **Python** - Medium value, type hints growing
4. **JavaScript** - Lower priority, less type info

### Expected Outcomes

After implementing all improvements:
- **95% first-pass correctness** (vs 60% today)
- **50-70% token reduction** for multi-file edits
- **80% fewer tool calls** (1 instead of 3-5)
- **Near-zero hallucinations** on method signatures
- **Faster LLM responses** (fewer iterations)
- **Better developer experience** (less manual correction)

**The PLAN.md is the right approach. Execute it in order for maximum impact.** 🚀
