# syntax=docker/dockerfile:1

# -----------------------------------------------------------------------------
# Builder stage: compile the static-linked musl server binary and frontend assets
# -----------------------------------------------------------------------------
# Trixie (Debian 13, glibc 2.41) — required because the prebuilt `dx` v0.7.9
# binary (aarch64/x86_64-unknown-linux-gnu) needs GLIBC_2.39; Bookworm only
# ships 2.36, so `dx --version` fails with "version `GLIBC_2.39' not found".
FROM rust:1.96-trixie AS builder

# Point every network download at a Chinese mirror so the build is fast/reliable
# from inside the container (the host proxy at 127.0.0.1:10808 is unreachable
# from Docker Desktop's NAT, and the official sources are slow or intercepted):
#   - Debian apt          -> TUNA (Tsinghua)
#   - Rust + crates.io    -> rsproxy (ByteDance)
#   - Node.js + npm/pnpm  -> npmmirror (Alibaba)
ARG DEBIAN_MIRROR=https://mirrors.tuna.tsinghua.edu.cn/debian
ARG DEBIAN_SECURITY_MIRROR=https://mirrors.tuna.tsinghua.edu.cn/debian-security
ARG NODE_MIRROR=https://registry.npmmirror.com/-/binary/node
ARG NPM_REGISTRY=https://registry.npmmirror.com
ARG RS_PROXY=https://rsproxy.cn
# GitHub Releases proxy — the dx tarball and tailwindcss binary live on
# github.com releases and download at ~300 KB/s with frequent connection
# resets from China. Prefixing the raw github.com URL routes the download
# through the proxy. Set to "" (empty) to bypass the proxy (e.g. building
# outside China where github.com is fast/reliable).
ARG GH_PROXY=https://gh-proxy.com

# --- Debian apt: rewrite the DEB822 sources to the TUNA mirror. ---
RUN sed -i \
    -e "s|http://deb.debian.org/debian|${DEBIAN_MIRROR}|g" \
    -e "s|http://deb.debian.org/debian-security|${DEBIAN_SECURITY_MIRROR}|g" \
    -e "s|http://security.debian.org/debian-security|${DEBIAN_SECURITY_MIRROR}|g" \
    /etc/apt/sources.list.d/debian.sources

# Install system build tooling. Native dependencies are needed for:
#   - musl-tools: linker for x86_64-unknown-linux-musl
#   - cmake/clang/nasm/libssl-dev: libwebp (zenwebp), ring, syntect
#   - binaryen: provides `wasm-opt` on PATH so dx's release client build
#     uses the local binary instead of fetching one from GitHub Releases.
#     dx's internal wasm-opt download ignores GH_PROXY (only the explicit
#     curl calls for dx/tailwindcss honor it), so without this the build
#     hangs ~80 min then fails with "stream error received: unspecific
#     protocol error detected" trying to fetch wasm-opt mid-build.
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
        binaryen \
        ca-certificates \
        curl \
        gnupg \
        xz-utils \
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

