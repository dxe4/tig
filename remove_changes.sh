#!/bin/bash
# Script to remove all test diffs and restore demo/ to committed state

set -e

cd "$(dirname "$0")"

echo "Removing test changes..."

git checkout -- demo/
git clean -fd demo/

echo "Done! demo/ is back to committed state."
