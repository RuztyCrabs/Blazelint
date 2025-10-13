###### *<div align="right"><sub>// by RuztyCrabs</sub></div>*

<img src="https://raw.githubusercontent.com/RuztyCrabs/Blazelint/refs/heads/main/docs/assets/blazelint-banner-2.webp" alt="BlazeLint banner" style="width: 2000px; height: auto;">

##

> [!CAUTION]
> This is implementing and intended for **Research Purposes** and **SHOULD NOT BE USED ON PRODUCTION ENVIRONMENTS**. The goal is to determine the feasibility, performance and developer experience of Rust Programming Language for implementing static code analyzers for Ballerina Language.

## Documentation

*   [BNF Grammar for Ballerina Subset](docs/BNF.md)
* [Software Requirement Specification (SRS)](https://github.com/RuztyCrabs/Blazelint/releases/latest/download/BlazeLint-SRS.pdf)
*   [Pipeline overview](docs/pipeline_overview.md)

## Installation

Install the latest published version from [crates.io](https://crates.io/crates/blazelint):

```bash
cargo install blazelint
```

> [!TIP]
> Re-run the command with `--force` to pick up newly published releases.

Prefer a prebuilt executable? Download the Linux x86_64 binary from the [latest GitHub release](https://github.com/RuztyCrabs/Blazelint/releases/latest) and place it in your `$PATH`.

Windows and MacOS binaries will be added in next release.

## Usage

Analyze a Ballerina source file by passing its path to `blazelint`:

```bash
blazelint path/to/file.bal
```

The tool prints the input program, a token stream, the parsed AST, and exits with a non-zero status or emits diagnostics if any detected.

Running from a checked-out repository is also supported:

```bash
cargo run -- path/to/file.bal
```

> [!NOTE]
> `cargo run` builds and executes an unoptimized build (for debug requirments). Always use `cargo build --release` for any benchmark or observations on performance.

For a quick smoke test, you can reuse the sample program in `tests/test.bal`:

```bash
blazelint tests/test.bal
```

## Development environment

We ship a ready-to-use [Dev Container](https://containers.dev/) so contributors can build and test BlazeLint without installing Rust, Typst, or CLI tooling locally.

### Visual Studio Code

1. Install the **Dev Containers** extension.
2. Clone this repository and open it in VS Code.
3. Run the **Dev Containers: Reopen in Container** command.

The container comes preloaded with the stable Rust toolchain, `toml-cli`, Typst, and a warmed cargo cache.

### GitHub Codespaces

Prefer a fully hosted setup?

1. Click the **Code** dropdown on GitHub and choose **Create codespace on main** (or your fork/branch).
2. Wait for the Codespace to start; it automatically uses this repo's devcontainer definition.
3. Open a terminal inside the Codespace and run project commands as usual (for example `cargo test` or `typst compile docs/SRS.typ`).
## Building

### Prerequsites

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
> [!NOTE]
> Cargo build will download any required dependancies automatically but you can explicitly get them using `cargo fetch` if still complains about missing libraries.

## Debugging

### Prerequsites

- Build requirements stated [here](#building).
- [VsCode IDE by Microsoft](https://code.visualstudio.com/download)
- [Rust Analyzer extension by rust-lang.org](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- [CodeLLDB extension by Vadim Chugunov](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)

### Steps
- You can adjust the `tests/test.bal` file if you need to debug a specific diagnostic.
- Set breakpoints as needed.
- Click on **Run and Debug** from the main method or use `ctrl+shift+D` to jump to debug menu.

> [!NOTE]
> It is possible to debug with any IDE including Neovim, Emacs and etc but we recommend vscode for easier setup. 

## Contributing

Run all formatter, lint, and test checks locally before opening a pull request:

```bash
bash scripts/check.sh
```
> [!NOTE]
> Cargo will download dev dependancies automatically but you can explicitly get them using `cargo fetch` if still complains about missing libraries.

## TODO

Roadmap of the project can be viewed from [here](TODO.md).

## License

This project is licensed under the [MIT License](LICENSE).
