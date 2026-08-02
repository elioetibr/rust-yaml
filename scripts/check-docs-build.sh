#!/bin/bash

# SPDX-FileCopyrightText: Rust Yaml contributors
#
# SPDX-License-Identifier: MIT OR Apache-2.0

# Script to diagnose docs.rs build issues

set -e

echo "🔍 Checking docs.rs compatibility for rust-yaml"
echo "================================================"

# Check Rust version requirements
echo ""
echo "📋 Checking Rust version requirements:"
RUST_VERSION=$(grep '^rust-version' Cargo.toml | cut -d'"' -f2)
EDITION=$(grep '^edition' Cargo.toml | cut -d'"' -f2)
echo "   Required Rust version: $RUST_VERSION"
echo "   Edition: $EDITION"

# Check current Rust version
echo ""
echo "🦀 Current Rust version:"
rustc --version
cargo --version

# Build documentation locally
echo ""
echo "📚 Building documentation locally..."
if cargo doc --no-deps --all-features; then
    echo "✅ Documentation builds successfully locally"
else
    echo "❌ Documentation build failed locally"
    exit 1
fi

# Check with docs.rs configuration
echo ""
echo "📋 Checking docs.rs metadata configuration:"
if grep -q '\[package.metadata.docs.rs\]' Cargo.toml; then
    echo "✅ docs.rs metadata section found"
    grep -A 10 '\[package.metadata.docs.rs\]' Cargo.toml
else
    echo "⚠️  No docs.rs metadata section found"
fi

# Simulate docs.rs build environment
echo ""
echo "🔨 Simulating docs.rs build (with all features)..."
RUSTDOCFLAGS="--cfg docsrs" cargo doc --all-features --no-deps

# Check for common issues
echo ""
echo "🔍 Checking for common issues:"

# Check for binary dependencies
if grep -q 'cmake\|pkg-config\|bindgen' Cargo.toml; then
    echo "⚠️  Found system dependencies that might not be available on docs.rs"
fi

# Check for large dependencies
echo ""
echo "📊 Checking package size:"
cargo package --list | wc -l | xargs -I {} echo "   Package contains {} files"

# Generate a report
echo ""
echo "📝 Summary:"
echo "==========="
echo "1. Edition: $EDITION"
echo "2. Rust version: $RUST_VERSION"
echo "3. Local doc build: ✅ Success"
echo ""
echo "💡 Recommendations:"
echo "   - Ensure rust-version is not too recent (docs.rs may lag behind)"
echo "   - Edition 2021 is widely supported, Edition 2024 may have limited support"
echo "   - Check https://docs.rs/about/builds for docs.rs build environment"
echo ""
echo "🔗 Useful links:"
echo "   - Build status: https://docs.rs/crate/rust-yaml"
echo "   - docs.rs about: https://docs.rs/about"
echo "   - Build metadata: https://docs.rs/about/metadata"
