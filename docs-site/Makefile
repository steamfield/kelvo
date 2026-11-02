# statglass-site development commands.
#
# A starting point: adapt the recipes, but keep the target names.
# `stacks verify` calls install, lint, check, test, build and dev, and CI
# should call the same targets.

PORT_WEB ?= 4321
export PORT_WEB

.DEFAULT_GOAL := help
.PHONY: help install demo dev lint check test coverage build preview format clean

help: ## List targets
	@grep -E '^[a-zA-Z0-9_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-14s %s\n", $$1, $$2}'

install: ## Install dependencies (frozen lockfile when CI is set)
	cd .. && bun install $(if $(CI),--frozen-lockfile)
	bun install $(if $(CI),--frozen-lockfile)

demo: ## Build the real app (mock transport) into public/demo; skipped when unchanged
	bun scripts/build-demo.mjs

# --ignore-lock keeps astro dev in the foreground when it detects an agent
# (otherwise it backgrounds itself and exits). Stop a stray one: bunx astro dev stop
dev: demo ## Astro dev server on PORT_WEB
	bunx astro dev --port $(PORT_WEB) --ignore-lock

lint: ## Fail if Prettier would change any file
	bunx prettier --check .

check: ## Type-check and validate content (astro check)
	@bun --no-install -e 'require.resolve("@astrojs/check"); require.resolve("typescript")' >/dev/null 2>&1 || { echo 'Missing checker dependencies. Run bun add -d @astrojs/check typescript, then make check.' >&2; exit 1; }
	bunx --no-install astro check

test: ## Unit and component tests
	bunx vitest run

coverage: ## Tests with coverage
	bunx vitest run --coverage

build: demo ## Static production build into dist/
	bunx astro build

preview: ## Serve the production build
	bunx astro preview --port $(PORT_WEB) --ignore-lock

format: ## Format with Prettier
	bunx prettier --write .

clean: ## Remove build output, the built demo and Astro caches
	rm -rf dist .astro public/demo node_modules/.cache/build-demo.stamp
