TARGET = wasm32-unknown-unknown
WASM_BINDGEN_OPTS = --no-typescript --out-dir static/
WASM_APP_OPTS = $(WASM_BINDGEN_OPTS) --target web
WASM_WORKER_OPTS = $(WASM_BINDGEN_OPTS) --target no-modules
WASM_TARGET_PATH = ../target/$(TARGET)
CARGO_OPTS = --target $(TARGET)

.PHONY: release dev

all: dev

release:
	cargo build $(CARGO_OPTS) --release --bin app
	cargo build $(CARGO_OPTS) --release --bin worker
	wasm-bindgen $(WASM_APP_OPTS) --out-name app $(WASM_TARGET_PATH)/release/app.wasm
	wasm-bindgen $(WASM_WORKER_OPTS) --out-name worker $(WASM_TARGET_PATH)/release/worker.wasm
	wasm-opt -Os static/app_bg.wasm -o static/app_bg.wasm
	wasm-opt -Os static/worker_bg.wasm -o static/worker_bg.wasm

dev:
	cargo build $(CARGO_OPTS) --bin app
	cargo build $(CARGO_OPTS) --bin worker
	wasm-bindgen $(WASM_APP_OPTS) --out-name app $(WASM_TARGET_PATH)/debug/app.wasm
	wasm-bindgen $(WASM_WORKER_OPTS) --out-name worker $(WASM_TARGET_PATH)/debug/worker.wasm
