.PHONY: dev build build-linux build-freebsd freebsd-sysroot docker docker-amd64 docker-apple docker-multiarch css css-watch clean build-libs build-editor build-codemirror build-lightbox build-core highlight-css test doc doc-open start lint fix restore-webp

build:
	@cd libs && pnpm install --frozen-lockfile
	@$(MAKE) build-libs
	@$(MAKE) highlight-css
	@tailwindcss -i input.css -o public/style.css --minify
	@$(MAKE) doc
	@dx build --release --debug-symbols=false
	@$(MAKE) restore-webp

build-linux:
	@cd libs && pnpm install --frozen-lockfile
	@$(MAKE) build-libs
	@$(MAKE) highlight-css
	@tailwindcss -i input.css -o public/style.css --minify
	@dx build @client --release --debug-symbols=false --wasm-js-cfg false
	@dx build @server --release --debug-symbols=false --target x86_64-unknown-linux-musl --wasm-js-cfg false --features server
	@$(MAKE) restore-webp
	@echo ""
	@echo "Linux build complete! The server binary is at target/dx/yggdrasil/release/web/server"
	@echo "Remember to deploy it alongside the target/dx/yggdrasil/release/web/public directory."
	@echo "When running the server, ensure DIOXUS_ASSET_DIR is set or the public directory is in CWD."

# FreeBSD 15.1 base.txz 版本与下载源。sysroot 仅需 ./lib 与 ./usr/lib（crt 对象 + 系统库）。
FREEBSD_VERSION ?= 15.1-RELEASE
FREEBSD_BASE_URL ?= https://download.freebsd.org/ftp/releases/amd64/amd64/$(FREEBSD_VERSION)/base.txz
FREEBSD_SYSROOT := $(CURDIR)/.freebsd-sysroot

# 下载并解压 FreeBSD base.txz 到 .freebsd-sysroot/，供交叉链接（crt 对象 + 系统库）。
# 幂等：若 sysroot 已存在则跳过下载。
freebsd-sysroot:
	@if [ -d "$(FREEBSD_SYSROOT)/usr/lib" ] && [ -d "$(FREEBSD_SYSROOT)/lib" ]; then \
		echo "FreeBSD sysroot already present at $(FREEBSD_SYSROOT)"; \
	else \
		echo "Downloading FreeBSD $(FREEBSD_VERSION) base.txz..."; \
		curl -fL --retry 3 -o /tmp/freebsd-base.txz "$(FREEBSD_BASE_URL)"; \
		mkdir -p "$(FREEBSD_SYSROOT)"; \
		echo "Extracting crt objects and system libs..."; \
		tar -xf /tmp/freebsd-base.txz -C "$(FREEBSD_SYSROOT)" ./lib ./usr/lib; \
		rm -f /tmp/freebsd-base.txz; \
		echo "FreeBSD sysroot ready at $(FREEBSD_SYSROOT)"; \
	fi

# 交叉编译 FreeBSD x86_64 release server 二进制。
# 前置：clang + lld 已装（pacman -S clang lld）、rustup target add x86_64-unknown-freebsd、
# `make freebsd-sysroot`。sysroot 路径经 CARGO_TARGET_*_RUSTFLAGS 注入，避免在
# .cargo/config.toml 里硬编码机器相关路径。server 二进制用 cargo 直出（dx CLI 对该
# target 未经验证）；前端 wasm 与静态资源与 build-linux 相同，不在此重复构建。
build-freebsd:
	@$(MAKE) freebsd-sysroot
	@SYSROOT="$(FREEBSD_SYSROOT)"; \
	RUSTFLAGS_FREEBSD="-C linker=clang -C link-arg=--target=x86_64-unknown-freebsd -C link-arg=-fuse-ld=lld -C link-arg=--sysroot=$$SYSROOT -C link-arg=-L$$SYSROOT/usr/lib -C link-arg=-L$$SYSROOT/lib"; \
	echo "Cross-compiling yggdrasil server for FreeBSD x86_64..."; \
	CARGO_TARGET_X86_64_UNKNOWN_FREEBSD_RUSTFLAGS="$$RUSTFLAGS_FREEBSD" \
		cargo build --release --target x86_64-unknown-freebsd --features server --bin yggdrasil
	@echo ""
	@echo "FreeBSD build complete! Server binary: target/x86_64-unknown-freebsd/release/yggdrasil"
	@echo "Deploy it to FreeBSD 15+ alongside the static public/ directory."
	@echo "Runtime needs (bundled in FreeBSD base): libc.so.7 libthr.so.3 libkvm.so.7 etc."

