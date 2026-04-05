# Command Runner
# Run `just` or `just --list` to see available commands

# Default recipe - show available commands
default:
    @just --list

# ══════════════════════════════════════════════════════════════════════════════
# Setup
# ══════════════════════════════════════════════════════════════════════════════

# Bootstrap the development environment (idempotent - safe to run multiple times)
[group('Setup')]
bootstrap:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "🚀 Bootstrapping development environment..."
    echo ""

    # Configure git hooks (idempotent - git config overwrites existing value)
    echo "🔧 Configuring git hooks..."
    git config core.hooksPath .githooks
    echo "✅ Git hooks configured (using .githooks/)"
    echo ""

    # Check required tools
    echo "📋 Checking required tools..."
    MISSING=0
    command -v cargo >/dev/null 2>&1 || { echo "❌ cargo not found (install via mise or rustup)"; MISSING=1; }
    command -v kingfisher >/dev/null 2>&1 || { echo "❌ kingfisher not found (brew install kingfisher)"; MISSING=1; }
    [ $MISSING -eq 0 ] && echo "✅ All required tools found"
    echo ""

    echo "🎉 Bootstrap complete!"
