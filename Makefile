.PHONY: dev build build-linux css css-watch clean build-editor highlight-css test

build:
	@$(MAKE) build-editor
	@$(MAKE) highlight-css
	@npx tailwindcss -i input.css -o public/style.css --minify
	@dx build --release --debug-symbols=false

build-linux:
	@$(MAKE) build-editor
	@$(MAKE) highlight-css
	@npx tailwindcss -i input.css -o public/style.css --minify
	@dx build --release --debug-symbols=false --target x86_64-unknown-linux-musl

highlight-css:
	@cargo run --bin generate_highlight_css

build-editor:
	@echo "Building Tiptap editor..."
	@cd libs/tiptap-editor && npm ci && npx vite build
	@echo "Tiptap editor built."

dev:
	@echo "Starting tailwindcss watch and dx serve..."
	@npx tailwindcss -i input.css -o public/style.css --watch & \
	TAILWIND_PID=$$!; \
	trap 'kill $$TAILWIND_PID 2>/dev/null; exit' INT TERM EXIT; \
	dx serve --addr 0.0.0.0

css:
	@npx tailwindcss -i input.css -o public/style.css

css-watch:
	@npx tailwindcss -i input.css -o public/style.css --watch

test:
	@cargo test

clean:
	@cargo clean
	@rm -f public/style.css
