###### *<div align="right"><sub>// by RuztyCrabs</sub></div>*

<img src="https://raw.githubusercontent.com/RuztyCrabs/Blazelint/refs/heads/main/docs/assets/blazelint-banner-2.webp" alt="BlazeLint banner" style="width: 2000px; height: auto;">

##

> [!WARNING]
> This is a **University Research Project** and **SHOULD NOT BE USED ON PRODUCTION ENVIRONMENTS**. The goal is to determine the feasibility, performance and developer experience of Rust Programming Language for implementing static code analyzers for Ballerina Language.

## Table of Contents

- [Documentation](#documentation)
- [Installation](#installation)
- [Usage](#usage)
- [Development environment](#development-environment)
	- [Using GitHub Codespaces](#using-github-codespaces)
	- [Using VS Code ](#using-vs-code-if-you-have-it-installed-locally)
- [Building](#building)
	- [Prerequsites ](#prerequsites-skip-if-using-the-dev-container)
	- [Steps](#steps)
- [Debugging](#debugging)
	- [Prerequsites ](#prerequsites-skip-if-using-the-dev-container-1)
	- [Steps](#steps-1)
- [Contributing](#contributing)
- [TODO](#todo)
- [License](#license)

## Documentation

*   [BNF Grammar for Ballerina Subset](docs/BNF.md)
* [Software Requirement Specification (SRS)](https://github.com/RuztyCrabs/Blazelint/releases/latest/download/BlazeLint-SRS.pdf)
*   [Pipeline overview](docs/pipeline_overview.md)

## Installation

Install the latest published version from [crates.io](https://crates.io/crates/blazelint):

```bash
cargo install blazelint
```

Pre-build binaries are available for Linux from the [latest GitHub release](https://github.com/RuztyCrabs/Blazelint/releases/latest). 

_Windows and MacOS binaries will be added in a later release._

## Usage

Analyze a Ballerina source file by passing its path to `blazelint`:

```bash
blazelint path/to/file.bal
```

> [!NOTE]
> Use the limited subset document in the [BNF](docs/BNF.md) when defining Ballerina syntax to be linted.

The tool prints the input program, a token stream, the parsed AST, and exits or emits diagnostics if there is any and exits with a non-zero status.

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
## Debugging

### Prerequsites

- Build requirements stated [here](#building).
- [Visual Studio Code IDE by Microsoft](https://code.visualstudio.com/download)
- [Rust Analyzer extension by rust-lang.org](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- [CodeLLDB extension by Vadim Chugunov](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)
- Ballerina toolchain and IDE extension (optional - for testing or writing ballerina codes)

### Steps
- You can adjust the `tests/test.bal` file if you need to debug a specific diagnostic.
- Set breakpoints as needed.
- Click on **Run and Debug** from the main method or use `ctrl+shift+D` to jump to debug menu.

> [!NOTE]
> It is possible to debug with any IDE including Neovim, Emacs and etc but we recommend Visual Studio Code for easier setup. 

## Contributing

- Changes should be developed and push to following branches based on the area of the feature.
    - feature/linter-core: Changes to the linter engine (lexer, parser, semantic analyzer and BNF document).
    - ci/cd: Changes related to continous integration and deployments.
    - docs: Changes related to documenation.

-  Run all formatter, lint, and test checks locally before opening a pull request:

    ```bash
    bash scripts/check.sh
    ```

## TODO

Roadmap of the project can be viewed from [here](TODO.md).

## License

This project is licensed under the [MIT License](LICENSE).
