# Absolute Squid

`absolute-squid` is an interactive CLI tool built in Rust to easily manage and toggle ArgoCD `Application` and `AppProject` manifests. It scans your local directory for ArgoCD YAMLs, identifies whether they belong to Staging or Production environments, and provides an interactive menu to turn them "on" or "off".

When an application is turned off, the CLI automatically comments out all lines in the pertinent `.yaml` or `.yml` file. When turned on, it uncomment all the lines, restoring the manifest.

## Features
- **Directory Scanning**: Recursively scans any given directory (defaults to `.`) for `.yaml` or `.yml` files.
- **ArgoCD Manifest Detection**: Identifies `Application` and `AppProject` kinds, even if they are currently commented out.
- **Environment Classification**: Identifies whether a manifest is for `Staging` or `Production` by looking at the filename and content.
- **Interactive Toggling**: Uses an interactive multi-select menu to display current states and allow you to quickly turn components on or off.
- **Smart Formatting**: Comments and uncomments lines robustly without destroying original indentation logic.

## Installation

Ensure you have [Rust and Cargo](https://rustup.rs/) installed.

Then, clone this repository and build the project:

```bash
git clone <your-repo-url> absolute-squid
cd absolute-squid
cargo build --release
```

The executable will be available at `target/release/absolute-squid`.

## Usage

You can run the CLI directly using Cargo or the built executable.

### Using Cargo

To scan the current directory:
```bash
cargo run -- .
```

To scan a specific directory:
```bash
cargo run -- /path/to/argocd/manifests
```

### Using the Binary

Once built, you can move the binary to your `PATH` and run it from anywhere.

```bash
absolute-squid .
```

### Interactive Menu

When you run the tool, you will see an interactive prompt:

```text
? Turn on/off Staging & Prod deployments:
> [x] [STG] App - staging-app.yaml
  [ ] [PRD] Proj - prod-project.yml
```

- Use the **Up/Down arrow keys** to navigate.
- Use **Space** to toggle a deployment on (checked) or off (unchecked).
- Press **Enter** to confirm your selection.

The files will automatically be updated (uncommented/commented) on disk based on your selection.

## Dependencies

- [clap](https://crates.io/crates/clap) - For command-line argument parsing.
- [walkdir](https://crates.io/crates/walkdir) - For recursive directory scanning.
- [inquire](https://crates.io/crates/inquire) - For the interactive multi-select prompt.
