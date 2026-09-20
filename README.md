# 🚀 Concurrent Linux Wi-Fi Hotspot (Rust + egui)

<p align="center">
  <a href="https://github.com/vaibhav2195/wifi-hotspot/releases/latest">
    <img src="https://img.shields.io/github/v/release/vaibhav2195/wifi-hotspot?color=brightgreen&label=Latest%20Release" alt="Latest Release" />
  </a>
  <a href="https://github.com/vaibhav2195/wifi-hotspot/stargazers">
    <img src="https://img.shields.io/github/stars/vaibhav2195/wifi-hotspot?style=social" alt="GitHub Stars" />
  </a>
  <img src="https://img.shields.io/badge/Platform-Linux-blue.svg" alt="Platform Linux" />
  <img src="https://img.shields.io/badge/Distros-Arch%20%7C%20Ubuntu%20%7C%20Fedora%20%7C%20Debian-purple.svg" alt="Supported Distros" />
  <img src="https://img.shields.io/badge/Rust-2024%20Edition-orange.svg" alt="Rust 2024" />
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="MIT License" />
  </a>
</p>

<p align="center">
  <strong>A lightweight, blazing-fast native Linux desktop GUI application built in Rust to create and manage a Wi-Fi Access Point (Hotspot) while remaining connected to an existing Wi-Fi network (AP/STA Concurrent Mode).</strong>
</p>

---

## 📖 About The Project

On Linux, default desktop tools (like GNOME Settings and basic NetworkManager profiles) **disconnect your laptop from Wi-Fi** as soon as you turn on a mobile hotspot. 

**Concurrent Linux Wi-Fi Hotspot** solves this problem. It leverages Linux kernel wireless virtualization (`nl80211`), `hostapd`, `dnsmasq`, and `iptables` to create an isolated virtual AP interface (`ap0`) alongside your active station interface. You can now **share your Wi-Fi internet connection (e.g. hotel, hostel, university, campus, or office Wi-Fi) with your phone, tablet, and other devices seamlessly**.

### 🌟 Key Highlights
- 🎯 **True AP/STA Concurrency**: Share Wi-Fi without losing your connection.
- 📱 **Real-Time Connected Devices**: See hostnames, IPs, MAC addresses, live signal strengths (📶 dBm), and upload/download transfer rates.
- 🚫 **Multi-Layer Device Blocking**: Instantly kick and ban unwanted devices with 1-click Layer-2 deauthentication (`hostapd_cli`), MAC blacklist (`hostapd.deny`), and Layer-3 firewall packet filtering (`iptables DROP`).
- 🛡️ **Fail-Safe Channel Roaming**: Background watchdog auto-synchronizes the hotspot if you roam across access points or switch to a different Wi-Fi network on another channel.
- ⚡ **Zero DNS Port 53 Conflicts**: Engineered for Arch Linux, Ubuntu, and Fedora without conflicting with `systemd-resolved`.
- 🎛️ **Modern, Simple Interface**: Minimalist primary view with one-click start/stop, plus a collapsible advanced drawer for power users.

---

