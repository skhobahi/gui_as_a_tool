#!/usr/bin/env python3
"""
Agent HUD v4 Build Script
A Python-based build script as an alternative to bash scripts
"""

import os
import sys
import subprocess
import json
from pathlib import Path

def run_command(cmd, cwd=None, capture_output=False):
    """Run a shell command and return the result"""
    print(f"🔧 Running: {cmd}")
    try:
        if capture_output:
            result = subprocess.run(cmd, shell=True, cwd=cwd, capture_output=True, text=True)
            return result.returncode == 0, result.stdout, result.stderr
        else:
            result = subprocess.run(cmd, shell=True, cwd=cwd)
            return result.returncode == 0, "", ""
    except Exception as e:
        print(f"❌ Error running command: {e}")
        return False, "", str(e)

def check_prerequisites():
    """Check if all prerequisites are installed"""
    print("📋 Checking prerequisites...")
    
    # Check Rust
    success, stdout, stderr = run_command("rustc --version", capture_output=True)
    if not success:
        print("❌ Rust is not installed. Please install from https://rustup.rs/")
        return False
    print(f"✅ {stdout.strip()}")
    
    # Check Cargo
    success, stdout, stderr = run_command("cargo --version", capture_output=True)
    if not success:
        print("❌ Cargo is not installed")
        return False
    print(f"✅ {stdout.strip()}")
    
    return True

def check_project_structure():
    """Check if project structure is correct"""
    print("📁 Checking project structure...")
    
    project_root = Path(__file__).parent
    src_tauri = project_root / "src-tauri"
    ui_dist = project_root / "ui" / "dist"
    
    if not src_tauri.exists():
        print("❌ src-tauri directory not found")
        return False
    print("✅ src-tauri directory exists")
    
    if not ui_dist.exists():
        print("❌ ui/dist directory not found")
        return False
    print("✅ ui/dist directory exists")
    
    # Check for required files
    required_files = [
        src_tauri / "Cargo.toml",
        src_tauri / "tauri.conf.json",
        ui_dist / "index.html",
        ui_dist / "app.js"
    ]
    
    for file_path in required_files:
        if not file_path.exists():
            print(f"❌ Required file missing: {file_path}")
            return False
        print(f"✅ {file_path.name} exists")
    
    return True

def validate_config():
    """Validate the Tauri configuration"""
    print("🔍 Validating configuration...")
    
    config_path = Path(__file__).parent / "src-tauri" / "tauri.conf.json"
    try:
        with open(config_path, 'r') as f:
            config = json.load(f)
        print("✅ tauri.conf.json is valid JSON")
        
        # Check for required fields
        required_fields = ["productName", "version", "identifier"]
        for field in required_fields:
            if field not in config:
                print(f"⚠️  Missing field in config: {field}")
            else:
                print(f"✅ {field}: {config[field]}")
        
        return True
    except json.JSONDecodeError as e:
        print(f"❌ Invalid JSON in tauri.conf.json: {e}")
        return False
    except Exception as e:
        print(f"❌ Error reading config: {e}")
        return False

def build_project():
    """Build the project"""
    print("🔨 Building project...")
    
    src_tauri_path = Path(__file__).parent / "src-tauri"
    
    # Clean previous builds
    print("🧹 Cleaning previous builds...")
    success, _, _ = run_command("cargo clean", cwd=src_tauri_path)
    if not success:
        print("⚠️  Failed to clean, continuing anyway...")
    
    # Update dependencies
    print("🔄 Updating dependencies...")
    success, _, _ = run_command("cargo update", cwd=src_tauri_path)
    if not success:
        print("⚠️  Failed to update dependencies, continuing anyway...")
    
    # Check compilation
    print("🔍 Checking compilation...")
    success, _, _ = run_command("cargo check", cwd=src_tauri_path)
    if not success:
        print("❌ Cargo check failed")
        return False
    print("✅ Cargo check passed")
    
    # Build debug version
    print("🔨 Building debug version...")
    success, _, _ = run_command("cargo build", cwd=src_tauri_path)
    if not success:
        print("❌ Debug build failed")
        return False
    print("✅ Debug build successful")
    
    # Build release version
    print("🔨 Building release version...")
    success, _, _ = run_command("cargo build --release", cwd=src_tauri_path)
    if success:
        print("🎉 Release build successful!")
        
        # Check for the binary
        binary_paths = [
            src_tauri_path / "target" / "release" / "agent-hud-v4",
            src_tauri_path / "target" / "release" / "agent-hud-v4.exe"
        ]
        
        for binary_path in binary_paths:
            if binary_path.exists():
                print(f"📦 Binary created: {binary_path}")
                print(f"📊 Size: {binary_path.stat().st_size / 1024 / 1024:.1f} MB")
                return True
        
        print("⚠️  Release build completed but binary not found in expected location")
        return True
    else:
        print("❌ Release build failed, but debug build succeeded")
        print("You can use the debug build from target/debug/")
        return False

def main():
    """Main build function"""
    print("🚀 Agent HUD v4 Build Script")
    print("=" * 40)
    
    # Change to script directory
    os.chdir(Path(__file__).parent)
    
    # Check prerequisites
    if not check_prerequisites():
        sys.exit(1)
    
    # Check project structure
    if not check_project_structure():
        sys.exit(1)
    
    # Validate configuration
    if not validate_config():
        sys.exit(1)
    
    # Build project
    success = build_project()
    
    if success:
        print("\n🎉 BUILD COMPLETED SUCCESSFULLY!")
        print("=" * 40)
        print("The Agent HUD v4 application has been built.")
        print("You can find the binary in:")
        print("  - Debug: src-tauri/target/debug/")
        print("  - Release: src-tauri/target/release/")
        print("\nTo run the application:")
        print("  ./src-tauri/target/release/agent-hud-v4")
    else:
        print("\n❌ BUILD FAILED")
        print("=" * 40)
        print("Please check the error messages above.")
        print("You may still be able to use the debug build if it succeeded.")
        sys.exit(1)

if __name__ == "__main__":
    main()