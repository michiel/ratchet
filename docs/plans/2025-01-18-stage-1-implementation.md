# Stage 1 Implementation Plan: Foundation Improvements

**Timeline**: Weeks 1-4  
**Goal**: Establish patterns and infrastructure without changing functionality  
**Risk Level**: Low - No functional changes, only additions  

## Week 1: Test Infrastructure Standardization

### Day 1-2: Test Organization RFC

Create `/docs/rfcs/001-test-organization.md`:

```markdown
# RFC 001: Test Organization Standards

## Conventions
1. Unit tests: In `src/` next to code under test
2. Integration tests: In `tests/` at crate root
3. Shared test utilities: In `tests/common/mod.rs`
4. Test data builders: In `tests/builders/`

## Naming
- Test modules: `#[cfg(test)] mod tests`
- Test functions: `test_<component>_<scenario>_<expected_outcome>`
- Test fixtures: `fixture_<name>`
- Builders: `<Entity>Builder`

## Organization Example:
```
ratchet-storage/
├── src/
│   ├── repositories/
│   │   ├── task.rs
│   │   └── task_tests.rs    # Unit tests
├── tests/
│   ├── common/
│   │   ├── mod.rs           # Shared utilities
│   │   └── database.rs      # Test database setup
│   ├── builders/
│   │   └── task_builder.rs  # Test data builders
│   └── integration/
│       └── task_repository_test.rs
```

### Day 3-4: Create ratchet-test-utils Crate

```toml
# ratchet-test-utils/Cargo.toml
[package]
name = "ratchet-test-utils"
version = "0.1.0"

[dependencies]
mockall = { workspace = true }
rstest = "0.18"
fake = "2.9"
arbitrary = "1.3"
proptest = { workspace = true }
test-case = "3.3"

# Re-export test frameworks
[features]
default = ["builders", "fixtures", "mocks"]
builders = []
fixtures = []
mocks = ["mockall"]
```

Core utilities to implement:
```rust
// Test database management
pub struct TestDatabase { ... }
impl TestDatabase {
    pub async fn new() -> Self { ... }
    pub async fn with_migrations() -> Self { ... }
    pub async fn cleanup(self) { ... }
}

// Test data builders
#[derive(Default)]
pub struct TaskBuilder {
    id: Option<Uuid>,
    name: Option<String>,
    // ...
}

impl TaskBuilder {
    pub fn with_name(mut self, name: &str) -> Self { ... }
    pub fn build(self) -> Task { ... }
}

// Common assertions
pub mod assertions {
    pub fn assert_error_type<T, E>(result: Result<T, E>, expected: ErrorType) { ... }
    pub fn assert_eventual_consistency<F, T>(f: F, expected: T, timeout: Duration) { ... }
}
```

### Day 5: Mock Generation

Create comprehensive mocks for all interfaces:

```rust
// ratchet-test-utils/src/mocks.rs
use mockall::automock;

#[automock]
pub trait TaskRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Task>>;
    async fn create(&self, task: Task) -> Result<Task>;
    // ... all methods
}

// Generate mocks for all interfaces in ratchet-interfaces
include!("generate_mocks.rs");
```

## Week 2: Documentation Enhancement

### Day 1-2: Inline Documentation Audit

Script to identify undocumented items:
```bash
#!/bin/bash
# tools/doc-coverage.sh
cargo doc --no-deps --document-private-items 2>&1 | \
  grep -E "warning:.*document" | \
  sort | uniq -c | sort -nr
```

Priority documentation targets:
1. All public APIs in `ratchet-interfaces`
2. Complex business logic in `ratchet-core`
3. Configuration options in `ratchet-config`
4. Error types and their meanings

### Day 3: Architectural Decision Records

Create `/docs/adrs/` directory with initial ADRs:

