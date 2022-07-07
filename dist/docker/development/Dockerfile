# (c) Copyright 2022 Christian Saide
# SPDX-License-Identifier: GPL-3.0-or-later

FROM ubuntu:jammy as devel

# We don't want warnings for no reason
ARG DEBIAN_FRONTEND=noninteractive

# Install dev-tooling
ENV TOOL_DEPS "curl wget telnet netcat cmake net-tools dnsutils vim-tiny bash-completion sudo jq git"
ENV COMPILER_DEPS "upx-ucl gcc-aarch64-linux-gnu build-essential"
RUN apt-get update \
    && apt-get upgrade -yqq \
    && apt-get install -yqq \
    ${TOOL_DEPS} ${COMPILER_DEPS} \
    && rm -rf /var/lib/apt/*

# Install rust-analyzer
ENV RUST_ANALYZER_VERSION "2022-07-04"
RUN curl -L https://github.com/rust-lang/rust-analyzer/releases/download/${RUST_ANALYZER_VERSION}/rust-analyzer-x86_64-unknown-linux-gnu.gz -o /usr/bin/rust-analyzer.gz \
    && gunzip /usr/bin/rust-analyzer.gz \
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

# Install docker-compose
ENV DOCKER_COMPOSE_VERSION "v2.6.1"
RUN curl -L https://github.com/docker/compose/releases/download/${DOCKER_COMPOSE_VERSION}/docker-compose-linux-x86_64 -o /usr/bin/docker-compose \
    && chmod +x /usr/bin/docker-compose

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
    && chown -R code:code /home/code /opt/prism \
    && echo '%sudo ALL=(ALL) NOPASSWD:ALL' >> /etc/sudoers

# Copy in a custom bashrc with util functions.
COPY --chown=code:code dist/docker/development/.bashrc /home/code/

USER code
ENV COMPONENTS "rust-std,rustfmt,clippy,rust-src,llvm-tools-preview"
ENV TARGETS "x86_64-unknown-linux-gnu,aarch64-unknown-linux-gnu"
# Add our rust utils and nightly toolchain.
RUN curl -o /home/code/install-rustup.sh --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    && sh /home/code/install-rustup.sh -y -q \
    -c ${COMPONENTS} -t ${TARGETS} \
    && . /home/code/.cargo/env \
    && rustup update \
    && rustup toolchain install nightly-x86_64-unknown-linux-gnu \
    && rustup component add llvm-tools-preview --toolchain nightly-x86_64-unknown-linux-gnu \
    && cargo install cargo-llvm-cov \
    && rm /home/code/install-rustup.sh


WORKDIR /opt/prism
