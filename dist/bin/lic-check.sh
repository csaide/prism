#!/bin/bash

# (c) Copyright 2022 Christian Saide
# SPDX-License-Identifier: GPL-3.0-or-later

function list_files_missing_lic() {
    find . \
        -not -path './.cargo/*' \
        -not -path './target/*' \
        -not -path './target-coverage/*' \
        -not -path './.git/*' \
        -not -path './.vscode/*' \
        -not -path './.github/*' \
        -not -path './output/*' \
        -not -path './dist/docker/development/.bashrc' \
        -not -name .gitignore \
        -not -name .dockerignore \
        -not -name LICENSE \
        -not -name README.md \
        -not -name rust-toolchain \
        -not -name '*.yaml' \
        -not -name '*.json' \
        -not -name '*.lock' \
        -not -name '*.toml' \
        -not -name '*.pem' \
        -not -name '*.srl' \
        -type f | xargs grep -L 'SPDX-License-Identifier: GPL-3.0-or-later'
}

function count_files_missing_lic() {
    list_files_missing_lic | wc -l
}

function list_files_missing_copy() {
    find . \
        -not -path './.cargo/*' \
        -not -path './target/*' \
        -not -path './target-coverage/*' \
        -not -path './.git/*' \
        -not -path './.vscode/*' \
        -not -path './.github/*' \
        -not -path './output/*' \
        -not -path './dist/docker/development/.bashrc' \
        -not -name .gitignore \
        -not -name .dockerignore \
        -not -name LICENSE \
        -not -name rust-toolchain \
        -not -name '*.yaml' \
        -not -name '*.json' \
        -not -name '*.lock' \
        -not -name '*.toml' \
        -not -name '*.pem' \
        -not -name '*.srl' \
        -type f | xargs grep -L -E '(&copy;|\(c\)) Copyright 2022 Christian Saide'
}

function count_files_missing_copy() {
    list_files_missing_copy | wc -l
}

if [ $(count_files_missing_lic) -ne 0 ]; then
    cat <<EOF
There are files missing the 'SPDX-License-Identifier: GPL-3.0-or-later' license identifier.

Files:
$(list_files_missing_lic)
EOF
    exit 1
fi

if [ $(count_files_missing_copy) -ne 0 ]; then
    cat <<EOF
There are files missing the '&copy;|(c) Copyright 2022 Christian Saide' copyright identifier.

Files:
$(list_files_missing_copy)
EOF
    exit 1
fi

echo "All files have correct copyright and license identifiers."
exit 0