# 兜底：dx build 0.7.9 会把 public/ 下的 .webp 重编码成 VP8L 无损静图
# （动画帧被丢弃，静图体积反增 7-8 倍），与文档承诺的"原样拷贝"不符。
# SVG/ICO 等其他格式不受影响，故只需覆盖 .webp。
# 遍历所有 dx 产物目录（release/debug），用源 public/ 的同名文件覆盖回去。
# 仅覆盖产物中已存在的 .webp，不引入源里新增但 dx 未生成的文件。
# 参考：https://dioxuslabs.com/learn/0.7/essentials/ui/assets/
# 上游修复后可移除此 target 及 build/build-linux 里的调用。
restore-webp:
	@find target/dx -type d -path "*/web/public" 2>/dev/null | while read prod; do \
		find "$$prod" -type f -name "*.webp" 2>/dev/null | while read p; do \
			rel=$${p#$$prod/}; \
			src="public/$$rel"; \
			if [ -f "$$src" ]; then \
				cp "$$src" "$$p"; \
			else \
				echo "restore-webp: 源缺失，跳过 $$rel"; \
			fi; \
		done; \
	done

highlight-css:
	@cargo run --bin generate_highlight_css

# 并行构建全部 4 个 libs/ 子项目（pnpm -r 拓扑顺序，无相互依赖则并发）。
# 依赖安装由调用方负责（build/build-linux 用 pnpm install --frozen-lockfile，
# dev 假设 node_modules 已存在）。
build-libs:
	@cd libs && pnpm -r run build

# 单库便利 target（替代旧的 build-<name>，用 pnpm --filter 精确定位）。
build-editor:     ; @cd libs && pnpm --filter @yggdrasil/tiptap-editor run build
build-codemirror: ; @cd libs && pnpm --filter @yggdrasil/codemirror-editor run build
build-lightbox:   ; @cd libs && pnpm --filter @yggdrasil/lightbox run build
build-core:       ; @cd libs && pnpm --filter @yggdrasil/core run build

dev: build-libs highlight-css
	@echo "Cleaning static/..."
	@rm -rf static/
	@echo "Building CSS..."
	@$(MAKE) css
	@echo "Starting dx serve..."
	@SSR_CACHE_SECS=0 dx serve --addr 0.0.0.0

css:
	@tailwindcss -i input.css -o public/style.css

css-watch:
	@tailwindcss -i input.css -o public/style.css --watch

test:
	@cargo test
	@cd libs && pnpm -r run test

# JS + Rust 一次性检查（不改动文件）。
lint:
	@echo "==> Biome check (libs)"
	@cd libs && pnpm exec biome check . && pnpm typecheck
	@echo "==> Cargo clippy (Rust)"
	@cargo clippy --all-targets --all-features -- -D warnings

# JS + Rust 自动修复（直接写入文件）。
# 顺序：Biome → cargo fix（应用编译器建议，重写代码）→ cargo fmt（格式化 Rust）
# → dx fmt（格式化 RSX 宏）。两道格式化收尾，保证最终文件状态整洁。
fix:
	@echo "==> Biome format (libs, 写入文件)"
	@cd libs && pnpm exec biome format --write .
	@echo "==> Cargo fix (Rust, 应用编译器建议)"
	@cargo fix --allow-dirty
	@echo "==> Cargo fmt (Rust, 格式化)"
	@cargo fmt
	@echo "==> Dioxus fmt (RSX 宏, 格式化)"
	@dx fmt

# 只编译当前 crate 的文档（--no-deps 跳过依赖，--document-private-items
# 让纯 binary crate 的内部模块/私有项也进文档，否则页面基本是空的）。
# RUSTDOCFLAGS 把 rustdoc 的 --default-theme=ayu 透传过去——cargo doc 本身
# 无主题参数，但会把该环境变量转交给底层 rustdoc。注意它是默认值，浏览器
# 若已记住上次的主题选择（localStorage）则不会被覆盖。
#
# 生成后拷贝到 public/doc/，让文档随 Dioxus 静态目录发布。先清空旧目录再
# 整体拷贝，避免删除模块后残留旧文件。rustdoc 内部用相对路径引用资源
# （如 ../../static.files/），原样挂载不会断链。
#
# 额外生成 public/doc/index.html 重定向页：Dioxus 在 dev 用
# nest_service("/doc", ServeDir) 托管该目录，ServeDir 访问目录根时默认
# 返回 index.html。用 meta refresh + JS 跳转到真正的文档入口
# yggdrasil/index.html，这样裸路径 /doc 也能直达文档，且不与 Dioxus 的
# /doc/* 路由冲突（手动注册 /doc 会在 merge 时 panic）。
doc:
	@RUSTDOCFLAGS="--default-theme=ayu" cargo doc --no-deps --document-private-items
	@rm -rf public/doc
	@cp -r target/doc public/doc
	@printf '<!DOCTYPE html><html><head><meta charset="utf-8"><meta http-equiv="refresh" content="0;url=yggdrasil/index.html"><title>Redirecting…</title></head><body><script>location.replace("yggdrasil/index.html")</script></body></html>' > public/doc/index.html

# 同 doc，生成完自动用浏览器打开。
doc-open:
	@RUSTDOCFLAGS="--default-theme=ayu" cargo doc --no-deps --document-private-items --open

# Multi-arch image build via buildx. The Dockerfile builds each platform leg
# natively (amd64→x86_64 musl, arm64→aarch64 musl), so no cross-compiler or
# QEMU is needed. Docker Desktop ships a buildx builder that handles this.
#
#   make docker              native arch only, load into local daemon (for testing)
#   make docker-amd64        x86_64 only, load into local daemon (buildx + QEMU on Apple Silicon)
#   make docker-apple        x86_64 only, via Apple Container CLI (macOS 26+, Apple Silicon native; no Docker needed)
#   make docker-multiarch    build amd64+arm64 and push to a registry
#                            (multi-arch manifests can't be --load-ed locally)
#
# Push examples:
#   make docker-multiarch IMAGE=ghcr.io/owner/yggdrasil:latest
#   make docker-multiarch IMAGE=user/yggdrasil:v1 PLATFORMS=linux/amd64
IMAGE ?= yggdrasil
PLATFORMS ?= linux/amd64,linux/arm64
docker:
	@docker buildx build --load -t yggdrasil .

# Cross-build x86_64 into the local daemon. buildx 仿真非原生架构,无需修改 Dockerfile
# (它本就按 dpkg --print-architecture 自适应选 musl target)。Apple Silicon 上走 QEMU,
# 比 native 慢;产物可直接 docker run / docker save 导出。
docker-amd64:
	@docker buildx build --platform linux/amd64 --load -t yggdrasil:amd64 .

# Apple Container CLI 构建 x86_64 镜像。前提:macOS 26 Tahoe + 已安装 `container`。
# 无需 Docker Desktop;使用 --arch 而非 --platform(Apple Container 用裸架构名,OS 固定 linux)。
# 产出标准 OCI 镜像,可 container push 到 registry 后在 Linux 上用 docker compose 运行。
docker-apple:
	@container build --arch amd64 -t yggdrasil:amd64 .

docker-multiarch:
	@docker buildx build --platform $(PLATFORMS) -t $(IMAGE) --push .

clean:
	@cargo clean
	@rm -f public/style.css public/highlight.css
	@rm -rf public/doc
	@rm -rf uploads/.cache
	@rm -rf libs/node_modules libs/*/node_modules