## 📑 Table of Contents
- [✨ Features](#-features)
- [🛠 System Prerequisites](#-system-prerequisites)
- [🚀 Installation Options](#-installation-options)
  - [Arch Linux (`makepkg`)](#option-a-arch-linux-makepkg)
  - [Debian / Ubuntu (`.deb` Package)](#option-b-debian--ubuntu-deb-package)
  - [Universal Installation (Any Distro)](#option-c-universal-installation-any-linux-distro)
  - [Run from Source](#option-d-run-directly-from-source)
- [📡 Wi-Fi Hardware & Driver Compatibility](#-wi-fi-hardware--driver-compatibility)
- [⚡ How It Works Under the Hood](#-how-it-works-under-the-hood)
- [📜 License](#-license)

---

## ✨ Features

- **Clean & Minimalist Interface**: Simple primary view with Network Name (SSID), Password (with 👁 show/hide toggle), and a prominent Start/Stop button.
- **Live Connected Devices List**: Displays connected clients in real time with Hostname, IP address, MAC address, signal strength (📶 dBm), and real-time data transfer statistics (↑/↓ bytes).
- **Multi-Layer Device Blocking & Banning**: Instant one-click device ban combining Layer 2 deauthentication (`hostapd_cli`), persistent MAC Access Control List (`hostapd.deny`), and Layer 3 packet filtering (`iptables DROP`), plus custom MAC banning and unbanning.
- **Upstream Network Roaming Fail-Safe**: Background watchdog monitors station connection. If you switch to a different Wi-Fi network or AP on a different channel while the hotspot is running, the hotspot auto-synchronizes to the new channel without conflicting or disconnecting you.
- **Configurable Hotspot Capacity**: Set maximum concurrent connections (1–32 clients) enforced natively at the radio level (`max_num_sta`).
- **Collapsible Advanced Settings**: Full diagnostics, manual channel pickers, frequency band overrides (2.4 GHz vs 5 GHz), interface selection, and live Wi-Fi network scans tucked neatly into an expandable drawer.
- **AP/STA Concurrency**: Share your Wi-Fi internet connection without dropping your station Wi-Fi.
- **Cross-Distro Compatibility**: Tested and engineered for **Arch Linux**, **Ubuntu/Debian**, and **Fedora**, including zero-collision `dnsmasq` binding that avoids port 53 conflicts with `systemd-resolved`.
- **Automatic Band & Channel Matching**: Automatically tunes the Hotspot channel and frequency band (2.4 GHz vs 5 GHz) to match your active Wi-Fi connection, preventing hardware synthesizer conflicts.
- **DFS Radar Protection Handling**: Intelligently identifies DFS channels (52–144) where AP beaconing is prohibited by regulation and driver firmware, seamlessly selecting compatible alternative channels.
- **Desktop Application & Polkit Integration**: Built-in `.desktop` entry and PolicyKit rule allowing users to launch the app directly from their Linux application menu with a graphical password prompt.

---

## 🛠 System Prerequisites

Install the standard networking daemons and utilities for your distribution:

### Arch Linux / Manjaro / EndeavourOS
```bash
sudo pacman -S hostapd dnsmasq iw iproute2 iptables polkit
```

### Ubuntu / Debian / Linux Mint
```bash
sudo apt update
sudo apt install hostapd dnsmasq iw iproute2 iptables pkexec
```

### Fedora / RHEL
```bash
sudo dnf install hostapd dnsmasq iw iproute iptables polkit
```

---

## 🚀 Installation Options

### Option A: Arch Linux (`makepkg`)
```bash
git clone https://github.com/vaibhav2195/wifi-hotspot.git
cd wifi-hotspot
makepkg -si
```

### Option B: Debian / Ubuntu (`.deb` Package)
Download the latest `.deb` release from [GitHub Releases](https://github.com/vaibhav2195/wifi-hotspot/releases/latest) or build locally:
```bash
./build_deb.sh
sudo dpkg -i wifi-hotspot_2.0.0_amd64.deb
sudo apt-get install -f
```

### Option C: Universal Installation (Any Linux Distro)
Using the included `Makefile` or `install.sh`:
```bash
# Using Makefile
make
sudo make install

# Or using the installer script
sudo ./install.sh
```

### Option D: Run Directly from Source
```bash
cargo build --release
sudo ./target/release/wifi
```

---

## 📡 Wi-Fi Hardware & Driver Compatibility

Linux AP/STA concurrency requires wireless drivers that implement `nl80211` virtual interface support.

| Chipset Family | Driver | AP/STA Concurrency | Synthesizer Constraints | DFS Radar (52–144) AP Mode |
| :--- | :--- | :--- | :--- | :--- |
| **Intel Wi-Fi 6 / 6E / 7** (AX211, AX210, AX201, AX200, BE200) | `iwlwifi` | **Full Support** | Single synthesizer (`#channels <= 1`); Hotspot must share Wi-Fi channel | **Restricted (`NO-IR`)**; cannot beacon on DFS channels |
| **Intel Wi-Fi 5 (802.11ac)** (AC 9560, AC 9260, AC 8265, AC 7265) | `iwlwifi` | **Full Support** | Single synthesizer (`#channels <= 1`); Hotspot must share Wi-Fi channel | **Restricted (`NO-IR`)**; cannot beacon on DFS channels |
| **MediaTek** (MT7921, MT7922, MT76x2, MT7603) | `mt7921e`, `mt76` | **Full Support** (Kernel 5.15+) | Channel matching recommended | Restricted or requires CAC |
| **Qualcomm Atheros** (QCA6174, AR9xxx) | `ath9k`, `ath10k`, `ath11k` | **Excellent** | Some support multi-channel concurrency | Supported with regulatory DFS CAC |
| **Realtek Wi-Fi 6** (RTL8852AE, RTL8852BE, RTL8852CE) | `rtw89` | **Supported** | Single synthesizer | Restricted |
| **Realtek Wi-Fi 5** (RTL8821CE, RTL8822CE) | `rtw88` | **Partial** (Kernel dependent) | May fail to create virtual `__ap` interface on older kernels | Restricted |
| **Broadcom** (BCM43xx) | `brcmfmac` | **Basic Support** | Chipset dependent | Restricted |
| **Broadcom Proprietary** | `wl` | **Not Supported** | Proprietary driver lacks `nl80211` virtual interface support | Not Supported |

> [!TIP]
> **Check Concurrency On Your System:**
> Run `iw list` in your terminal and look for `valid interface combinations`. Look for a rule allowing both `managed` and `AP` simultaneously:
> `#{ managed } <= 1, #{ AP } <= 1, total <= 2, #channels <= 1`

---

## ⚡ How It Works Under the Hood

1. **Virtual Interface Creation**: Spawns an isolated virtual interface `ap0` linked to the physical device (`iw dev <iface> interface add ap0 type __ap`).
2. **NetworkManager Isolation**: Flags `ap0` as unmanaged in NetworkManager (`nmcli device set ap0 managed no`) so it doesn't interrupt or conflict with existing network profiles.
3. **Dedicated Subnet & Conflict-Free DNS**: Configures `ap0` on `192.168.50.1/24`. `dnsmasq` binds specifically to `192.168.50.1:53` with `except-interface=lo`, preventing any port 53 collision with `systemd-resolved`.
4. **Hostapd Authentication**: Generates dynamic configuration files in `/tmp` matching the physical radio's active frequency channel and launches `hostapd` for WPA2-PSK security.
5. **IP Forwarding & NAT**: Enables Linux kernel packet forwarding (`sysctl net.ipv4.ip_forward=1`) and provisions `iptables` MASQUERADE rules so connected clients route internet through the upstream interface.

---

## 🤝 Contributing
Contributions, issues, and feature requests are welcome! Feel free to check the [issues page](https://github.com/vaibhav2195/wifi-hotspot/issues).

---

## 📜 License

This project is licensed under the [MIT License](LICENSE).
