.PHONY: dev build build-linux css css-watch clean build-editor build-editor-incremental build-lightbox build-lightbox-incremental build-core build-core-incremental highlight-css test doc doc-open

build:
	@$(MAKE) build-editor
	@$(MAKE) build-lightbox
	@$(MAKE) build-core
	@$(MAKE) highlight-css
	@tailwindcss -i input.css -o public/style.css --minify
	@dx build --release --debug-symbols=false

build-linux:
	@$(MAKE) build-editor
	@$(MAKE) build-lightbox
	@$(MAKE) build-core
	@$(MAKE) highlight-css
	@tailwindcss -i input.css -o public/style.css --minify
	@dx build @client --release --debug-symbols=false --wasm-js-cfg false
	@dx build @server --release --debug-symbols=false --target x86_64-unknown-linux-musl --wasm-js-cfg false --features server

highlight-css:
	@cargo run --bin generate_highlight_css

build-editor:
	@echo "Building Tiptap editor..."
	@cd libs/tiptap-editor && npm ci --include=dev && npm run build
	@echo "Tiptap editor built."

# dev 用的增量构建：跳过 npm ci（假设 node_modules 已存在），仅 vite build。
# 与 build-editor 分开，避免每次 make dev 都重装依赖。
build-editor-incremental:
	@cd libs/tiptap-editor && npm run build

build-lightbox:
	@echo "Building Lightbox..."
	@cd libs/lightbox && npm install && npm run build
	@echo "Lightbox built."

# dev 用的增量构建：跳过 npm ci（假设 node_modules 已存在），仅 vite build。
build-lightbox-incremental:
	@cd libs/lightbox && npm run build

build-core:
	@echo "Building yggdrasil-core..."
	@cd libs/yggdrasil-core && npm install && npm run build
	@echo "yggdrasil-core built."

# dev 用的增量构建：跳过 npm install（假设 node_modules 已存在），仅 vite build。
build-core-incremental:
	@cd libs/yggdrasil-core && npm run build

dev: build-editor-incremental build-lightbox-incremental build-core-incremental
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
	@cd libs/tiptap-editor && npm test
	@cd libs/lightbox && npm test
	@cd libs/yggdrasil-core && npm test

# 只编译当前 crate 的文档（--no-deps 跳过依赖，--document-private-items
# 让纯 binary crate 的内部模块/私有项也进文档，否则页面基本是空的）。
# RUSTDOCFLAGS 把 rustdoc 的 --default-theme=ayu 透传过去——cargo doc 本身
# 无主题参数，但会把该环境变量转交给底层 rustdoc。注意它是默认值，浏览器
# 若已记住上次的主题选择（localStorage）则不会被覆盖。
doc:
	@RUSTDOCFLAGS="--default-theme=ayu" cargo doc --no-deps --document-private-items

# 同 doc，生成完自动用浏览器打开。
doc-open:
	@RUSTDOCFLAGS="--default-theme=ayu" cargo doc --no-deps --document-private-items --open

clean:
	@cargo clean
	@rm -f public/style.css public/highlight.css
	@rm -rf uploads/.cache
