.PHONY: dev build build-linux css css-watch clean build-editor build-editor-incremental build-lightbox build-lightbox-incremental build-core build-core-incremental build-codemirror build-codemirror-incremental highlight-css test doc doc-open start clippy fix

build:
	@$(MAKE) build-editor
	@$(MAKE) build-lightbox
	@$(MAKE) build-core
	@$(MAKE) build-codemirror
	@$(MAKE) highlight-css
	@tailwindcss -i input.css -o public/style.css --minify
	@$(MAKE) doc
	@dx build --release --debug-symbols=false

build-linux:
	@$(MAKE) build-editor
	@$(MAKE) build-lightbox
	@$(MAKE) build-core
	@$(MAKE) build-codemirror
	@$(MAKE) highlight-css
	@tailwindcss -i input.css -o public/style.css --minify
	@dx build @client --release --debug-symbols=false --wasm-js-cfg false
	@dx build @server --release --debug-symbols=false --target x86_64-unknown-linux-musl --wasm-js-cfg false --features server
	@echo ""
	@echo "Linux build complete! The server binary is at target/dx/yggdrasil/release/web/server"
	@echo "Remember to deploy it alongside the target/dx/yggdrasil/release/web/public directory."
	@echo "When running the server, ensure DIOXUS_ASSET_DIR is set or the public directory is in CWD."

highlight-css:
	@cargo run --bin generate_highlight_css

build-editor:
	@echo "Building Tiptap editor..."
	@cd libs/tiptap-editor && pnpm ci --include=dev && pnpm run build
	@echo "Tiptap editor built."

# dev 用的增量构建：跳过 pnpm ci（假设 node_modules 已存在），仅 vite build。
# 与 build-editor 分开，避免每次 make dev 都重装依赖。
build-editor-incremental:
	@cd libs/tiptap-editor && pnpm run build

build-lightbox:
	@echo "Building Lightbox..."
	@cd libs/lightbox && pnpm install && pnpm run build
	@echo "Lightbox built."

# dev 用的增量构建：跳过 pnpm ci（假设 node_modules 已存在），仅 vite build。
build-lightbox-incremental:
	@cd libs/lightbox && pnpm run build

build-core:
	@echo "Building yggdrasil-core..."
	@cd libs/yggdrasil-core && pnpm install && pnpm run build
	@echo "yggdrasil-core built."

# dev 用的增量构建：跳过 pnpm install（假设 node_modules 已存在），仅 vite build。
build-core-incremental:
	@cd libs/yggdrasil-core && pnpm run build

build-codemirror:
	@echo "Building CodeMirror editor..."
	@cd libs/codemirror-editor && pnpm ci --include=dev && pnpm run build
	@echo "CodeMirror editor built."

# dev 用的增量构建：跳过 pnpm ci（假设 node_modules 已存在），仅 vite build。
build-codemirror-incremental:
	@cd libs/codemirror-editor && pnpm run build

dev: build-editor-incremental build-lightbox-incremental build-core-incremental build-codemirror-incremental
	@echo "Cleaning static/..."
	@rm -rf static/
	@echo "Building Tiptap editor (incremental)..."
	@echo "Starting tailwindcss watch and dx serve..."
	@tailwindcss -i input.css -o public/style.css --watch & \
	TAILWIND_PID=$$!; \
	trap 'kill $$TAILWIND_PID 2>/dev/null; exit' INT TERM EXIT; \
	SSR_CACHE_SECS=0 dx serve --addr 0.0.0.0

css:
	@tailwindcss -i input.css -o public/style.css

css-watch:
	@tailwindcss -i input.css -o public/style.css --watch

test:
	@cargo test
	@cd libs/tiptap-editor && pnpm test
	@cd libs/lightbox && pnpm test
	@cd libs/yggdrasil-core && pnpm test
	@cd libs/codemirror-editor && pnpm test

clippy:
	@cargo clippy --all-targets --all-features -- -D warnings

fix:
	@cargo fix --allow-dirty

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

clean:
	@cargo clean
	@rm -f public/style.css public/highlight.css
	@rm -rf public/doc
	@rm -rf uploads/.cache
