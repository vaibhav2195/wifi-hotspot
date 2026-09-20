#!/usr/bin/env bash
set -e

if [ "$(id -u)" -ne 0 ]; then
    echo "This script must be run as root (use sudo ./install.sh)" >&2
    exit 1
fi

echo "Building release binary..."
cargo build --release

echo "Installing files to /usr..."
install -d /usr/bin /usr/share/applications /usr/share/polkit-1/actions
install -m 755 target/release/wifi /usr/bin/wifi-hotspot
install -m 755 deb_package/usr/bin/wifi-hotspot-launcher /usr/bin/wifi-hotspot-launcher
install -m 644 deb_package/usr/share/applications/wifi-hotspot.desktop /usr/share/applications/wifi-hotspot.desktop
install -m 644 deb_package/usr/share/polkit-1/actions/com.wifi.hotspot.policy /usr/share/polkit-1/actions/com.wifi.hotspot.policy

echo "Wi-Fi Hotspot successfully installed!"
echo "You can launch it from your application launcher or by running: wifi-hotspot-launcher"
