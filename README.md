# Concurrent Linux Wi-Fi Hotspot (Rust + egui)

![Rust](https://img.shields.io/badge/Language-Rust-orange.svg)
![Platform](https://img.shields.io/badge/Platform-Linux-blue.svg)
![License](https://img.shields.io/badge/License-MIT-green.svg)

A lightweight, native Linux desktop GUI application built in Rust to create and manage a **Wi-Fi Access Point (Hotspot) while remaining connected to an existing Wi-Fi network (AP/STA Concurrent Mode)**.

Unlike default NetworkManager tools which disconnect your Wi-Fi when activating a hotspot, this application leverages `iw`, `hostapd`, `dnsmasq`, and `iptables` to create a virtual AP interface (`ap0`) alongside your physical station interface.

---

## ✨ Features

- **AP/STA Concurrency**: Share your Wi-Fi internet connection without losing your existing Wi-Fi station connection.
- **Dynamic Interface Detection**: Automatically scans and detects the primary wireless card on any laptop (`wlp0s20f3`, `wlan0`, `wlp2s0`, etc.).
- **Automatic Band & Channel Matching**: Automatically tunes the Hotspot channel and frequency band (2.4 GHz vs 5 GHz) to match your active Wi-Fi connection, preventing hardware channel conflicts.
- **Frequency Band Controls**: Option to configure `Auto`, `Force 2.4 GHz`, or `Force 5 GHz` with custom channel dropdowns.
- **Desktop Application & Polkit Integration**: Built-in `.desktop` entry and PolicyKit rule allowing users to launch the app directly from their Linux application menu with a graphical password prompt.
- **Native GUI**: Built using `eframe` / `egui` for instant startup and low memory usage.

---

## 🛠 System Prerequisites

This application relies on standard Linux networking utilities. Ensure they are installed:

```bash
sudo apt update
sudo apt install hostapd dnsmasq iw iproute2 iptables pkexec
```

---

## 🚀 Installation Options

### Option A: Install via `.deb` Package (Recommended)

1. Build the `.deb` package or download it from releases:
   ```bash
   ./build_deb.sh
   ```
2. Install the package:
   ```bash
   sudo dpkg -i wifi-hotspot_0.1.0_amd64.deb
   sudo apt-get install -f
   ```
3. Launch **Wi-Fi Hotspot** from your Linux application menu!

---

### Option B: Build & Run from Source

1. Ensure you have Rust installed (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`).
2. Build the project:
   ```bash
   cargo build --release
   ```
3. Execute the binary with `sudo` permissions (required for creating network interfaces and configuring IP forwarding/iptables):
   ```bash
   sudo ./target/release/wifi
   ```

---

## ⚡ How It Works Under the Hood

1. **Virtual Interface Creation**: Creates a secondary virtual interface `ap0` attached to the physical PHY (`iw dev <iface> interface add ap0 type __ap`).
2. **NetworkManager Isolation**: Tells NetworkManager to ignore `ap0` (`nmcli device set ap0 managed no`) so it doesn't lock or interfere with the interface.
3. **MAC & IP Allocation**: Assigns a unique MAC address and local subnet (`192.168.50.1/24`).
4. **Hostapd & Dnsmasq**: Generates dynamic configuration files in `/tmp` matching the physical radio's active frequency channel and spawns `hostapd` for WPA2 authentication and `dnsmasq` for DHCP/DNS leases.
5. **NAT & IP Forwarding**: Enables Linux kernel IP forwarding (`sysctl net.ipv4.ip_forward=1`) and adds `iptables` MASQUERADE rules so connected clients receive internet traffic through your main Wi-Fi interface.

---

## ℹ️ Hardware Limitations Note

Most modern Wi-Fi cards support multi-interface concurrency, but have **1 physical channel synthesizer (`#channels <= 1`)**. This means the Hotspot **must operate on the same channel and frequency band (2.4GHz or 5GHz) as your connected Wi-Fi station network**. The application automatically handles this channel matching via the **Auto** setting.

---

## 📜 License

This project is licensed under the [MIT License](LICENSE).
