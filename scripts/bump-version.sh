#!/usr/bin/env bash
# Sets the same version in package.json, src-tauri/tauri.conf.json and src-tauri/Cargo.toml.
# Usage: bash scripts/bump-version.sh 0.2.0
set -euo pipefail
V="${1:-}"; [[ "$V" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "usage: bump-version.sh x.y.z"; exit 2; }
cd "$(dirname "$0")/.."
sed -i -E "s/(\"version\": *\")[0-9]+\.[0-9]+\.[0-9]+(\")/\1$V\2/" package.json src-tauri/tauri.conf.json
sed -i -E "0,/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"/s//version = \"$V\"/" src-tauri/Cargo.toml
grep -H '"version"' package.json src-tauri/tauri.conf.json; grep -H -m1 '^version' src-tauri/Cargo.toml
