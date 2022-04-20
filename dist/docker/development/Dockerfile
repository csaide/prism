# (c) Copyright 2022 Christian Saide
# SPDX-License-Identifier: GPL-3.0-or-later

FROM rust:1-bullseye as rust

# We don't want warnings for no reason
ARG DEBIAN_FRONTEND=noninteractive

# Install dev-tooling
ENV TOOL_DEPS "curl wget telnet netcat net-tools dnsutils vim-tiny bash-completion sudo jq"
ENV COMPILER_DEPS "upx-ucl gcc-aarch64-linux-gnu"
RUN apt-get update \
    && apt-get upgrade -yqq \
    && apt-get install -yqq \
    ${TOOL_DEPS} ${COMPILER_DEPS} \
    && rm -rf /var/lib/apt/*

# Add our rust utils and nightly toolchain.
RUN rustup component add rust-std \
    && rustup component add rustfmt \
    && rustup component add clippy \
    && rustup component add rust-src \
    && rustup target add x86_64-unknown-linux-gnu \
    && rustup target add aarch64-unknown-linux-gnu \
    && rustup toolchain install nightly-x86_64-unknown-linux-gnu \
    && cargo install cargo-tarpaulin \
    && rustup update

# Install rust-analyzer
RUN curl -L https://github.com/rust-analyzer/rust-analyzer/releases/latest/download/rust-analyzer-linux -o /usr/bin/rust-analyzer \
    && chmod +x /usr/bin/rust-analyzer

# Install dive
ENV DIVE_VERSION "0.9.2"
RUN wget https://github.com/wagoodman/dive/releases/download/v${DIVE_VERSION}/dive_${DIVE_VERSION}_linux_amd64.deb \
    && apt-get install -yqq ./dive_${DIVE_VERSION}_linux_amd64.deb \
    && rm -f ./dive_${DIVE_VERSION}_linux_amd64.deb

# Install grpcurl
ENV GRPCURL_VERSION "1.8.5"
RUN wget https://github.com/fullstorydev/grpcurl/releases/download/v${GRPCURL_VERSION}/grpcurl_${GRPCURL_VERSION}_linux_x86_64.tar.gz \
    && tar xzvf grpcurl_${GRPCURL_VERSION}_linux_x86_64.tar.gz \
    && mv grpcurl /usr/bin/ \
    && rm -rf

# Install grpc_health_probe
ENV GRPC_HEALTH_PROBE_VERSION "v0.4.6"
RUN curl -L https://github.com/grpc-ecosystem/grpc-health-probe/releases/download/${GRPC_HEALTH_PROBE_VERSION}/grpc_health_probe-linux-amd64 --output grpc_health_probe \
    && chmod +x grpc_health_probe \
    && mv grpc_health_probe /usr/bin

# Install docker cli
ARG docker_gid=968
ENV DOCKER_DEPS "apt-transport-https ca-certificates curl gnupg2 software-properties-common"
RUN apt-get update \
    && apt-get install -yqq \
    ${DOCKER_DEPS} \
    && curl -fsSL https://download.docker.com/linux/debian/gpg | apt-key add - \
    && add-apt-repository "deb [arch=amd64] https://download.docker.com/linux/debian bullseye stable" \
    && apt-get update \
    && apt-get install -yqq --no-install-recommends docker-ce-cli \
    && rm -rf /var/lib/apt/* \
    && groupadd -g ${docker_gid} docker

# Add appropriate user ID.
ARG uid=1000
RUN useradd -s /bin/bash -d /home/code -u ${uid} -U -G sudo,docker code \
    && mkdir -p /home/code /opt/prism \
    && chown -R code:code /home/code /usr/local/cargo /opt/prism \
    && echo '%sudo ALL=(ALL) NOPASSWD:ALL' >> /etc/sudoers

# Copy in a custom bashrc with util functions.
COPY --chown=code:code dist/docker/development/.bashrc /home/code/

WORKDIR /opt/prism
