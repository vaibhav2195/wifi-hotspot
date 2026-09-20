#!/bin/bash
set -e

echo "Building Rust release binary..."
cargo build --release

echo "Preparing package files..."
mkdir -p deb_package/usr/bin
cp target/release/wifi deb_package/usr/bin/wifi-hotspot
chmod +x deb_package/usr/bin/wifi-hotspot deb_package/usr/bin/wifi-hotspot-launcher

echo "Building .deb package..."
VERSION=$(grep -i '^Version:' deb_package/DEBIAN/control | awk '{print $2}')
DEB_NAME="wifi-hotspot_${VERSION}_amd64.deb"
dpkg-deb --build deb_package "$DEB_NAME"

echo "Successfully built $DEB_NAME!"
