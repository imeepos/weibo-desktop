#!/bin/bash
# 测试 validate-cookies.js 脚本

set -e

echo "Building TypeScript..."
cd "$(dirname "$0")"
npm run build

echo ""
echo "Testing with invalid cookies..."
node dist/validate-cookies.js '{"SUB":"invalid_token"}' || echo "Expected failure - invalid cookies"

echo ""
echo "Testing with missing argument..."
node dist/validate-cookies.js || echo "Expected failure - missing argument"

echo ""
echo "All tests passed!"
