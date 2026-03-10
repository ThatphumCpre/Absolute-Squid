# Absolute Squid

`absolute-squid` is an interactive CLI tool built in Rust to easily manage and toggle Kubernetes manifests, including ArgoCD `Application`s, `AppProject`s, and autoscalers (`HorizontalPodAutoscaler`, `ScaledObject`). It scans your local directory for these manifests, identifies their target environments, and groups them by application name. It provides an intuitive nested interactive menu to turn environments and applications "ON", "OFF", or "SEMI" (partially active).

When an application is turned off, the CLI automatically comments out all lines in the pertinent `.yaml` or `.yml` files. When turned on, it uncomments all the lines, restoring the manifest.

## Features
- **Directory Scanning**: Recursively scans any given directory (defaults to `.`) for `.yaml` or `.yml` files.
- **Manifest Detection**: Identifies `Application`, `AppProject`, and autoscale kinds (`HorizontalPodAutoscaler`, `ScaledObject`), even if they are currently commented out.
- **Environment Classification**: Identifies whether a manifest is for `Staging` or `Production` by looking at the directory structure and filename.
- **Group Toggling & SEMI State**: Groups applications and their corresponding autoscalers together. Supports `[ON]`, `[OFF]`, and a `[SEMI]` state (e.g., app is active but autoscaler is commented out).
- **Interactive Nested Menus**: Multi-level menus let you first select an environment, then interactively toggle specific application groups.
- **Smart Formatting**: Comments and uncomments lines robustly without destroying original indentation logic.

## Installation

Ensure you have [Rust and Cargo](https://rustup.rs/) installed.

Then, clone this repository:

```bash
git clone <your-repo-url> absolute-squid
cd absolute-squid
```

### Quick Install (Recommended)

The easiest way to install the tool globally is using the provided `install.sh` script or the `Makefile`:

```bash
./install.sh
# OR
make install
```

This will automatically build and install the `absolute-squid` binary to your `~/.cargo/bin` directory, making it available globally on your system.

### Manual Install

Alternatively, you can install it manually using Cargo:

```bash
cargo install --path .
```

## Usage

Once installed globally, you can run the CLI directly from anywhere.

To scan the current directory:
```bash
absolute-squid .
```

To scan a specific directory:
```bash
absolute-squid /path/to/argocd/manifests
```

### Using Cargo (without installing)

If you prefer not to install the binary globally, you can run it via Cargo from the project root:

```bash
cargo run -- .
cargo run -- /path/to/argocd/manifests
```

### Interactive Menu

When you run the tool, you will see an interactive prompt that first asks for the environment:

```text
? Which environment do you want to manage?
> Staging Environment (1 app)
  Production Environment (1 app)
```

After selecting an environment, you will see a list of application groups and their states:

```text
? Which project name?
> [SEMI] - staging-web
  [ON] - some-other-app
  == Exit / Done ==
```

Selecting an application will let you choose its new state:

```text
? Current state is [SEMI]. What do you want to do rn?
> Turn ON
  Turn OFF
  Cancel
```

- **Turn ON**: Uncomments all associated manifests (Application, AppProject, Autoscalers).
- **Turn OFF**: Comments out all associated manifests.
- **Turn SEMI**: Uncomments the main Application/AppProject but comments out the Autoscalers.

The files will automatically be updated (uncommented/commented) on disk based on your selection.

## Dependencies

- [clap](https://crates.io/crates/clap) - For command-line argument parsing.
- [walkdir](https://crates.io/crates/walkdir) - For recursive directory scanning.
- [inquire](https://crates.io/crates/inquire) - For the interactive multi-select prompt.