# Install the Dioxus CLI from the official prebuilt binary (GitHub Releases),
# NOT `cargo install` (which compiles dx-cli's huge dep tree from source — the
# slowest single Docker step). The release tag v0.7.9 matches the crate version
# we previously pinned. The prebuilt dx is a glibc (linux-gnu) binary requiring
# GLIBC_2.39 — that's why the builder stage above uses Trixie (glibc 2.41), not
# Bookworm (glibc 2.36). dx runs only in this builder stage (to emit the WASM
# client bundle); it never enters the static-musl runtime image. Each buildx
# platform leg downloads only its native arch; the sha256 pins the exact
# artifact (supply-chain integrity, verified against the release's .sha256
# sidecar).
ARG DX_VERSION=0.7.9
# The 32 MB dx tarball sits on github.com releases; from China it downloads at
# ~300 KB/s and the connection is frequently reset mid-transfer with
# "curl: (56) ... unexpected eof while reading" — the same flaky-upstream
# problem the mirror rewrites above solve for apt/crates/npm. --retry with
# --retry-all-errors (curl 7.71+, Trixie ships 8.x) covers SSL/EOF resets, and
# --continue-at - resumes the partial file instead of restarting from zero on
# each retry. The sha256 pin still catches a corrupted/partial download.
RUN ARCH="$(dpkg --print-architecture)" \
    && case "$ARCH" in \
        amd64) DX_TRIPLET=x86_64-unknown-linux-gnu  DX_SHA256=3b132551b480bc96f938f9f0d37936ee1190f994977539dcc347eaf38540d005 ;; \
        arm64) DX_TRIPLET=aarch64-unknown-linux-gnu DX_SHA256=8cf14db0b11b43b31dd6d39e71b00e567f2fccfde85ae3a8f7ef0f8745e5ccfb ;; \
        *) echo "unsupported arch: $ARCH" >&2; exit 1 ;; \
    esac \
    && DX_URL="${GH_PROXY:+${GH_PROXY}/}https://github.com/DioxusLabs/dioxus/releases/download/v${DX_VERSION}/dx-${DX_TRIPLET}.tar.gz" \
    && curl -fsSL --retry 5 --retry-delay 5 --retry-all-errors --retry-connrefused --continue-at - "${DX_URL}" -o /tmp/dx.tar.gz \
    && echo "${DX_SHA256}  /tmp/dx.tar.gz" | sha256sum -c - \
    && tar -xzf /tmp/dx.tar.gz -C /usr/local/bin \
    && rm /tmp/dx.tar.gz \
    && dx --version

# --- Tailwind CSS v4: the standalone binary is distributed via GitHub
# Releases (~106 MB). ---
ARG TAILWIND_VERSION=4.3.1
RUN ARCH="$(dpkg --print-architecture)" \
    && case "$ARCH" in \
        amd64)  TW_ARCH=x64   ;; \
        arm64)  TW_ARCH=arm64 ;; \
        *) echo "unsupported arch: $ARCH" >&2; exit 1 ;; \
    esac \
    && GH_URL="${GH_PROXY:+${GH_PROXY}/}https://github.com/tailwindlabs/tailwindcss/releases/download/v${TAILWIND_VERSION}/tailwindcss-linux-${TW_ARCH}" \
    && curl -fsSL -o /usr/local/bin/tailwindcss "${GH_URL}" \
    && chmod +x /usr/local/bin/tailwindcss

# --- cargo-chef: 用于把 Rust 依赖编译与源码编译分离成独立的 Docker 层。
# 改一行 .rs 不再触发 576 个依赖的全量重编。CI 端用 GHA cache (type=gha)
# 跨 run 持久化 cooker 层,依赖不变时该层 cache-hit(秒级),只编 app。
# 用 GitHub Releases 预编译二进制(不 cargo install:后者编译 cargo-chef 自身
# 依赖要 2-3min,且在 QEMU 下 SIGSEGV)。与 dx/tailwind 同模式:按架构选 tarball。
ARG CHEF_VERSION=0.1.77
RUN ARCH="$(dpkg --print-architecture)" \
    && case "$ARCH" in \
        amd64) CHEF_TRIPLET=x86_64-unknown-linux-musl   ;; \
        arm64) CHEF_TRIPLET=aarch64-unknown-linux-musl ;; \
        *) echo "unsupported arch: $ARCH" >&2; exit 1; \
    esac \
    && CHEF_URL="${GH_PROXY:+${GH_PROXY}/}https://github.com/LukeMathWalker/cargo-chef/releases/download/v${CHEF_VERSION}/cargo-chef-${CHEF_TRIPLET}.tar.xz" \
    && curl -fsSL --retry 5 --retry-delay 5 --retry-all-errors "${CHEF_URL}" -o /tmp/chef.tar.xz \
    && tar -xJf /tmp/chef.tar.xz -C /usr/local/bin --strip-components=1 \
    && rm /tmp/chef.tar.xz \
    && cargo-chef --version


WORKDIR /build

