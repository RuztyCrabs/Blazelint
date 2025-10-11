<div align="center">

<img src=".github/assets/blazelint_logo.webp" alt="BlazeLint logo" style="width: 90px; height: auto;">

# BlazeLint
 -- An efficient linter for Ballerina Lang --

</div>

##

> [!CAUTION]
> This program is still in **early-development** and some critical components are not yet fully implemented.

## Documentation

*   [BNF Grammar for Ballerina Subset](docs/BNF.md)
*   [Software Requirement Specification (Available through releases)]()

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
