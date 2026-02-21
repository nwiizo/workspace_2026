APP_NAME = yasume
APP_DIR = $(APP_NAME).app
CONTENTS = $(APP_DIR)/Contents
MACOS = $(CONTENTS)/MacOS

.PHONY: build bundle install clean

build:
	cargo build --release

bundle: build
	mkdir -p "$(MACOS)"
	cp Info.plist "$(CONTENTS)/Info.plist"
	cp target/release/yasume "$(MACOS)/yasume"
	cp target/release/yasume-ctl "$(MACOS)/yasume-ctl"
	@echo "Created $(APP_DIR)"

install: bundle
	cp -r "$(APP_DIR)" /Applications/
	@echo "Installed to /Applications/$(APP_DIR)"

clean:
	cargo clean
	rm -rf "$(APP_DIR)"