```markdown
# ADR-001: Use Interface Segregation for Repositories

## Status
Accepted

## Context
The original `DatabaseRepository` trait contained 25+ methods covering all entity types.

## Decision
Split into focused repository traits: `TaskRepository`, `ExecutionRepository`, etc.

## Consequences
- ✅ Easier to mock for testing
- ✅ Clear dependencies for each component
- ✅ Follows Interface Segregation Principle
- ⚠️ More traits to manage
- ⚠️ Potential for duplication

## References
- Issue #123
- PR #456
```

### Day 4-5: API Documentation with Examples

For each public API, add comprehensive examples:

```rust
/// Creates a new task in the system.
/// 
/// # Arguments
/// * `input` - Task creation parameters
/// 
/// # Returns
/// The created task with generated ID and timestamps
/// 
/// # Errors
/// * `ValidationError` - If input validation fails
/// * `DuplicateError` - If task name already exists
/// * `DatabaseError` - If database operation fails
/// 
/// # Example
/// ```
/// use ratchet_api::create_task;
/// use ratchet_types::{CreateTaskInput, TaskType};
/// 
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let client = ApiClient::new("http://localhost:8080")?;
///     
///     let input = CreateTaskInput {
///         name: "data-processor".to_string(),
///         task_type: TaskType::JavaScript,
///         script: "export default function(input) { return input; }".to_string(),
///         description: Some("Processes incoming data".to_string()),
///         ..Default::default()
///     };
///     
///     let task = client.create_task(input).await?;
///     println!("Created task: {}", task.id);
///     Ok(())
/// }
/// ```
/// 
/// # Integration Example
/// ```
/// // Using with job scheduling
/// let task = client.create_task(input).await?;
/// let schedule = client.create_schedule(
///     CreateScheduleInput {
///         task_id: task.id,
///         cron: "0 0 * * *".to_string(),
///         ..Default::default()
///     }
/// ).await?;
/// ```
pub async fn create_task(input: CreateTaskInput) -> Result<Task> {
    // ...
}
```

## Week 3: Development Tools

### Day 1-2: Code Complexity Metrics

Set up complexity monitoring:

```toml
# .cargo/config.toml
[alias]
complexity = "run --bin complexity-check"
```

```rust
// tools/complexity-check/src/main.rs
use syn::{visit::Visit, File};
use std::fs;

struct ComplexityVisitor {
    current_function: Option<String>,
    complexity: usize,
    violations: Vec<Violation>,
}

impl Visit<'_> for ComplexityVisitor {
    fn visit_item_fn(&mut self, node: &syn::ItemFn) {
        // Calculate cyclomatic complexity
        // Check line count
        // Record violations
    }
}

fn main() {
    // Scan all Rust files
    // Report complexity violations
    // Exit with error if thresholds exceeded
}
```

### Day 3: Pre-commit Hooks

`.pre-commit-config.yaml`:
```yaml
repos:
  - repo: local
    hooks:
      - id: rust-linting
        name: Rust linting
        entry: cargo clippy -- -D warnings
        language: system
        files: '\.rs$'
        pass_filenames: false
      
      - id: rust-formatting
        name: Rust formatting
        entry: cargo fmt -- --check
        language: system
        files: '\.rs$'
        pass_filenames: false
      
      - id: complexity-check
        name: Complexity check
        entry: cargo complexity
        language: system
        files: '\.rs$'
        pass_filenames: false
      
      - id: test-organization
        name: Test organization check
        entry: ./tools/check-test-organization.sh
        language: script
        files: 'tests/.*\.rs$'
```

### Day 4: Module Dependency Visualization

Create visualization tool:
```rust
// tools/dep-graph/src/main.rs
use cargo_metadata::{MetadataCommand, Package};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::dot::{Dot, Config};

