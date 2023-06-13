#!/usr/bin/env bash

set -eu

readonly release_url="https://raw.githubusercontent.com/rust-lang/rust/master/RELEASES.md";
readonly release_date=`curl -s ${release_url} | grep 'Version' | head -n 1 | cut -d'(' -f2 | cut -d')' -f1`

cat >rust-toolchain.toml <<EOF
[toolchain]
channel = "stable-${release_date}"
EOF

echo "Bumped to ${release_date}"