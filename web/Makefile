OPTS = --target web --no-typescript --out-name wasm --out-dir static/ .

.PHONY: release dev

all: dev

release:
	wasm-pack build --release $(OPTS)

dev:
	wasm-pack build --dev $(OPTS)
