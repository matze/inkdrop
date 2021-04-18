.PHONY: release dev

all: dev

release:
	wasm-pack build --target web --out-name wasm --out-dir static/ .

dev:
	wasm-pack build --dev --target web --out-name wasm --out-dir static/ .
