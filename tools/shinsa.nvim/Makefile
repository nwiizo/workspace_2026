.PHONY: test fmt fmt-check lint check

test:
	nvim --headless --clean -u tests/minimal_init.lua -l tests/run.lua

fmt:
	stylua lua plugin tests

fmt-check:
	stylua --check lua plugin tests

lint:
	luacheck lua plugin tests --globals vim

check: fmt-check lint test
