.PHONY: test fmt fmt-check lint check

test:
	nvim --headless --clean -u tests/minimal_init.lua -l tests/run.lua

fmt:
	stylua lua tests

fmt-check:
	stylua --check lua tests

lint:
	luacheck lua tests --globals vim

check: fmt-check lint test
