.PHONY: dev build build-linux css css-watch clean build-editor build-editor-incremental highlight-css test

build:
	@$(MAKE) build-editor
	@$(MAKE) highlight-css
	@tailwindcss -i input.css -o public/style.css --minify
	@dx build --release --debug-symbols=false

build-linux:
	@$(MAKE) build-editor
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

dev: build-editor-incremental
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

clean:
	@cargo clean
	@rm -f public/style.css public/highlight.css
	@rm -rf uploads/.cache
