#!/bin/bash

# Clean up any bad icon files
cd /Users/shahin/Desktop/Projects/cc_quant/agent-hud-v4/src-tauri
rm -rf icons
mkdir -p icons

# Source Rust environment
source $HOME/.cargo/env

# Clean previous build
cargo clean

# Try to build
cargo build --release