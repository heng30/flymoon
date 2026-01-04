#!/bin/sh

pwd=${shell pwd}
app-name=flymoon
version=`git describe --tags --abbrev=0`
build-env=SLINT_STYLE=fluent
run-env=RUST_LOG=debug

all: build-release # latex-image-build-release

build-debug:
	$(build-env) cargo build

build-release:
	$(build-env) cargo build --release

debug:
	$(build-env) $(run-env) cargo run --bin ${app-name}

test:
	$(build-env) $(run-env) cargo test -- --nocapture

clean:
	cargo clean

check:
	$(build-env) cargo check --bin ${app-name}

packing-linux:
	cp -f target/release/${app-name} target/${app-name}-${version}-x86_64-linux
	echo "${app-name}-${version}-x86_64-linux" > target/output-name

packing-windows:
	cp -f target/release/${app-name}.exe target/${app-name}-${version}-x86_64-windows.exe
	echo "${app-name}-${version}-x86_64-windows.exe" > target/output-name

packing-darwin:
	cp -f target/release/${app-name} target/${app-name}-${version}-x86_64-darwin
	echo "${app-name}-${version}-x86_64-darwin" > target/output-name

slint-viewer:
	$(build-env) slint-viewer --auto-reload -I $(app-name)/ui ${app-name}/ui/desktop-window.slint

deb:
	cd ./${app-name}/pkg/deb && bash -e "./create_deb.sh"
	mv ./${app-name}/pkg/deb/$(app-name).deb ./target

app-name:
	- mkdir -p target
	echo "$(app-name)" > target/app-name

get-font-name:
	fc-scan ./${app-name}/ui/fonts/*.{ttf,otf} | grep "fullname:"

latex-image-build-release:
	cargo build --release --bin latex-image