fn main() -> Result<()> {
    let metadata = MetadataCommand::new().exec()?;
    let mut graph = DiGraph::new();
    
    // Build dependency graph
    let nodes: HashMap<&str, NodeIndex> = metadata.packages
        .iter()
        .filter(|p| p.source.is_none()) // Only workspace members
        .map(|p| (p.name.as_str(), graph.add_node(&p.name)))
        .collect();
    
    // Add edges for dependencies
    for package in &metadata.packages {
        if let Some(&node) = nodes.get(package.name.as_str()) {
            for dep in &package.dependencies {
                if let Some(&dep_node) = nodes.get(dep.name.as_str()) {
                    graph.add_edge(node, dep_node, ());
                }
            }
        }
    }
    
    // Generate visualization
    println!("{:?}", Dot::with_config(&graph, &[Config::EdgeNoLabel]));
    Ok(())
}
```

### Day 5: Performance Benchmarking Framework

Set up criterion benchmarks:
```rust
// benches/task_execution.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ratchet_execution::TaskExecutor;

fn benchmark_task_execution(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let executor = runtime.block_on(create_test_executor());
    
    c.bench_function("simple_task_execution", |b| {
        b.to_async(&runtime).iter(|| async {
            let task = create_simple_task();
            let input = json!({"value": 42});
            let result = executor.execute(&task, input).await.unwrap();
            black_box(result);
        });
    });
    
    c.bench_function("complex_task_execution", |b| {
        b.to_async(&runtime).iter(|| async {
            let task = create_complex_task();
            let input = create_large_input();
            let result = executor.execute(&task, input).await.unwrap();
            black_box(result);
        });
    });
}

criterion_group!(benches, benchmark_task_execution);
criterion_main!(benches);
```

## Week 4: Integration and Rollout

### Day 1-2: CI/CD Integration

Update `.github/workflows/ci.yml`:
```yaml
name: CI
on: [push, pull_request]

jobs:
  complexity:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - name: Check complexity
        run: cargo complexity
      
  test-organization:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Check test organization
        run: ./tools/check-test-organization.sh
      
  documentation:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - name: Check documentation
        run: cargo doc --no-deps --document-private-items
      - name: Documentation coverage
        run: ./tools/doc-coverage.sh
      
  benchmarks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - name: Run benchmarks
        run: cargo bench -- --output-format bencher | tee output.txt
      - name: Store benchmark result
        uses: benchmark-action/github-action-benchmark@v1
```

### Day 3-4: Team Training

Create training materials:
1. **Test Writing Guide** (`docs/guides/writing-tests.md`)
2. **Documentation Standards** (`docs/guides/documentation.md`)
3. **Complexity Guidelines** (`docs/guides/code-complexity.md`)
4. **Performance Testing** (`docs/guides/performance-testing.md`)

Conduct training sessions:
- 2-hour workshop on new test utilities
- 1-hour session on documentation standards
- 1-hour session on using development tools

### Day 5: Metrics Baseline

Establish baseline metrics:
```bash
#!/bin/bash
# tools/metrics-baseline.sh

echo "=== Ratchet Metrics Baseline ==="
echo "Date: $(date)"
echo ""

echo "=== Build Metrics ==="
time cargo build --workspace --release

echo "=== Test Metrics ==="
time cargo test --workspace

echo "=== Code Metrics ==="
echo "Total lines of code: $(find . -name "*.rs" | xargs wc -l | tail -1)"
echo "Number of crates: $(ls -d */ | grep -E "^ratchet-" | wc -l)"
echo "Test files: $(find . -name "*test*.rs" | wc -l)"

echo "=== Complexity Metrics ==="
cargo complexity --summary

echo "=== Documentation Coverage ==="
./tools/doc-coverage.sh | tail -10
```

## Success Criteria

By the end of Week 4:
1. ✅ All developers using new test utilities
2. ✅ Zero failing complexity checks in CI
3. ✅ Documentation coverage > 60%
4. ✅ Performance baselines established
5. ✅ Team trained on new tools and patterns

## Next Steps

With Stage 1 complete, the team will have:
- Clear patterns for testing and documentation
- Tools to maintain code quality
- Metrics to track improvements
- Foundation for Stage 2 structural changes

The infrastructure established in Stage 1 ensures that subsequent stages can proceed with confidence, knowing that quality gates are in place and the team is aligned on best practices.