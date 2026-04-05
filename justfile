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

# ══════════════════════════════════════════════════════════════════════════════
# Development
# ══════════════════════════════════════════════════════════════════════════════

# Format all code
[group('Development')]
fmt:
    cargo fmt --all

# Run clippy linter
[group('Development')]
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Run all tests
[group('Development')]
test:
    cargo test --all

# Run tests with coverage report
[group('Development')]
coverage:
    cargo llvm-cov --all --open

# Format and lint
[group('Development')]
check: fmt lint

# ══════════════════════════════════════════════════════════════════════════════
# GitHub
# ══════════════════════════════════════════════════════════════════════════════

# Show unresolved, non-outdated review comments on a PR
[group('GitHub')]
pr-status pr:
    #!/usr/bin/env bash
    set -euo pipefail
    RESULT=$(gh api graphql -f query='
    {
      repository(owner: "robwilkerson", name: "weld-tui") {
        pullRequest(number: '"{{pr}}"') {
          reviewThreads(first: 50) {
            nodes {
              isResolved
              isOutdated
              comments(first: 1) {
                nodes {
                  path
                  line
                  body
                }
              }
            }
          }
        }
      }
    }' --jq '.data.repository.pullRequest.reviewThreads.nodes[] | select(.isResolved == false and .isOutdated == false) | "\(.comments.nodes[0].path):\(.comments.nodes[0].line) — \(.comments.nodes[0].body | split("\n")[0])"')
    if [ -z "$RESULT" ]; then
        echo "✅ No unresolved comments"
    else
        echo "⚠️  Unresolved comments:"
        echo "$RESULT"
    fi
