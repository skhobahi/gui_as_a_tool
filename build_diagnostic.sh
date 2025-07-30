#!/bin/bash

# Build Diagnostic Script for Agent HUD v4
echo "🔍 Agent HUD v4 Build Diagnostic"
echo "================================="

# Check system prerequisites
echo "📋 Checking system prerequisites..."

# Check if we're in the right directory
if [ ! -f "src-tauri/Cargo.toml" ]; then
    echo "❌ Error: Must be run from project root (where src-tauri/ directory exists)"
    exit 1
fi

# Check if rust is installed
if ! command -v rustc &> /dev/null; then
    echo "❌ Rust is not installed. Please install Rust first:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo is not installed. Please install Rust toolchain"
    exit 1
fi

echo "✅ Rust $(rustc --version)"
echo "✅ Cargo $(cargo --version)"

# Check frontend dist directory
if [ ! -d "ui/dist" ]; then
    echo "❌ Frontend dist directory not found"
    echo "📁 Creating minimal frontend dist..."
    mkdir -p ui/dist
    cp ui/dist/index.html ui/dist/index.html.bak 2>/dev/null || true
    echo "⚠️  Please ensure frontend is properly built"
else
    echo "✅ Frontend dist directory exists"
    echo "📁 Frontend files:"
    ls -la ui/dist/
fi

# Move to src-tauri directory
cd src-tauri

# Check Tauri dependencies
echo ""
echo "🔍 Checking Tauri configuration..."
echo "📋 Cargo.toml dependencies:"
grep -A 20 "^\[dependencies\]" Cargo.toml

echo ""
echo "📋 tauri.conf.json:"
if [ -f "tauri.conf.json" ]; then
    echo "✅ Configuration file exists"
    # Check if JSON is valid
    if command -v python3 &> /dev/null; then
        if python3 -m json.tool tauri.conf.json > /dev/null 2>&1; then
            echo "✅ JSON syntax is valid"
        else
            echo "❌ JSON syntax error in tauri.conf.json"
        fi
    fi
else
    echo "❌ tauri.conf.json not found"
fi

# Clean and update dependencies
echo ""
echo "🧹 Cleaning previous builds..."
cargo clean

echo ""
echo "🔄 Updating dependencies..."
cargo update

# Try check first
echo ""
echo "🔍 Running cargo check..."
if cargo check 2>&1 | tee check_output.log; then
    echo "✅ Cargo check passed"
else
    echo "❌ Cargo check failed - see check_output.log"
    echo "🔍 Last 10 lines of check output:"
    tail -10 check_output.log
fi

# Try debug build
echo ""
echo "🔨 Attempting debug build..."
if cargo build 2>&1 | tee build_output.log; then
    echo "✅ Debug build successful!"
    
    # List the built binary
    if [ -f "target/debug/agent-hud-v4" ]; then
        echo "📦 Debug binary: target/debug/agent-hud-v4"
        ls -la target/debug/agent-hud-v4
    elif [ -f "target/debug/agent-hud-v4.exe" ]; then
        echo "📦 Debug binary: target/debug/agent-hud-v4.exe"
        ls -la target/debug/agent-hud-v4.exe
    else
        echo "🔍 Debug build files:"
        ls -la target/debug/ | head -10
    fi
    
    # Try release build
    echo ""
    echo "🔨 Attempting release build..."
    if cargo build --release 2>&1 | tee -a build_output.log; then
        echo "🎉 Release build successful!"
        
        # List the built binary
        if [ -f "target/release/agent-hud-v4" ]; then
            echo "📦 Release binary: target/release/agent-hud-v4"
            ls -la target/release/agent-hud-v4
        elif [ -f "target/release/agent-hud-v4.exe" ]; then
            echo "📦 Release binary: target/release/agent-hud-v4.exe"  
            ls -la target/release/agent-hud-v4.exe
        else
            echo "🔍 Release build files:"
            ls -la target/release/ | head -10
        fi
        
        echo ""
        echo "🎉 BUILD SUCCESSFUL!"
        echo "The Agent HUD v4 application has been built successfully."
        echo "You can run it from the target/release/ directory."
        
    else
        echo "❌ Release build failed, but debug build succeeded"
        echo "You can use the debug build from target/debug/"
    fi
    
else
    echo "❌ Debug build failed"
    echo ""
    echo "🔍 Build error summary:"
    echo "======================"
    tail -30 build_output.log
    echo ""
    echo "📋 Full build log saved to: build_output.log"
    echo ""
    echo "🛠️  Common fixes:"
    echo "   1. Make sure all dependencies are compatible"
    echo "   2. Check if system libraries are installed (on Linux: pkg-config, libssl-dev, etc.)"
    echo "   3. Try 'cargo update' to update dependencies"
    echo "   4. Check that the frontend dist directory exists and has content"
fi

echo ""
echo "📋 Build diagnostic complete."