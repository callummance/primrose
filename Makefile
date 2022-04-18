VERSION := $(shell awk '/^version = .*/ { gsub(/"/, "", $$3); print $$3 }' < Cargo.toml)
BIN = target/release/primrose
PKG_BUILD_DIR = build


.PHONY: clean all deb

all: $(BIN)

$(PKG_BUILD_DIR):
	mkdir $(PKG_BUILD_DIR)


#Debian build
DEB_BUILD_DIR = $(PKG_BUILD_DIR)/primrose_$(VERSION)-1_amd64

$(DEB_BUILD_DIR): $(PKG_BUILD_DIR)
	mkdir -p $(DEB_BUILD_DIR)

deb: $(BIN) $(DEB_BUILD_DIR) packages/debian/*
	mkdir -p $(DEB_BUILD_DIR)/etc/systemd/system
	mkdir -p $(DEB_BUILD_DIR)/usr/local/bin
	mkdir -p $(DEB_BUILD_DIR)/DEBIAN
	cp $(BIN) $(DEB_BUILD_DIR)/usr/local/bin/primrose
	cp packages/primrose.service $(DEB_BUILD_DIR)/etc/systemd/system/primrose.service
	cp packages/debian/* $(DEB_BUILD_DIR)/DEBIAN/
	chmod 755 $(DEB_BUILD_DIR)/DEBIAN/postinst
	dpkg-deb --build --root-owner-group $(DEB_BUILD_DIR)



#Main binary build
$(BIN): src/*.rs
	cargo build --release

clean:
	cargo clean
	rm -rf build