# Maintainer: Vaibhav Sharma <vaibhav.sharma.2195@gmail.com>
pkgname=wifi-hotspot
pkgver=2.0.0
pkgrel=1
pkgdesc="Lightweight concurrent Linux Wi-Fi Hotspot GUI application (Rust + egui)"
arch=('x86_64' 'aarch64')
url="https://github.com/vaibhav2195/wifi-hotspot"
license=('MIT')
depends=('hostapd' 'dnsmasq' 'iw' 'iproute2' 'iptables' 'polkit')
makedepends=('cargo' 'rust')

build() {
    cargo build --release
}

package() {
    install -Dm755 "target/release/wifi" "$pkgdir/usr/bin/wifi-hotspot"
    install -Dm755 "deb_package/usr/bin/wifi-hotspot-launcher" "$pkgdir/usr/bin/wifi-hotspot-launcher"
    install -Dm644 "deb_package/usr/share/applications/wifi-hotspot.desktop" "$pkgdir/usr/share/applications/wifi-hotspot.desktop"
    install -Dm644 "deb_package/usr/share/polkit-1/actions/com.wifi.hotspot.policy" "$pkgdir/usr/share/polkit-1/actions/com.wifi.hotspot.policy"
    install -Dm644 "LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
