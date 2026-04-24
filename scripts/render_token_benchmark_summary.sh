#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <raw-benchmark-output>" >&2
  exit 1
fi

raw_file="$1"

if [[ ! -f "$raw_file" ]]; then
  echo "raw benchmark output not found: $raw_file" >&2
  exit 1
fi

rows="$(
  awk -F'|' '
    NF == 8 && $2 ~ /^[0-9]+$/ {
      print $1 "|" $2 "|" $3 "|" $4 "|" $5 "|" $6 "|" $7 "|" $8
    }
  ' "$raw_file"
)"

if [[ -z "$rows" ]]; then
  echo "no benchmark rows found in: $raw_file" >&2
  exit 1
fi

cat <<'EOF'
## Token Efficiency Summary

Generated from `cargo test report_average_token_benchmarks -- --ignored --nocapture`.

| Workflow average | Samples | Standard tool/action | MCP tool | Raw avg tokens | MCP avg tokens | Saved | Smaller |
|---|---:|---|---|---:|---:|---:|---:|
EOF

while IFS='|' read -r name samples default_tool mcp_tool raw_tokens mcp_tokens saved_pct smaller_factor; do
  printf '| %s | %s | `%s` | `%s` | %s | %s | %s | %s |\n' \
    "$name" \
    "$samples" \
    "$default_tool" \
    "$mcp_tool" \
    "$raw_tokens" \
    "$mcp_tokens" \
    "$saved_pct" \
    "$smaller_factor"
done <<<"$rows"
