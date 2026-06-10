SHELL := /bin/sh

NPM ?= npm
HOST ?= 127.0.0.1
PORT ?= 5173
NODE_MODULES := node_modules/.package-lock

.PHONY: help install dev start build preview clean

help:
	@printf '%s\n' 'TurboSync website targets:'
	@printf '%s\n' '  make install   Install website dependencies'
	@printf '%s\n' '  make dev       Start the Vite dev server'
	@printf '%s\n' '  make start     Alias for make dev'
	@printf '%s\n' '  make build     Type-check and build the website'
	@printf '%s\n' '  make preview   Preview the production build'
	@printf '%s\n' '  make clean     Remove build output'

$(NODE_MODULES): package.json
	$(NPM) install

dev: $(NODE_MODULES)
	$(NPM) run dev -- --host $(HOST) --port $(PORT)

start: dev

install:
	$(NPM) install

build: $(NODE_MODULES)
	$(NPM) run build

preview: $(NODE_MODULES)
	$(NPM) run preview -- --host $(HOST) --port $(PORT)

clean:
	rm -rf dist
