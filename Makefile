PREFIX ?= /usr
DESTDIR ?=

.PHONY: all build install uninstall clean

all: build

build:
	cargo build --release

install: build
	install -d $(DESTDIR)$(PREFIX)/bin
	install -d $(DESTDIR)$(PREFIX)/share/applications
	install -d $(DESTDIR)$(PREFIX)/share/polkit-1/actions
	install -m 755 target/release/wifi $(DESTDIR)$(PREFIX)/bin/wifi-hotspot
	install -m 755 deb_package/usr/bin/wifi-hotspot-launcher $(DESTDIR)$(PREFIX)/bin/wifi-hotspot-launcher
	install -m 644 deb_package/usr/share/applications/wifi-hotspot.desktop $(DESTDIR)$(PREFIX)/share/applications/wifi-hotspot.desktop
	install -m 644 deb_package/usr/share/polkit-1/actions/com.wifi.hotspot.policy $(DESTDIR)$(PREFIX)/share/polkit-1/actions/com.wifi.hotspot.policy

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/wifi-hotspot
	rm -f $(DESTDIR)$(PREFIX)/bin/wifi-hotspot-launcher
	rm -f $(DESTDIR)$(PREFIX)/share/applications/wifi-hotspot.desktop
	rm -f $(DESTDIR)$(PREFIX)/share/polkit-1/actions/com.wifi.hotspot.policy

clean:
	cargo clean
