SHELL := /bin/bash
.ONESHELL:
outdir ?= $(CURDIR)/out
spec ?= complete-album.spec

srpm:
	set -euo pipefail
	dnf -y install cargo rust gcc make perl tar xz rpm-build
	version=$$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
	test -n "$$version"
	rm -rf build
	mkdir -p build/complete-album-$$version "$(outdir)"
	tar --exclude=.git --exclude=target --exclude=build -cf - . \
		| tar -xf - -C build/complete-album-$$version
	tar -C build -czf build/complete-album-$$version.tar.gz complete-album-$$version
	cargo vendor build/vendor >/dev/null
	tar -C build -cJf build/vendor.tar.xz vendor
	sed "s/^Version:.*/Version: $$version/" "$(spec)" > build/complete-album.spec
	rpmbuild -bs \
		--define "_sourcedir $(CURDIR)/build" \
		--define "_srcrpmdir $(outdir)" \
		build/complete-album.spec
