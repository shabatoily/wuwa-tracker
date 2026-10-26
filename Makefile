.PHONY: help setup webui-install build-webui check-webui webui-dev fmt check-fmt check check-cli clippy test audit ci build release run serve version release-dry-run bump-patch bump-minor bump-major release-tag clean distclean

APP := wuwa-tracker
CLI_BIN := wuwa-tracker-cli
WEBUI_DIR := crates/wuwa-tracker-webui
HOST ?= 127.0.0.1
PORT ?= 3000
WEBUI ?= 0
CARGO ?= cargo
TRUNK ?= trunk
WASM_TOOLCHAIN ?= 1.96.0
RUSTUP ?= $(shell if test -x /opt/homebrew/opt/rustup/bin/rustup; then echo /opt/homebrew/opt/rustup/bin/rustup; else command -v rustup 2>/dev/null || echo rustup; fi)
RUSTUP_BIN_DIR := $(dir $(RUSTUP))
WASM_ENV := PATH="$(RUSTUP_BIN_DIR):$$PATH" RUSTUP_TOOLCHAIN=$(WASM_TOOLCHAIN) NO_COLOR=false
SERVE_WEBUI := $(if $(filter 1 true yes,$(WEBUI)),--webui,)

help:
	@echo "Wuwa Tracker"
	@echo ""
	@echo "Development:"
	@echo "  make help            Show available commands and options"
	@echo "  make setup           Install the Rust WASM toolchain and target; check Trunk"
	@echo "  make run             Build WebUI and run Tauri GUI"
	@echo "  make serve           Run Axum API server"
	@echo "  make webui-dev       Run Trunk dev server"
	@echo ""
	@echo "Checks:"
	@echo "  make fmt             Format Rust code"
	@echo "  make check-fmt       Check Rust formatting"
	@echo "  make check-webui     Check WebUI types"
	@echo "  make check           cargo check --workspace"
	@echo "  make check-cli       Check the CLI-only build"
	@echo "  make clippy          Check all workspace targets with warnings denied"
	@echo "  make test            cargo test --workspace"
	@echo "  make audit           Audit Cargo dependencies for vulnerabilities"
	@echo "  make ci              check-fmt + build-webui + check-cli + check-webui + check + clippy + audit + test"
	@echo ""
	@echo "Build:"
	@echo "  make webui-install   Check Trunk and the Rust WASM target"
	@echo "  make build-webui     Build release WebUI assets with Trunk"
	@echo "  make build           Build WebUI and debug Rust binaries"
	@echo "  make release         Build WebUI and release Rust binaries"
	@echo ""
	@echo "Versioning:"
	@echo "  make version         Print Cargo package version"
	@echo "  make release-dry-run Preview cargo-release changes"
	@echo "  make bump-patch      Bump patch version and create release commit"
	@echo "  make bump-minor      Bump minor version and create release commit"
	@echo "  make bump-major      Bump major version and create release commit"
	@echo "  make release-tag     Tag and push the version from synced main"
	@echo ""
	@echo "Cleanup:"
	@echo "  make clean           Remove Cargo build artifacts and WebUI dist"
	@echo "  make distclean       Alias for clean"
	@echo ""
	@echo "Options:"
	@echo "  make serve HOST=127.0.0.1 PORT=3000 WEBUI=1"
	@echo "  make setup WASM_TOOLCHAIN=1.96.0"

setup:
	$(RUSTUP) toolchain install $(WASM_TOOLCHAIN) --profile minimal --target wasm32-unknown-unknown
	@command -v $(TRUNK) >/dev/null || { echo "trunk is required: cargo install trunk --locked"; exit 1; }

webui-install:
	@command -v $(TRUNK) >/dev/null || { echo "trunk is required: run make setup after installing Trunk"; exit 1; }
	@$(RUSTUP) target list --installed --toolchain $(WASM_TOOLCHAIN) | grep -qx wasm32-unknown-unknown || { echo "wasm32-unknown-unknown is required: run make setup"; exit 1; }

build-webui: webui-install
	cd $(WEBUI_DIR) && $(WASM_ENV) $(TRUNK) build --release

check-webui: webui-install
	$(WASM_ENV) $(RUSTUP) run $(WASM_TOOLCHAIN) cargo check -p wuwa-tracker-webui --target wasm32-unknown-unknown

webui-dev: webui-install
	cd $(WEBUI_DIR) && $(WASM_ENV) $(TRUNK) serve

fmt:
	$(CARGO) fmt --all

check-fmt:
	$(CARGO) fmt --all -- --check

check:
	$(CARGO) check --workspace

check-cli:
	$(CARGO) check -p $(APP) --no-default-features --bin $(CLI_BIN)

clippy:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

test:
	$(CARGO) test --workspace

audit:
	$(CARGO) audit

ci: check-fmt build-webui
	$(MAKE) check-cli
	$(MAKE) check-webui
	$(MAKE) check
	$(MAKE) clippy
	$(MAKE) audit
	$(MAKE) test

build: build-webui
	$(CARGO) build --workspace
	$(CARGO) build -p $(APP) --no-default-features --bin $(CLI_BIN)

release: build-webui
	$(CARGO) build --release -p $(APP) --bin $(APP)
	$(CARGO) build --release -p $(APP) --no-default-features --bin $(CLI_BIN)

run: build-webui
	$(CARGO) run -p $(APP)

serve:
	$(CARGO) run -p $(APP) --no-default-features --bin $(CLI_BIN) -- serve --host $(HOST) --port $(PORT) $(SERVE_WEBUI)

version:
	@$(CARGO) pkgid -p $(APP) | sed 's/.*#//; s/.*@//'

release-dry-run:
	$(CARGO) release patch --workspace --no-tag --dry-run

bump-patch:
	$(CARGO) release patch --workspace --no-tag --execute

bump-minor:
	$(CARGO) release minor --workspace --no-tag --execute

bump-major:
	$(CARGO) release major --workspace --no-tag --execute

release-tag:
	@git diff --quiet || { echo "Working tree has unstaged changes."; exit 1; }
	@git diff --cached --quiet || { echo "Index has staged changes."; exit 1; }
	@branch="$$(git branch --show-current)"; \
	if [ "$$branch" != "main" ]; then \
		echo "release-tag must run on main."; \
		exit 1; \
	fi
	@git fetch origin main --tags
	@local_head="$$(git rev-parse HEAD)"; \
	remote_head="$$(git rev-parse origin/main)"; \
	if [ "$$local_head" != "$$remote_head" ]; then \
		echo "main is not synced with origin/main. Push or merge the release commit before tagging."; \
		exit 1; \
	fi
	@version="$$($(CARGO) pkgid -p $(APP) | sed 's/.*#//; s/.*@//')"; \
	tag="v$$version"; \
	if git rev-parse -q --verify "refs/tags/$$tag" >/dev/null; then \
		echo "Tag $$tag already exists."; \
		exit 1; \
	fi; \
	git tag "$$tag"; \
	git push origin "$$tag"; \
	echo "Pushed $$tag."

clean:
	$(CARGO) clean
	rm -rf $(WEBUI_DIR)/dist

distclean: clean
