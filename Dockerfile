# syntax=docker/dockerfile:1

# -----------------------------------------------------------------------------
# Builder stage: compile the static-linked musl server binary and frontend assets
# -----------------------------------------------------------------------------
FROM rust:1.96-bookworm AS builder

# Point every network download at a Chinese mirror so the build is fast/reliable
# from inside the container (the host proxy at 127.0.0.1:10808 is unreachable
# from Docker Desktop's NAT, and the official sources are slow or intercepted):
#   - Debian apt          -> TUNA (Tsinghua)
#   - Rust + crates.io    -> rsproxy (ByteDance)
#   - Node.js + npm/pnpm  -> npmmirror (Alibaba)
#   - Tailwind standalone -> gh-proxy.com (China GitHub-release CDN)
ARG DEBIAN_MIRROR=https://mirrors.tuna.tsinghua.edu.cn/debian
ARG DEBIAN_SECURITY_MIRROR=https://mirrors.tuna.tsinghua.edu.cn/debian-security
ARG NODE_MIRROR=https://registry.npmmirror.com/-/binary/node
ARG NPM_REGISTRY=https://registry.npmmirror.com
ARG RS_PROXY=https://rsproxy.cn

# --- Debian apt: rewrite the DEB822 sources to the TUNA mirror. ---
RUN sed -i \
    -e "s|http://deb.debian.org/debian|${DEBIAN_MIRROR}|g" \
    -e "s|http://deb.debian.org/debian-security|${DEBIAN_SECURITY_MIRROR}|g" \
    -e "s|http://security.debian.org/debian-security|${DEBIAN_SECURITY_MIRROR}|g" \
    /etc/apt/sources.list.d/debian.sources

# Install system build tooling. Native dependencies are needed for:
#   - musl-tools: linker for x86_64-unknown-linux-musl
#   - cmake/clang/nasm/libssl-dev: libwebp (zenwebp), ring, syntect
#   - curl/gnupg/ca-certificates: download tooling
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

# --- Node.js 22 + pnpm: install from the npmmirror binary mirror instead of
# the NodeSource apt repo (both TUNA and USTC have dropped their NodeSource
# mirrors; the upstream repo is slow/unreliable from inside the container). ---
ARG NODE_VERSION=22.20.0
RUN ARCH="$(dpkg --print-architecture)" \
    && case "$ARCH" in \
        amd64)  NODE_ARCH=x64   ;; \
        arm64)  NODE_ARCH=arm64 ;; \
        *) echo "unsupported arch: $ARCH" >&2; exit 1 ;; \
    esac \
    && curl -fsSL "${NODE_MIRROR}/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-${NODE_ARCH}.tar.gz" \
        | tar -xz -C /usr/local --strip-components=1 \
    && corepack enable \
    && corepack prepare pnpm@11.8.0 --activate

# Configure npm/pnpm to use the npmmirror registry for all subsequent installs.
RUN npm config set registry "${NPM_REGISTRY}" \
    && pnpm config set registry "${NPM_REGISTRY}"

# --- Rust: point rustup and cargo at the rsproxy mirror. ---
ENV RUSTUP_DIST_SERVER=${RS_PROXY}
ENV RUSTUP_UPDATE_ROOT=${RS_PROXY}/rustup
RUN mkdir -p /usr/local/cargo \
    && printf \
        '[source.crates-io]\nreplace-with = "rsproxy-sparse"\n\n[source.rsproxy-sparse]\nregistry = "sparse+%s/index/"\n' \
        "${RS_PROXY}" \
        > /usr/local/cargo/config.toml

# Add the targets used by Dioxus fullstack builds. Both musl targets are
# installed; each buildx platform leg builds only its native one (see below).
RUN rustup target add wasm32-unknown-unknown \
        x86_64-unknown-linux-musl aarch64-unknown-linux-musl

# Install the Dioxus CLI (must match the dioxus crate version).
RUN cargo install dioxus-cli --version 0.7.9 --locked

