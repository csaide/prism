#!/bin/bash

# (c) Copyright 2022 Christian Saide
# SPDX-License-Identifier: GPL-3.0-or-later

BUILD="${1}"
TARGET="${2}"
SUFFIX="${3}"

for bin in $(find target/${TARGET}/${BUILD}/* -maxdepth 0 -type f -executable); do
    name="$(basename ${bin})"
    rm -f ./target/output/${BUILD}/${name}_${SUFFIX}
    cp ${bin} ./target/output/${BUILD}/${name}_${SUFFIX}
done
