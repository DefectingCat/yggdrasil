# syntax=docker/dockerfile:1

# -----------------------------------------------------------------------------
# Builder stage: compile the static-linked musl server binary and frontend assets
# -----------------------------------------------------------------------------
FROM rust:1.96-bookworm AS builder

# Install system build tooling. Native dependencies are needed for:
#   - musl-tools: linker for x86_64-unknown-linux-musl
#   - cmake/clang/nasm/libssl-dev: libwebp (zenwebp), ring, syntect
#   - curl/gnupg/ca-certificates: NodeSource repository setup
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        cmake \
        clang \
        nasm \
        pkg-config \
        libssl-dev \
        musl-tools \
        ca-certificates \
        curl \
        gnupg \
        git \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js 22 (required by Tailwind CSS v4 and the Tiptap editor build).
RUN mkdir -p /etc/apt/keyrings \
    && curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key \
       | gpg --dearmor -o /etc/apt/keyrings/nodesource.gpg \
    && echo "deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_22.x nodistro main" \
       > /etc/apt/sources.list.d/nodesource.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

# Add the targets used by Dioxus fullstack builds.
RUN rustup target add wasm32-unknown-unknown x86_64-unknown-linux-musl

# Install the Dioxus CLI (must match the dioxus crate version).
RUN cargo install dioxus-cli --version 0.7.9 --locked

# Install the Tailwind CSS v4 standalone binary.
ARG TAILWIND_VERSION=4.3.1
RUN curl -fsSL "https://github.com/tailwindlabs/tailwindcss/releases/download/v${TAILWIND_VERSION}/tailwindcss-linux-x64" \
    -o /usr/local/bin/tailwindcss \
    && chmod +x /usr/local/bin/tailwindcss

WORKDIR /build

# Cache the Tiptap editor's node_modules by copying only package manifests first.
COPY libs/tiptap-editor/package*.json libs/tiptap-editor/
RUN cd libs/tiptap-editor && npm ci --include=dev

# Copy the rest of the source tree and build everything.
COPY . .

# Build the Tiptap editor, syntax-highlight CSS and Tailwind stylesheet.
# These steps produce the contents of the public/ directory.
RUN make build-editor && make highlight-css && tailwindcss -i input.css -o public/style.css --minify

# Build the client-side Dioxus WASM bundle. We use dx only for the client assets;
# dx's linker wrapper is incompatible with a raw static linker, so the server
# binary is built with plain cargo in the next step. The client build emits a
# ready-to-serve public/ directory under target/dx/yggdrasil/*/web/public.
RUN dx build @client --release --debug-symbols=false --wasm-js-cfg false && \
    mkdir -p /build/dist/public && \
    cp -r /build/target/dx/yggdrasil/*/web/public/* /build/dist/public/

# Build the server as a fully static musl binary. musl-gcc is used as the
# linker driver so that -lc/-ldl are resolved against the static musl C library.
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc
ENV RUSTFLAGS="-C target-feature=+crt-static -C relocation-model=static"
RUN cargo build --release --target x86_64-unknown-linux-musl --no-default-features --features server

# Ensure the uploads directory exists for runtime image caching.
RUN mkdir -p uploads

# -----------------------------------------------------------------------------
# Runtime stage: minimal scratch image with the static musl binary
# -----------------------------------------------------------------------------
FROM scratch

WORKDIR /app

# Copy the static musl server binary and the bundled public assets.
COPY --from=builder --chown=65534:65534 /build/target/x86_64-unknown-linux-musl/release/yggdrasil /app/server
COPY --from=builder --chown=65534:65534 /build/dist/public /app/public
COPY --from=builder --chown=65534:65534 /build/uploads /app/uploads

# The app checks for DATABASE_URL on startup even though this image is intended
# to run without a real database. A placeholder is enough to let the server boot.
ENV DATABASE_URL=postgres://postgres:postgres@localhost:5432/yggdrasil
ENV DIOXUS_PUBLIC_PATH=/app/public
ENV IP=0.0.0.0
ENV PORT=3000
ENV RUST_LOG=info

USER 65534:65534

EXPOSE 3000

ENTRYPOINT ["/app/server"]
