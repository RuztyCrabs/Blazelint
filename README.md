###### *<div align="right"><sub>// by RuztyCrabs</sub></div>*

<img src="docs/assets/Blazelint-banner.webp" alt="BlazeLint banner" style="width: 2000px; height: auto;">

##

> [!CAUTION]
> This program is still in **early-development** and some critical components are not yet fully implemented.

## Documentation

*   [BNF Grammar for Ballerina Subset](docs/BNF.md)
*   [Software Requirement Specification (Available through releases)]()
*   [Pipeline overview](docs/pipeline_overview.md)

## Building

### Prerequsites

- Git 2.51.0 or newer
- Rust Toolchain 1.86.0 or newer

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
- VsCode IDE
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
