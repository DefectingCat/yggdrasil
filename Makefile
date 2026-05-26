.PHONY: dev build css css-watch clean

dev:
	@echo "Starting tailwindcss watch and dx serve..."
	@tailwindcss -i input.css -o public/style.css --watch & \
	TAILWIND_PID=$$!; \
	trap 'kill $$TAILWIND_PID 2>/dev/null; exit' INT TERM EXIT; \
	dx serve

build:
	@tailwindcss -i input.css -o public/style.css --minify
	@dx build --release

css:
	@tailwindcss -i input.css -o public/style.css

css-watch:
	@tailwindcss -i input.css -o public/style.css --watch

clean:
	@cargo clean
	@rm -f public/style.css
