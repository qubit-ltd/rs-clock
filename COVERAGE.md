# Code Coverage Testing Guide

This project uses `cargo-llvm-cov` for code coverage statistics.

## Install Dependencies

If you haven't installed `cargo-llvm-cov` yet, please install it first:

```bash
cargo install cargo-llvm-cov
```

## Quick Start

### Using Convenience Script (Recommended)

We provide a convenience script `coverage.sh` that can quickly generate coverage reports in various formats:

```bash
# Generate HTML report and open in browser (default)
./coverage.sh

# Or specify format
./coverage.sh html       # HTML report (opens in browser)
./coverage.sh text       # Terminal text report
./coverage.sh lcov       # LCOV format
./coverage.sh json       # JSON format
./coverage.sh cobertura  # Cobertura XML format
./coverage.sh all        # Generate all formats

# View help
./coverage.sh help
```

### Using cargo Commands

You can also use `cargo llvm-cov` commands directly:

```bash
# Clean old coverage data
cargo llvm-cov clean

# Generate HTML report and open in browser
cargo llvm-cov --html --open

# Generate text format report (output to terminal)
cargo llvm-cov

# Generate LCOV format report
cargo llvm-cov --lcov --output-path target/llvm-cov/lcov.info

# Generate JSON format report
cargo llvm-cov --json --output-path target/llvm-cov/coverage.json

# Generate Cobertura XML format report
cargo llvm-cov --cobertura --output-path target/llvm-cov/cobertura.xml
```

## Report Locations

Generated reports are saved in the following locations by default:

- **HTML Report**: `target/llvm-cov/html/index.html`
- **LCOV Report**: `target/llvm-cov/lcov.info`
- **JSON Report**: `target/llvm-cov/coverage.json`
- **Cobertura Report**: `target/llvm-cov/cobertura.xml`

## Scope and Thresholds

The repository wrapper measures all features by default and enforces thresholds
for every Rust source file below `src/`:

- functions: at least 100%
- lines: greater than 95%
- regions: greater than 95%

Override variables such as `MIN_FUNCTION_COVERAGE`, `MIN_LINE_COVERAGE`, and
`MIN_REGION_COVERAGE` only when intentionally diagnosing a local report. CI uses
the defaults from `.rs-ci/coverage.sh`.

The `.llvm-cov.toml` configuration excludes non-production report inputs:

- `tests/*` - Test files
- `benches/*` - Benchmark files
- `examples/*` - Example files

If you need to modify exclusion rules, please edit the `.llvm-cov.toml` file.

## CI Integration

Run the repository CI wrapper for the complete check sequence:

```bash
./ci-check.sh
```

For a coverage-only diagnostic, run `./coverage.sh json`. The JSON mode applies
the same per-source thresholds used by CI.

## Common Issues

### 1. Cannot find `cargo-llvm-cov` command

Make sure you have installed `cargo-llvm-cov`:

```bash
cargo install cargo-llvm-cov
```

### 2. Coverage data is inaccurate

Clean old coverage data first:

```bash
cargo llvm-cov clean
```

### 3. How to improve coverage?

- Write tests for all public APIs
- Test boundary conditions and exception cases
- Use coverage reports to identify untested code paths
- Write tests for complex logic branches

## Coverage Requirements

The enforced defaults are 100% function coverage and greater than 95% line and
region coverage for each production source file. Aggregate percentages do not
override a source file that falls below one of these thresholds.

## References

- [cargo-llvm-cov GitHub](https://github.com/taiki-e/cargo-llvm-cov)
- [LLVM Coverage Mapping](https://llvm.org/docs/CoverageMappingFormat.html)
