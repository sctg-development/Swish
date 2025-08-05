# syntax=docker/dockerfile:experimental
# Copyright 2024 SCTG Development - Ronan LE MEILLAT
# SPDX-License-Identifier: AGPL-3.0-or-later
FROM ubuntu:24.10 AS builder
RUN apt-get update && apt-get install --no-install-recommends -y curl build-essential debhelper devscripts \
                pkg-config libssl-dev libc-dev libstdc++-13-dev libgcc-13-dev \
                zip git libcurl4-openssl-dev musl-dev musl-tools cmake libclang-dev g++
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y 
RUN echo $(dpkg --print-architecture)
RUN mkdir /build
RUN if [ "$(dpkg --print-architecture)" = "armhf" ]; then \
       . /root/.cargo/env && rustup target add armv7-unknown-linux-musleabihf; \
       ln -svf /usr/bin/ar /usr/bin/arm-linux-musleabihf-ar; \
       ln -svf /usr/bin/strip /usr/bin/arm-linux-musleabihf-strip; \
       ln -svf /usr/bin/ranlib /usr/bin/arm-linux-musleabihf-ranlib; \
       echo "armv7-unknown-linux-musleabihf" > /build/_target ; \
    fi
RUN if [ "$(dpkg --print-architecture)" = "arm64" ]; then \
       . /root/.cargo/env && rustup target add aarch64-unknown-linux-musl; \
       ln -svf /usr/bin/ar /usr/bin/aarch64-linux-musl-ar; \
       ln -svf /usr/bin/strip /usr/bin/aarch64-linux-musl-strip; \
       ln -svf /usr/bin/ranlib /usr/bin/aarch64-linux-musl-ranlib; \
       echo "aarch64-unknown-linux-musl" > /build/_target ; \
    fi
RUN if [ "$(dpkg --print-architecture)" = "amd64" ]; then \
       . /root/.cargo/env && rustup target add x86_64-unknown-linux-musl; \
       echo "x86_64-unknown-linux-musl" > /build/_target ; \
    fi
COPY sevenz-rust  /build/sevenz-rust
COPY src /build/src
COPY build.rs /build/build.rs
COPY Cargo.toml /build/Cargo.toml
RUN mkdir -p /build/caroot
# Set up cargo config to use git tool
# this workaround is needed because QEMU emulating 32 bits platfom on 64 bits host
# see https://github.com/rust-lang/cargo/issues/8719
# RUN mv /root/.cargo /tmp && rm -rf /root/.cargo && mkdir -p /root/.cargo 
# RUN --mount=type=tmpfs,target=/root/.cargo export TARGET=$(cat /build/_target) \
#     && mkdir -p /root/.cargo \
#     && cp -av /tmp/.cargo/* /root/.cargo/ && ls -lR /root/.cargo \
#     && if [ ! -f /root/.cargo/config.toml ]; then \
#         echo "" > /root/.cargo/config.toml; \
#     fi \
#     && if [ "$(dpkg --print-architecture)" = "armhf" ]; then \
#         export CFLAGS=" -I/usr/include/arm-linux-musleabihf -I/usr/lib/gcc/arm-linux-gnueabihf/13/include -I/usr/include"; \
#     fi && \
#     awk 'BEGIN{net_section=0;git_fetch_found=0;printed=0}/^\[net\]/{net_section=1;print;next}/^\[/{if(net_section&&!git_fetch_found){print "git-fetch-with-cli = true";printed=1}net_section=0;print;next}net_section&&/^git-fetch-with-cli\s*=/{print "git-fetch-with-cli = true";git_fetch_found=1;next}{print}END{if(!printed&&!git_fetch_found){if(!net_section)print "\n[net]";print "git-fetch-with-cli = true"}}' /root/.cargo/config.toml > /root/.cargo/config.tmp && \
#     mv /root/.cargo/config.tmp /root/.cargo/config.toml \
#     && . /root/.cargo/env && cd /build \ 
#     && cargo build --target=$TARGET --release || (echo "Build failed, entering sleep mode for debugging..." && cp -av /root/.cargo /build/ && exit 1) \
#     && mkdir -p /build/ubuntu-noble/bin \
#     && cp /build/target/$(cat /build/_target)/release/swish /build/ubuntu-noble/bin/ 
RUN export TARGET=$(cat /build/_target) \
    && . /root/.cargo/env && cd /build \ 
    && cargo build --target=$TARGET --release || (echo "Build failed, entering sleep mode for debugging..." && cp -av /root/.cargo /build/ && exit 1) \
    && mkdir -p /build/ubuntu-noble/bin \
    && cp /build/target/$(cat /build/_target)/release/swish /build/ubuntu-noble/bin/
FROM ubuntu:24.10
COPY --from=builder /build/ubuntu-noble/bin/swish /usr/local/bin/swish