# Cache the pnpm workspace node_modules by copying only package manifests first.
# Copying every sub-package's manifest + the workspace root lets pnpm install
# everything in one shot; this layer is reused as long as the manifests don't
# change. pnpm-workspace.yaml uses `packages: ['*']`, so ALL sub-package
# manifests must be present before `pnpm install --frozen-lockfile` or pnpm
# only links deps for the manifests it sees and `pnpm -r run build` fails later
# (e.g. mermaid-renderer "Cannot find module 'mermaid'").
# `pnpm-workspace.yaml` declares a patched dep (@tiptap/markdown) pointing at
# `patches/@tiptap__markdown@3.27.3.patch`, so the patches/ tree must be present
# before `pnpm install --frozen-lockfile` or it fails with ENOENT on the patch.
COPY libs/package.json libs/pnpm-workspace.yaml libs/pnpm-lock.yaml libs/
COPY libs/patches/                         libs/patches/
COPY libs/shared/package.json             libs/shared/
COPY libs/tiptap-editor/package.json      libs/tiptap-editor/
COPY libs/codemirror-editor/package.json  libs/codemirror-editor/
COPY libs/lightbox/package.json           libs/lightbox/
COPY libs/xterm-terminal/package.json     libs/xterm-terminal/
COPY libs/mermaid-renderer/package.json   libs/mermaid-renderer/
COPY libs/yggdrasil-core/package.json     libs/yggdrasil-core/
RUN cd libs && pnpm install --frozen-lockfile

# Build-time git info, injected by the caller via --build-arg. `.dockerignore`
# excludes `.git/`, so build.rs can't run `git` inside the container — these
# ARGs are the only channel for git metadata to reach build.rs (it reads them
# via std::env::var, its first-precedence source). Defaults are empty so a
# bare `docker build` without args degrades to "unknown" gracefully (build.rs
# falls back to the git command, then "unknown").
#
# Each ARG → ENV pair is needed: ARG is only visible in Dockerfile RUN
# commands (and build.rs is invoked by cargo, not a RUN), so we export it as
# ENV for the cargo build step to inherit. Both default to empty; Makefile's
# docker/docker-amd64/docker-multiarch targets override them on the host.
ARG YGG_BUILD_GIT_DESCRIBE=""
ARG YGG_BUILD_GIT_HASH=""
ARG YGG_BUILD_GIT_COMMIT_DATE=""
ENV YGG_BUILD_GIT_DESCRIBE=${YGG_BUILD_GIT_DESCRIBE}
ENV YGG_BUILD_GIT_HASH=${YGG_BUILD_GIT_HASH}
ENV YGG_BUILD_GIT_COMMIT_DATE=${YGG_BUILD_GIT_COMMIT_DATE}

# ──────────────────────────────────────────────────────────────────────────
# cargo-chef 依赖分层:把 576 个 Rust 依赖的编译与 app 源码分离。
# planner 只需 Cargo.toml + Cargo.lock 生成 recipe.json(无源码),所以只要
# 依赖清单不变,recipe.json 不变,cooker 层就 cache-hit。CI 用 GHA cache
# (type=gha, mode=max) 跨 run 持久化 cooker 层——改一行 .rs 时,依赖编译
# 从 ~10min 降到 ~0s,只编 app(~2min)。
#
# cook 必须用与最终 cargo build 完全相同的 target/features/rustflags,
# 否则产物目录不匹配、缓存形同虚设。这里镜像后端(server musl release),
# WASM 前端走 dx 不经 cargo-chef(它另走 wasm32 target)。
COPY Cargo.toml Cargo.lock ./
# cargo-chef prepare 运行 `cargo metadata`，后者校验所有 autobins/[[bin]] 源文件
# 物理存在；此刻仅 COPY 了 manifest，需 dummy 入口让 metadata 解析通过。内容无关：
# cook 阶段由 cargo-chef 自带 skeleton 接管编译依赖，下方 `COPY . .` 再以真实源码覆盖。
RUN mkdir -p src/bin \
    && printf 'fn main() {}\n' > src/main.rs \
    && printf 'fn main() {}\n' > src/bin/generate_highlight_css.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    ARCH="$(dpkg --print-architecture)" \
    && case "$ARCH" in \
        amd64) MUSL_TARGET=x86_64-unknown-linux-musl  ;; \
        arm64) MUSL_TARGET=aarch64-unknown-linux-musl ;; \
        *) echo "unsupported arch: $ARCH" >&2; exit 1; \
    esac \
    && cargo chef prepare --recipe-path recipe.json \
    && export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc \
    && export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc \
    && export RUSTFLAGS="-C target-feature=+crt-static -C relocation-model=static" \
    && cargo chef cook --release --target "$MUSL_TARGET" \
        --no-default-features --features server --recipe-path recipe.json \
    && cargo chef cook --no-default-features --features server --recipe-path recipe.json
