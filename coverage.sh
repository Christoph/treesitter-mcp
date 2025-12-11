#!/bin/bash
# Code Coverage Script
# Generates HTML coverage report using cargo-tarpaulin

set -e

echo "=== Code Coverage Report ==="
echo ""

# Check if tarpaulin is installed
if ! command -v cargo-tarpaulin &> /dev/null; then
    echo "❌ cargo-tarpaulin not found. Installing..."
    cargo install cargo-tarpaulin
fi

echo "📊 Generating coverage report..."
echo ""

# Run tarpaulin with HTML output
cargo tarpaulin \
    --out Html \
    --output-dir coverage \
    --exclude-files 'tests/*' \
    --exclude-files 'benches/*' \
    --timeout 300 \
    --verbose

echo ""
echo "✅ Coverage report generated!"
echo "📁 Open coverage/index.html to view the report"
echo ""

# Display summary
if [ -f "coverage/index.html" ]; then
    echo "=== Coverage Summary ==="
    # Extract coverage percentage from HTML (basic parsing)
    if command -v grep &> /dev/null; then
        grep -o '[0-9]\+\.[0-9]\+%' coverage/index.html | head -1 || echo "Coverage: See coverage/index.html"
    fi
fi

echo ""
echo "💡 Tips:"
echo "  - Aim for > 80% line coverage"
echo "  - Focus on business logic, not boilerplate"
echo "  - Use coverage to find untested code paths"
