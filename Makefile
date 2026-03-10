.PHONY: build run install clean

# Default target
all: build

# Build the project in release mode
build:
	cargo build --release

# Run the project
run:
	cargo run --release

# Install the binary globally (to ~/.cargo/bin)
install:
	cargo install --path .

# Clean the target directory
clean:
	cargo clean