# 注:host target 的依赖也 cook 一次(dev profile)——cargo doc 走 host(非 musl)

# Copy the rest of the source tree and build everything. 依赖已由 cooker 编好,
# 此后的 cargo build/doc 只增量编译 app 代码。
COPY . .

# Build all 4 JS libs, syntax-highlight CSS, KaTeX CSS + fonts and Tailwind
# stylesheet. These steps produce the contents of the public/ directory.
# Must stay in sync with make build-linux — katex-css was previously missing,
# which left math rendering as bare spans without KaTeX fonts.
RUN make build-libs && make highlight-css && make katex-css && tailwindcss -i input.css -o public/style.css --minify

# Build the client-side Dioxus WASM bundle. We use dx only for the client assets;
# dx's linker wrapper is incompatible with a raw static linker, so the server
# binary is built with plain cargo in the next step. The client build emits a
# ready-to-serve public/ directory under target/dx/yggdrasil/*/web/public.
# restore-webp overwrites dx's re-encoded VP8L .webp stills with the source
# originals — keep in sync with make build-linux, which runs the same target.
RUN dx build @client --release --debug-symbols=false --wasm-js-cfg false && \
    make restore-webp && \
    mkdir -p /build/dist/public && \
    cp -r /build/target/dx/yggdrasil/*/web/public/* /build/dist/public/

# Generate the project's rustdoc API docs and stage them under public/doc so
# they are served at /doc on the live site. Mirrors `make doc` exactly:
# --no-deps skips dependency docs, --document-private-items documents internal
# / private items (this is effectively a single-crate binary — without it the
# pages would be nearly empty), RUSTDOCFLAGS pins the ayu theme. The tiny
# index.html redirects bare /doc to the real crate entry yggdrasil/.
# NOTE: .dockerignore excludes host-side public/doc/, so this RUN is the ONLY
# channel that puts the docs into the image — without it /doc is a 404 online.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    RUSTDOCFLAGS="--default-theme=ayu" cargo doc --no-deps --document-private-items && \
    rm -rf dist/public/doc && \
    cp -r target/doc dist/public/doc && \
    printf '<!DOCTYPE html><html><head><meta charset="utf-8"><meta http-equiv="refresh" content="0;url=yggdrasil/index.html"><title>Redirecting…</title></head><body><script>location.replace("yggdrasil/index.html")</script></body></html>' > dist/public/doc/index.html

# Build the server as a fully static musl binary, **natively for the buildx
# platform leg**. Each leg builds only its own arch, so musl-gcc (which Debian
# ships for the host arch only) and the target always match — no cross-compiler,
# no QEMU. Cross-compiling here (e.g. building the x86_64 musl target from an
# arm64 leg) breaks ring: cc-rs emits -m64 for the x86_64 target and hands it to
# the arm64 musl-gcc, whose cc1 has no -m64 → "unrecognized command-line option".
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    ARCH="$(dpkg --print-architecture)" \
    && case "$ARCH" in \
        amd64) MUSL_TARGET=x86_64-unknown-linux-musl  ;; \
        arm64) MUSL_TARGET=aarch64-unknown-linux-musl ;; \
        *) echo "unsupported arch: $ARCH" >&2; exit 1 ;; \
    esac \
    && export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc \
    && export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc \
    && export RUSTFLAGS="-C target-feature=+crt-static -C relocation-model=static" \
    && cargo build --release --target "$MUSL_TARGET" --no-default-features --features server

# Ensure the uploads/backups directories exist for runtime. backups/ holds DB
# dump files written by the backup feature; without a nobody-owned dir here the
# scratch runtime (USER 65534) cannot create it under root-owned /app.
RUN mkdir -p uploads backups

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
COPY --from=builder --chown=65534:65534 /build/backups /app/backups

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
