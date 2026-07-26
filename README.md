###### *<div align="right"><sub>// by RuztyCrabs</sub></div>*

<img src="https://raw.githubusercontent.com/RuztyCrabs/Blazelint/refs/heads/main/docs/assets/blazelint-banner-2.webp" alt="BlazeLint banner" style="width: 2000px; height: auto;">

##

> [!WARNING]
> This is a **University Research Project** and **SHOULD NOT BE USED ON PRODUCTION ENVIRONMENTS**. The goal is to determine the feasibility, performance and developer experience of Rust Programming Language for implementing static code analyzers for Ballerina Language.

## Table of Contents

- [Benchmarks](#benchmarks)
- [Documentation](#documentation)
- [Installation](#installation)
- [Usage](#usage)
- [Configuration](#configuration)
- [Development environment](#development-environment)
	- [Using GitHub Codespaces](#using-github-codespaces)
	- [Using VS Code ](#using-vs-code-if-you-have-it-installed-locally)
- [Building](#building)
	- [Prerequisites ](#prerequisites-skip-if-using-the-dev-container)
	- [Steps](#steps)
- [Debugging](#debugging)
	- [Prerequisites ](#prerequisites-skip-if-using-the-dev-container-1)
	- [Steps](#steps-1)
- [Contributing](#contributing)
- [TODO](#todo)
- [License](#license)

## Benchmarks

### Versus the official `bal scan`

Both tools run **the same two rules** — `ballerina:1` (avoid `checkpanic`) and
`ballerina:2` (unused function parameter), which is the complete rule set of
`bal scan` 0.5.0 — over a 40-file package, and **report identical findings**.
Every other Blazelint rule is disabled so the comparison is like-for-like.
Median of 3 runs; reproduce with `bash scripts/benchmark_vs_scan.sh`.

| Tool | Time | vs Blazelint |
|---|---:|---|
| **`bal scan`** (total) | 1,703 ms | 30× slower |
| &nbsp;&nbsp;↳ JVM startup | 966 ms | — |
| &nbsp;&nbsp;↳ analysis only | 737 ms | 13× slower |
| **Blazelint** (40 files) | **56 ms** | baseline |

Note that **57% of `bal scan`'s runtime is JVM startup**, paid on every
invocation — a structural cost that matters most for editor integration, where
the tool runs constantly. The conservative, analysis-only figure is **13×**.

### Internal Performance Breakdown

Per file, from `blazelint --detailed-timing` (~103 µs on a small module):

| Stage | Percentage of Total |
|--------------------|---------------------|
| Parsing            | 65% |
| Linting Rules      | 13% |
| Semantic Analysis  | 12% |
| Lexical Analysis   | 10% |

### Notes on Benchmark Context

- **Grammar Coverage**: Blazelint parses **93% of the syntactic grammar** of the [Ballerina 2024R1 specification](https://ballerina.io/spec/lang/2024R1/) into structured AST (391 of 419 productions). All 419 are *accepted* — no construct in the spec causes a parse error — but 28 (object *type* bodies, annotation declarations, `fork` bodies, regex templates) are consumed as opaque blocks rather than decomposed, since no lint rule inspects their internals. On real code it parses **99% of the official "Ballerina by Example" corpus** (162/163 sampled files) without grammar errors. Full breakdown and caveats in [`docs/GRAMMAR_COVERAGE.md`](docs/GRAMMAR_COVERAGE.md); reproduce with `cargo test --lib grammar_coverage_of_official_spec` and `bash scripts/grammar_coverage.sh`.

- **Validated against the official compiler**: `scripts/compare_with_ballerina.sh` runs the official Ballerina 2201.10.0 (Swan Lake, language spec 2024R1) compiler over the same corpus and compares verdicts. On the sampled files Blazelint produced **zero false positives** — it never rejected a program the official compiler accepts. The compiler does report errors Blazelint does not; those are cross-file/generated-symbol references and deep type-checking, both outside the scope of a single-file linter.

- **What the benchmark does and does not claim**: both tools type-check, but Blazelint's pass is shallower — on a four-error sample the compiler caught all four and Blazelint three, missing a record *field* type. Field types, lang-library method signatures, and cross-file symbols resolve to `Unknown`. Type checking is ~12% of Blazelint's runtime, so it is real work rather than a step being skipped.

  Neither benchmarked rule needs type information at all: detecting `checkpanic` is syntactic, and unused-parameter is scope-based. `bal scan` compiles the whole package regardless, because it runs as a compiler plugin — an architectural cost of that design, and equally the reason its other rules get type information for free.

  Semantic analysis is deliberately shallower than the parser: new constructs are *parse-tolerant*, accepted into the AST with deep type-checking deferred, so the linter does not reject valid programs.
- **Lexer Scalability**: The lexer uses a switch-case dispatch mechanism, ensuring constant time complexity per character. Adding new lexemes will not significantly impact performance.
- **Parser Scalability**: Uses a recursive descent parser is designed for modular expansion. While adding new grammar rules increases the depth of recursive calls, the architecture supports efficient scaling with minimal overhead for additional rules.

## Documentation

*   [Grammar Coverage vs. the official spec](docs/GRAMMAR_COVERAGE.md)
*   [Scan-rule parity plan](docs/SCAN_RULES_PLAN.md)
*   [Semantic analysis plan](docs/SEMANTIC_PLAN.md)
*   [Grammar in spec EBNF notation](docs/EBNF.md)
*   [Grammar in BNF notation](docs/BNF.md)
* [Software Requirement Specification (SRS)](https://github.com/RuztyCrabs/Blazelint/releases/latest/download/BlazeLint-SRS.pdf)
*   [Pipeline overview](docs/pipeline_overview.md)
*   [Quick Reference](docs/QUICK_REFERENCE.md)
*   [Implementation Notes](docs/IMPLEMENTATION_NOTES.md)

## Installation

Install the latest published version from [crates.io](https://crates.io/crates/blazelint):

```bash
cargo install blazelint
```

Pre-build binaries are available for Linux from the [latest GitHub release](https://github.com/RuztyCrabs/Blazelint/releases/latest). 

_Windows and MacOS binaries will be added in a later release._

## Usage

### Basic Usage

Analyze a Ballerina source file by passing its path to `blazelint`:

```bash
blazelint path/to/file.bal
```

> [!NOTE]
> The parser implements the full 2024R1 grammar ([BNF](docs/BNF.md) / [EBNF](docs/EBNF.md)). Semantic analysis is deliberately shallower — see [grammar coverage](docs/GRAMMAR_COVERAGE.md).

The tool prints the detected diagnostics if there is any and exits with a non-zero status or exits with a zero status with no prints to stdout if the passed file is clean.

### Development Usage

Running from a checked-out repository is also supported:

```bash
cargo run -- path/to/file.bal
```

> [!NOTE]
> `cargo run` builds and executes an unoptimized build (for debug requirements). Always use `cargo build --release` for any benchmark or observations on performance.

For a quick smoke test, you can reuse the sample program in `tests/test-bal-files/`:

```bash
blazelint tests/test-bal-files/simple_errors.bal
```

### Timing Instrumentation

Blazelint supports timing analysis for each pipeline stage. Use the following flags:

- `--timing`: Displays the total time taken by each pipeline stage (lexing, parsing, semantic analysis, linting).
- `--detailed-timing`: Provides a detailed breakdown, including per-rule linting durations.

Example:
```bash
blazelint --timing path/to/file.bal
```

## Configuration

#### Configuration File

Blazelint looks for a `.blazerc` configuration file in the current directory or any parent directory. The configuration uses TOML format:

```toml
# .blazerc - Blazelint Configuration File

[rules]
# Naming convention rules
camel-case = "error"       # Enforces camelCase for variables/functions
constant-case = "warn"     # Enforces SCREAMING_SNAKE_CASE for constants

# Code style rules  
line-length = "warn"       # Limits line length
max-function-length = "error"  # Limits function body length
missing-return = "error"   # Ensures functions have return statements
unused-variables = "warn"  # Detects unused variable declarations

# Official `bal scan` rules
avoid-checkpanic = "warn"          # ballerina:1
unused-parameters = "off"          # ballerina:2 (off: signatures often cannot drop a param)
self-assignment = "warn"           # ballerina:10
invalid-range = "warn"             # ballerina:12
isolated-public-function = "off"   # ballerina:3 (advisory)
isolated-public-method = "off"     # ballerina:4 (advisory)
isolated-public-class = "off"      # ballerina:5 (advisory)

# Disable specific rules
some-rule = "off"

[settings]
max-line-length = 120      # Maximum characters per line
max-function-length = 50   # Maximum lines in function body
```

#### Rule Configuration Values

Each rule can be configured with one of these severity levels:

- `"error"` - Causes build failure (non-zero exit code)
- `"warn"` - Shows warnings but allows build to succeed
- `"info"` - Shows informational messages
- `"off"` - Disables the rule completely

#### Available Rules

| Rule | Description | Default Severity | Settings |
|------|-------------|------------------|----------|
| `camel-case` | Enforces camelCase naming for variables and functions | `error` | None |
| `constant-case` | Enforces SCREAMING_SNAKE_CASE for constants | `warn` | None |
| `line-length` | Limits line length | `warn` | `max-line-length` |
| `max-function-length` | Limits function body length | `warn` | `max-function-length` |
| `missing-return` | Ensures functions have return statements | `error` | None |
| `unused-variables` | Detects unused variable declarations | `warn` | None |
| `unused-parameters` | Detects unused function parameters (`bal scan` ballerina:2) | `off` | None |
| `avoid-checkpanic` | Flags `checkpanic`, which panics on error (`bal scan` ballerina:1) | `warn` | None |
| `self-assignment` | Flags `x = x` (`bal scan` ballerina:10) | `warn` | None |
| `invalid-range` | Flags ranges that never iterate, e.g. `9...0` (`bal scan` ballerina:12) | `warn` | None |
| `isolated-public-function` | Public function not `isolated` (`bal scan` ballerina:3) | `off` | None |
| `isolated-public-method` | Public method not `isolated` (`bal scan` ballerina:4) | `off` | None |
| `isolated-public-class` | Public class not `isolated` (`bal scan` ballerina:5) | `off` | None |

### Configuration Discovery

Blazelint searches for `.blazerc` files in this order:

1. **Current directory**: `./.blazerc`
2. **Parent directories**: Walks up the directory tree looking for `.blazerc`
3. **Default configuration**: Uses built-in defaults if no file found

### Rule Engine

The rule engine features:

- **Dynamic Rule Loading**: Only enabled rules are executed
- **Configurable Severity**: Each rule respects configured severity levels
- **Caching**: Configuration is cached for performance
- **Extensible Design**: New rules can be added easily

## Development environment

A pre-configured [Dev Container](https://containers.dev/) is available that can be used to investigate, develop or debug the program without installing anything on the host machine.

It can be launched and used fully remotely inside a browser using GitHub codespaces, or locally using Visual Studio Code.

### Using GitHub Codespaces

1. Click **Code → Create codespace** from the GitHub UI.
2. Wait for the Codespace to provision (first run will take some significant time).
3. Start Developing!

### Using Visual Studio Code

1. Install the **Dev Containers** extension.
2. Clone this repository and open it in VS Code.
3. Run the **Dev Containers: Reopen in Container** command.
4. Wait till the container spins up.
5. Start Developing!

The container comes with:

- Rust toolchain
- Typst CLI for building the SRS
- Ballerina runtime
- Extensions for Language Servers, syntax highlighting and debugging support
- Common utilities (zsh, GitHub CLI, git, etc.)

### Development Dependencies

The project uses the following key dependencies:

- **Core**: Standard library only for main linting logic
- **Configuration**: `serde`, `toml` for config file parsing
- **Utilities**: `once_cell`, `thiserror` for error handling and caching
- **Testing**: `assert_cmd`, `tempfile` for integration tests
 
## Building

### Prerequisites

- Git 2.51.0 or newer
- Rust Toolchain 1.86.0 or newer [(Get it here)](https://rust-lang.org/tools/install/)

### Steps

1. Create a fork and clone to local:
    ```bash
    git clone https://github.com/<your-profile-name>/Blazelint.git
    ```

2. `cd` into the directory:
    ```bash
    cd Blazelint
    ```

3. Build with cargo:
    ```bash
    cargo build --release
    ```
## Debugging

### Prerequisites

- Build requirements stated [here](#building).
- [Visual Studio Code IDE by Microsoft](https://code.visualstudio.com/download)
- [Rust Analyzer extension by rust-lang.org](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- [CodeLLDB extension by Vadim Chugunov](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)
- Ballerina toolchain and IDE extension (optional - for testing or writing ballerina codes)

### Steps
- You can adjust the `tests/test-bal-files/` files if you need to debug a specific diagnostic.
- Create a `.blazerc` config file to test configuration changes.
- Set breakpoints as needed.
- Click on **Run and Debug** from the main method or use `ctrl+shift+D` to jump to debug menu.

> [!NOTE]
> It is possible to debug with any IDE including Neovim, Emacs and etc but we recommend Visual Studio Code for easier setup. 

## Contributing

- Changes should be developed and push to following branches based on the area of the feature.
    - feature/linter-core: Changes to the linter engine (lexer, parser, semantic analyzer and BNF document).
    - feature/rule-engine: Changes to rule engine, configuration system, and linter rules.
    - ci/cd: Changes related to continous integration and deployments.
    - docs: Changes related to documentation.

-  Run all formatter, lint, and test checks locally before opening a pull request:

    ```bash
    bash scripts/check.sh
    ```

## License

This project is licensed under the [MIT License](LICENSE).
