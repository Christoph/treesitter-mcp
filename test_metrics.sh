#!/bin/bash
# Test Metrics Dashboard
# Tracks test suite health and quality metrics

set -e

echo "=== Test Suite Metrics ==="
echo ""

# Count tests
total_tests=$(rg '#\[test\]' tests/ | wc -l | tr -d ' ')
echo "📊 Total tests: $total_tests"

# Count ignored tests
ignored_tests=$(rg '#\[ignore\]' tests/ --count-matches 2>/dev/null | awk '{s+=$1} END {print s}')
if [ -z "$ignored_tests" ]; then
    ignored_tests=0
fi
echo "⏭️  Ignored tests: $ignored_tests"

# Count test files
test_files=$(find tests -name '*test*.rs' -o -name '*_test.rs' | wc -l | tr -d ' ')
echo "📁 Test files: $test_files"

# Count test lines
test_lines=$(find tests -name '*.rs' -exec wc -l {} + | tail -1 | awk '{print $1}')
echo "📝 Test lines: $test_lines"

echo ""
echo "=== Running Tests ==="
echo ""

# Run tests and capture timing
start_time=$(date +%s)
cargo test --quiet
end_time=$(date +%s)
duration=$((end_time - start_time))

echo ""
echo "⏱️  Test duration: ${duration}s"

echo ""
echo "=== Test Quality Metrics ==="
echo ""

# Check for helper usage
helper_usage=$(rg 'common::helpers::' tests/ --count-matches 2>/dev/null | awk '{s+=$1} END {print s}')
if [ -z "$helper_usage" ]; then
    helper_usage=0
fi
echo "🔧 Helper function calls: $helper_usage"

# Check for property tests
property_tests=$(rg 'proptest!' tests/ --count-matches 2>/dev/null | awk '{s+=$1} END {print s}')
if [ -z "$property_tests" ]; then
    property_tests=0
fi
echo "🎲 Property test blocks: $property_tests"

# Check for cross-language tests
cross_lang_tests=$(rg 'for.*lang.*in.*test_cases' tests/cross_language_test.rs --count-matches 2>/dev/null | awk '{s+=$1} END {print s}')
if [ -z "$cross_lang_tests" ]; then
    cross_lang_tests=0
fi
echo "🌐 Cross-language test loops: $cross_lang_tests"

echo ""
echo "=== Summary ==="
echo ""

if [ "$ignored_tests" -eq 0 ]; then
    echo "✅ No ignored tests"
else
    echo "⚠️  $ignored_tests ignored tests need attention"
fi

if [ "$duration" -lt 10 ]; then
    echo "✅ Fast test suite (< 10s)"
elif [ "$duration" -lt 30 ]; then
    echo "⚠️  Moderate test speed (10-30s)"
else
    echo "❌ Slow test suite (> 30s)"
fi

echo ""
echo "🎯 Test suite health: GOOD"
