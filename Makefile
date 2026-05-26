.PHONY: dev build css css-watch clean build-editor

build:
	@$(MAKE) build-editor
	@tailwindcss -i input.css -o public/style.css --minify
	@dx build --release

build-editor:
	@echo "Building Tiptap editor..."
	@cd libs/tiptap-editor && npm install && npx vite build
	@mv public/tiptap/editor.iife.js public/tiptap/editor.js 2>/dev/null || true
	@mv public/tiptap/editor.iife.js.map public/tiptap/editor.js.map 2>/dev/null || true
	@echo "Tiptap editor built."

dev:
	@echo "Starting tailwindcss watch and dx serve..."
	@tailwindcss -i input.css -o public/style.css --watch & \
	TAILWIND_PID=$$!; \
	trap 'kill $$TAILWIND_PID 2>/dev/null; exit' INT TERM EXIT; \
	dx serve

css:
	@tailwindcss -i input.css -o public/style.css

css-watch:
	@tailwindcss -i input.css -o public/style.css --watch

clean:
	@cargo clean
	@rm -f public/style.css
