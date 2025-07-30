#!/bin/bash

# Agent HUD v4 Build Fix Script
echo "🚀 Starting Agent HUD v4 build process..."

# Change to the src-tauri directory
cd "$(dirname "$0")"
echo "📁 Working directory: $(pwd)"

# Check if Cargo.toml exists
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Cargo.toml not found in current directory"
    exit 1
fi

# Check if frontend dist exists
if [ ! -d "../ui/dist" ]; then
    echo "❌ Error: Frontend dist directory not found at ../ui/dist"
    echo "Please ensure the frontend is built first"
    exit 1
fi

echo "✅ Frontend dist directory found"

# Clean previous builds
echo "🧹 Cleaning previous builds..."
cargo clean

# Try building in debug mode first
echo "🔨 Attempting debug build..."
if cargo build 2>&1 | tee build_output.log; then
    echo "✅ Debug build successful!"
    
    # Try release build
    echo "🔨 Attempting release build..."
    if cargo build --release 2>&1 | tee -a build_output.log; then
        echo "🎉 Release build successful!"
        echo "📦 Binary location: target/release/"
        ls -la target/release/ | grep -E "(agent-hud-v4|exe)$" || echo "Binary files:"
        ls -la target/release/ | head -10
        exit 0
    else
        echo "❌ Release build failed, but debug build succeeded"
        echo "📦 Debug binary location: target/debug/"
        ls -la target/debug/ | grep -E "(agent-hud-v4|exe)$" || echo "Binary files:"
        ls -la target/debug/ | head -10
        exit 1
    fi
else
    echo "❌ Debug build failed"
    echo "📋 Build errors have been saved to build_output.log"
    echo "🔍 Last 20 lines of build output:"
    tail -20 build_output.log
    exit 1
fi