# --- Tailwind CSS v4: the standalone binary is distributed via GitHub
# Releases (~106 MB). GitHub's release CDN is slow/unreliable from inside the
# container, and the host proxy (host.docker.internal:10808) throttles large
# files to ~16 KB/s. Route the download through a China GitHub proxy CDN
# instead — gh-proxy.com served the full file at ~430 KB/s in testing.
# Falls back to the direct GitHub URL if GH_PROXY is set empty. ---
ARG TAILWIND_VERSION=4.3.1
ARG GH_PROXY=https://gh-proxy.com
RUN ARCH="$(dpkg --print-architecture)" \
    && case "$ARCH" in \
        amd64)  TW_ARCH=x64   ;; \
        arm64)  TW_ARCH=arm64 ;; \
        *) echo "unsupported arch: $ARCH" >&2; exit 1 ;; \
    esac \
    && GH_URL="https://github.com/tailwindlabs/tailwindcss/releases/download/v${TAILWIND_VERSION}/tailwindcss-linux-${TW_ARCH}" \
    && curl -fsSL -o /usr/local/bin/tailwindcss "${GH_PROXY:+${GH_PROXY}/}${GH_URL}" \
    && chmod +x /usr/local/bin/tailwindcss

WORKDIR /build

# Cache the pnpm workspace node_modules by copying only package manifests first.
# Copying all 4 libs' manifests + the workspace root lets pnpm install everything
# in one shot; this layer is reused as long as the manifests don't change.
COPY libs/package.json libs/pnpm-workspace.yaml libs/pnpm-lock.yaml libs/
COPY libs/tiptap-editor/package.json      libs/tiptap-editor/
COPY libs/codemirror-editor/package.json  libs/codemirror-editor/
COPY libs/lightbox/package.json           libs/lightbox/
COPY libs/yggdrasil-core/package.json     libs/yggdrasil-core/
RUN cd libs && pnpm install --frozen-lockfile

# Copy the rest of the source tree and build everything.
COPY . .

# Build all 4 JS libs, syntax-highlight CSS and Tailwind stylesheet.
# These steps produce the contents of the public/ directory.
RUN make build-libs && make highlight-css && tailwindcss -i input.css -o public/style.css --minify

# Build the client-side Dioxus WASM bundle. We use dx only for the client assets;
# dx's linker wrapper is incompatible with a raw static linker, so the server
# binary is built with plain cargo in the next step. The client build emits a
# ready-to-serve public/ directory under target/dx/yggdrasil/*/web/public.
RUN dx build @client --release --debug-symbols=false --wasm-js-cfg false && \
    mkdir -p /build/dist/public && \
    cp -r /build/target/dx/yggdrasil/*/web/public/* /build/dist/public/

# Build the server as a fully static musl binary, **natively for the buildx
# platform leg**. Each leg builds only its own arch, so musl-gcc (which Debian
# ships for the host arch only) and the target always match — no cross-compiler,
# no QEMU. Cross-compiling here (e.g. building the x86_64 musl target from an
# arm64 leg) breaks ring: cc-rs emits -m64 for the x86_64 target and hands it to
# the arm64 musl-gcc, whose cc1 has no -m64 → "unrecognized command-line option".
RUN ARCH="$(dpkg --print-architecture)" \
    && case "$ARCH" in \
        amd64) MUSL_TARGET=x86_64-unknown-linux-musl  ;; \
        arm64) MUSL_TARGET=aarch64-unknown-linux-musl ;; \
        *) echo "unsupported arch: $ARCH" >&2; exit 1 ;; \
    esac \
    && export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc \
    && export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc \
    && export RUSTFLAGS="-C target-feature=+crt-static -C relocation-model=static" \
    && cargo build --release --target "$MUSL_TARGET" --no-default-features --features server

# Ensure the uploads directory exists for runtime image caching.
RUN mkdir -p uploads

# Stage the built binary + assets at arch-independent paths so the scratch
# runtime stage can COPY them without knowing which musl target was built.
RUN ARCH="$(dpkg --print-architecture)" \
    && case "$ARCH" in \
        amd64) MUSL_TARGET=x86_64-unknown-linux-musl  ;; \
        arm64) MUSL_TARGET=aarch64-unknown-linux-musl ;; \
        *) echo "unsupported arch: $ARCH" >&2; exit 1 ;; \
    esac \
    && cp "/build/target/${MUSL_TARGET}/release/yggdrasil" /build/server

# -----------------------------------------------------------------------------
# Runtime stage: minimal scratch image with the static musl binary
# -----------------------------------------------------------------------------
FROM scratch

WORKDIR /app

# Copy the static musl server binary and the bundled public assets.
COPY --from=builder --chown=65534:65534 /build/server /app/server
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
