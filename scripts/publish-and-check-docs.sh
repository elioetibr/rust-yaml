#!/bin/bash

# Script to publish to crates.io and monitor docs.rs build status

set -e

CRATE_NAME="rust-yaml"
VERSION=$(grep '^version' Cargo.toml | cut -d'"' -f2)

echo "📦 Publishing $CRATE_NAME version $VERSION"

# Check if version already exists
if cargo search "$CRATE_NAME" --limit 1 | grep -q "^$CRATE_NAME.*\"$VERSION\""; then
    echo "❌ Version $VERSION already exists on crates.io"
    exit 1
fi

# Verify the package builds
echo "🔨 Building package..."
cargo build --release

# Run tests
echo "🧪 Running tests..."
cargo test

# Package verification
echo "📋 Verifying package..."
cargo package --list

# Publish to crates.io (requires CARGO_REGISTRY_TOKEN)
if [ -z "$CARGO_REGISTRY_TOKEN" ]; then
    echo "⚠️  CARGO_REGISTRY_TOKEN not set. Run: cargo login"
    echo "    Then: export CARGO_REGISTRY_TOKEN=<your-token>"
    exit 1
fi

echo "🚀 Publishing to crates.io..."
cargo publish

echo "⏳ Waiting for crates.io to index (60 seconds)..."
sleep 60

# Check if published
if cargo search "$CRATE_NAME" --limit 1 | grep -q "^$CRATE_NAME.*\"$VERSION\""; then
    echo "✅ Version $VERSION published to crates.io"
else
    echo "⚠️  Version not yet indexed on crates.io"
fi

# Monitor docs.rs build
echo "📚 Monitoring docs.rs build status..."
echo "   This may take a few minutes..."

DOCS_URL="https://docs.rs/$CRATE_NAME/$VERSION"
MAX_ATTEMPTS=20
ATTEMPT=0

while [ $ATTEMPT -lt $MAX_ATTEMPTS ]; do
    ATTEMPT=$((ATTEMPT + 1))
    echo -n "   Attempt $ATTEMPT/$MAX_ATTEMPTS: "
    
    # Check if docs are available
    HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$DOCS_URL")
    
    if [ "$HTTP_STATUS" = "200" ]; then
        echo "✅ Documentation successfully built!"
        echo "   View at: $DOCS_URL"
        exit 0
    elif [ "$HTTP_STATUS" = "404" ]; then
        echo "⏳ Not ready yet..."
        sleep 30
    else
        echo "⚠️  Unexpected status: $HTTP_STATUS"
        sleep 30
    fi
done

echo "⚠️  Documentation build is taking longer than expected."
echo "   Check build status at: https://docs.rs/crate/$CRATE_NAME"
echo "   Direct link will be: $DOCS_URL"