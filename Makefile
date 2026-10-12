.PHONY: setup run app test format lint icon

# rustup puts cargo here. Some shells do not have this folder on the PATH.
export PATH := $(HOME)/.cargo/bin:$(PATH)

# Install the two dev tools: the Tauri CLI and Biome.
setup:
	bun install

# Run the app. It builds again when the Rust code changes.
run:
	bun tauri dev

# Build the app and its installers for this computer. They are in src-tauri/target/release/bundle.
app:
	bun tauri build

test:
	cd src-tauri && cargo test

format:
	cd src-tauri && cargo fmt
	bun run format

lint:
	cd src-tauri && cargo fmt --check
	cd src-tauri && cargo clippy --all-targets -- -D warnings
	bun run lint

# Rebuild the app icons from Resources/Logo.png. That file is exported from the Pencil design.
icon:
	bun tauri icon Resources/Logo.png
	rm -rf src-tauri/icons/android src-tauri/icons/ios
	cp "src-tauri/icons/128x128@2x.png" ui/assets/logo.png
