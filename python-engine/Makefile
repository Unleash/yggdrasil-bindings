SHELL := /bin/bash

PACKAGE = yggdrasil_engine
SOURCES = yggdrasil_engine tests scripts

.PHONY: install format fmt lint test build clean

install:
	poetry install --with dev

fmt:
	poetry run black $(SOURCES)
	poetry run ruff check --fix $(SOURCES)

lint:
	poetry run black --check $(SOURCES)
	poetry run ruff check $(SOURCES)
	poetry run mypy $(PACKAGE)

test:
	poetry run pytest

build:
	bash ./build.sh

clean:
	rm -rf dist staging yggdrasil_engine/lib
