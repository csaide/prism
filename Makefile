# (c) Copyright 2022 Christian Saide
# SPDX-License-Identifier: GPL-3.0-or-later

###
# OS Determination
###

detected_OS := $(shell uname 2>/dev/null || echo Unknown)
ifeq ($(detected_OS),Linux)
    BUILD_OS := linux
endif
ifeq ($(detected_OS),FreeBSD)
    BUILD_OS := freebsd
endif
ifeq ($(detected_OS),NetBSD)
    BUILD_OS := netbsd
endif
ifeq ($(detected_OS),OpenBSD)
    BUILD_OS := openbsd
endif


###
# Build args and variables.
###

.SECONDEXPANSION:
BUILD := debug
BUILD_ARCH ?= amd64

###
# Target and build definitions for varios OS/arch combinations
###

# Define the resulting targets for building cross-platform
target_linux-amd64 := x86_64-unknown-linux-gnu
target_linux-arm64 := aarch64-unknown-linux-gnu
target_linux := linux-amd64 linux-arm64

# Define an override so that we can turn on/off release builds.
build_debug =
build_release = --release

strip_linux-amd64 := strip
strip_linux-arm64 := aarch64-linux-gnu-strip

###
# Default target definition
###

.PHONY: default devel
default: compile
devel: check compile

###
# Binary compilation steps.
###

.PHONY: docs compile compile.linux

docs:
	@bash ./dist/bin/print.sh "Generating Docs"
	@cargo doc

# Ensure we compile each of the targets properly using the correct mode.
compile-bin.%:
	@bash ./dist/bin/print.sh "Building target: '$*' mode: '$(BUILD)'"
	@mkdir -p ./output/$(BUILD)
	@RUSTFLAGS="-Ctarget-feature=+crt-static" cargo build $(build_$(BUILD)) --target $(target_$*)
	@if [ "$(BUILD)" = "release" ]; then bash ./dist/bin/strip-compress.sh "$(BUILD)" "$(target_$*)" "$(strip_$*)"; fi
	@bash dist/bin/package.sh "$(BUILD)" "$(target_$*)" "$*"

# Build all targets for the biven OS.
compile-exp.%: $$(foreach target,$$(target_$$*),compile-bin.$$(target))
	@bash ./dist/bin/print.sh "Finished building targets for OS: '$*' mode: '$(BUILD)'"

# By default build targets for the local OS, but in theory it should be possible to at least
# compile cross-platform.
compile.linux: compile-exp.linux

compile: compile-bin.$(BUILD_OS)-$(BUILD_ARCH)

###
# Docker commands
###

.PHONY: docker

HASH ?= $(shell git rev-parse HEAD)
docker:
	@bash ./dist/bin/print.sh "Building image"
	@docker buildx build \
		--platform linux/arm64,linux/amd64 \
		--tag ghcr.io/csaide/prism:$(HASH) \
		--build-arg BUILD=release \
		--file ./dist/docker/prism/Dockerfile \
		--push \
		.

promote:
	@bash ./dist/bin/print.sh "Promoting image"
	@docker buildx build \
		--platform linux/arm64,linux/amd64 \
		--tag ghcr.io/csaide/prism:$(TAG) \
		--build-arg HASH=$(HASH) \
		--file ./dist/docker/prism/Dockerfile.promote \
		--push \
		.

###
# Source code validation, formatting, linting.
###

.PHONY: fmt lint units bench coverage coverage-ci coverage-html license check

fmt:
	@bash ./dist/bin/print.sh "Formatting Code"
	@cargo fmt --all -- --emit=files

lint:
	@bash ./dist/bin/print.sh "Linting"
	@cargo fmt --all -- --check
	@cargo clippy -- --no-deps

units:
	@bash ./dist/bin/print.sh "Running tests"
	@cargo test

coverage:
	@bash ./dist/bin/print.sh "Running tests with coverage"
	@cargo +nightly llvm-cov

coverage-ci:
	@bash ./dist/bin/print.sh "Running tests with coverage"
	@cargo +nightly llvm-cov --hide-instantiations --lcov --output-path coverage.lcov

coverage-html:
	@bash ./dist/bin/print.sh "Running tests with coverage"
	@cargo +nightly llvm-cov --hide-instantiations --html

license:
	@bash ./dist/bin/print.sh "Verifying licensing"
	@bash ./dist/bin/lic-check.sh

check: fmt lint units license

###
# Cleanup
###

.PHONY: clean

clean:
	@bash ./dist/bin/print.sh "Cleaning"
	@rm -rf target/ output/
