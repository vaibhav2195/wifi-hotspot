#!/bin/bash
set -e

echo "Building Rust release binary..."
cargo build --release

echo "Preparing package files..."
mkdir -p deb_package/usr/bin
cp target/release/wifi deb_package/usr/bin/wifi-hotspot
chmod +x deb_package/usr/bin/wifi-hotspot deb_package/usr/bin/wifi-hotspot-launcher

echo "Building .deb package..."
dpkg-deb --build deb_package wifi-hotspot_0.1.0_amd64.deb

echo "Successfully built wifi-hotspot_0.1.0_amd64.deb!"
