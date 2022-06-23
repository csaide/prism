#!/bin/bash

# (c) Copyright 2022 Christian Saide
# SPDX-License-Identifier: GPL-3.0-or-later

RUSTFLAGS="-C instrument-coverage" \
    RUSTDOCFLAGS="-C instrument-coverage -Z unstable-options --persist-doctests target/debug/doctestbins" \
    LLVM_PROFILE_FILE="test-%m.profraw" \
    cargo test --tests

llvm-profdata merge -sparse test-*.profraw -o test.profdata

echo llvm-cov report \
    --use-color \
    --ignore-filename-regex=${CARGO_HOME}'/registry' \
    --ignore-filename-regex='target/debug/build' \
    --ignore-filename-regex='src/thread/local.rs' \
    --instr-profile=test.profdata \
    $( \
      for file in \
        $( \
            RUSTFLAGS="-C instrument-coverage" \
                RUSTDOCFLAGS="-C instrument-coverage -Z unstable-options --persist-doctests target/debug/doctestbins" \
                cargo test --tests --no-run --message-format=json \
                    | jq -r "select(.profile.test == true) | .filenames[]" \
                    | grep -v dSYM - \
        ) \
        target/debug/doctestbins/*/rust_out; \
      do \
        [[ -x $file ]] && printf "%s %s " -object $file; \
      done \
    )

# (llvm-cov show \
#     --use-color \
#     --ignore-filename-regex=${CARGO_HOME}'/registry' \
#     --ignore-filename-regex='target/debug/build' \
#     --ignore-filename-regex='src/thread/local.rs' \
#     --instr-profile=test.profdata \
#     $( \
#       for file in \
#         $( \
#           RUSTFLAGS="-C instrument-coverage" \
#             cargo test --tests --no-run --message-format=json \
#               | jq -r "select(.profile.test == true) | .filenames[]" \
#               | grep -v dSYM - \
#         ); \
#       do \
#         printf "%s %s " -object $file; \
#       done \
#     ) \
#     --show-instantiations --show-line-counts-or-regions \
#     --Xdemangler=rustfilt)
