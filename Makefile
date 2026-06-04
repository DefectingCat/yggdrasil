.PHONY: dev build css css-watch clean build-editor highlight-css

build:
	@$(MAKE) build-editor
	@$(MAKE) highlight-css
	@tailwindcss -i input.css -o public/style.css --minify
	@dx build --release

highlight-css:
	@cargo run --bin generate_highlight_css

build-editor:
	@echo "Building Tiptap editor..."
	@cd libs/tiptap-editor && npm install && npx vite build
	@echo "Tiptap editor built."

dev:
	@echo "Starting tailwindcss watch and dx serve..."
	@tailwindcss -i input.css -o public/style.css --watch & \
	TAILWIND_PID=$$!; \
	trap 'kill $$TAILWIND_PID 2>/dev/null; exit' INT TERM EXIT; \
	dx serve --addr 0.0.0.0

css:
	@tailwindcss -i input.css -o public/style.css

css-watch:
	@tailwindcss -i input.css -o public/style.css --watch

clean:
	@cargo clean
	@rm -f public/style.css
