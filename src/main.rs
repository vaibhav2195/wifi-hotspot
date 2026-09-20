#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, PartialEq, Clone, Copy)]
enum BandMode {
    Auto,
    Force2_4GHz,
    Force5GHz,
}

fn is_dfs_channel(channel: u32) -> bool {
    (52..=144).contains(&channel)
}

fn is_5ghz_channel(channel: u32) -> bool {
    channel >= 32
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct AvailableChannel {
    channel: u32,
    freq_mhz: u32,
    is_5ghz: bool,
    is_dfs: bool,
    bssid: String,
    ssid: String,
}

#[derive(Clone, Debug)]
struct ConnectedDevice {
    mac: String,
    ip: String,
    hostname: String,
    signal_dbm: i32,
    rx_bytes: u64,
    tx_bytes: u64,
    connected_time_sec: u64,
}

#[derive(Clone, Debug, PartialEq)]
struct BlockedMacEntry {
    mac: String,
    hostname: String,
    blocked_at: String,
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([480.0, 580.0])
            .with_min_inner_size([380.0, 320.0])
            .with_title("Linux Wi-Fi Hotspot"),
        ..Default::default()
    };
    eframe::run_native(
        "Hotspot App",
        options,
        Box::new(|_cc| Ok(Box::new(HotspotApp::default()))),
    )
}

struct HotspotApp {
    interface: String,
    available_interfaces: Vec<String>,
    ssid: String,
    password: String,
    show_password: bool,
    ap_interface: String,
    band_mode: BandMode,
    custom_channel: String,
    auto_resolve_dfs: bool,
    max_connections: u32,
    manual_block_mac: String,
    modified_conn_uuid: Arc<Mutex<Option<String>>>,
    status_msg: Arc<Mutex<String>>,
    is_running: Arc<Mutex<bool>>,
    active_ssid: String,
    active_channel: u32,
    active_mode: &'static str,
    available_network_channels: Vec<AvailableChannel>,
    driver_name: String,
    connected_devices: Arc<Mutex<Vec<ConnectedDevice>>>,
    blocked_macs: Arc<Mutex<Vec<BlockedMacEntry>>>,
    stop_watchdog: Arc<Mutex<bool>>,
}

impl Default for HotspotApp {
    fn default() -> Self {
        let detected = Self::detect_wireless_interfaces();
        let default_iface = detected.first().cloned().unwrap_or_else(|| "wlan0".to_string());
        let driver_name = Self::detect_driver_name(&default_iface);
        let (ssid, ch, mode, _) = Self::detect_active_wifi_details(&default_iface);
        let network_channels = if !ssid.is_empty() {
            Self::scan_channels_for_ssid(&ssid)
        } else {
            Vec::new()
        };

        Self {
            interface: default_iface,
            available_interfaces: detected,
            ssid: "RustHotspot".to_owned(),
            password: "password123".to_owned(),
            show_password: false,
            ap_interface: "ap0".to_owned(),
            band_mode: BandMode::Auto,
            custom_channel: "Auto".to_owned(),
            auto_resolve_dfs: true,
            max_connections: 10,
            manual_block_mac: String::new(),
            modified_conn_uuid: Arc::new(Mutex::new(None)),
            status_msg: Arc::new(Mutex::new("Ready".to_owned())),
            is_running: Arc::new(Mutex::new(false)),
            active_ssid: ssid,
            active_channel: ch,
            active_mode: mode,
            available_network_channels: network_channels,
            driver_name,
            connected_devices: Arc::new(Mutex::new(Vec::new())),
            blocked_macs: Arc::new(Mutex::new(Vec::new())),
            stop_watchdog: Arc::new(Mutex::new(false)),
        }
    }
}

impl eframe::App for HotspotApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let is_running = *self.is_running.lock().unwrap();
        let status = self.status_msg.lock().unwrap().clone();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // 1. Header with Title and Status Pill
                ui.horizontal(|ui| {
                    ui.heading("Wi-Fi Hotspot");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if is_running {
                            ui.label(
                                egui::RichText::new("● ACTIVE")
                                    .color(egui::Color32::from_rgb(46, 204, 113))
                                    .strong(),
                            );
                        } else if status.starts_with("Error") {
                            ui.label(
                                egui::RichText::new("● ERROR")
                                    .color(egui::Color32::from_rgb(231, 76, 60))
                                    .strong(),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new("● READY")
                                    .color(egui::Color32::from_rgb(149, 165, 166)),
                            );
                        }
                    });
                });
                ui.separator();
                ui.add_space(6.0);

                // 2. Simple Primary View: SSID and Password
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.add_space(2.0);

                    ui.label(egui::RichText::new("Hotspot Name (SSID)").strong());
                    ui.add(
                        egui::TextEdit::singleline(&mut self.ssid)
                            .hint_text("Enter hotspot name")
                            .margin(egui::vec2(8.0, 6.0))
                            .desired_width(f32::INFINITY),
                    );

                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Password").strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let eye_text = if self.show_password { "👁 Hide" } else { "👁 Show" };
                            if ui.small_button(eye_text).clicked() {
                                self.show_password = !self.show_password;
                            }
                        });
                    });

                    ui.add(
                        egui::TextEdit::singleline(&mut self.password)
                            .password(!self.show_password)
                            .hint_text("At least 8 characters")
                            .margin(egui::vec2(8.0, 6.0))
                            .desired_width(f32::INFINITY),
                    );

                    ui.add_space(2.0);
                });

                ui.add_space(8.0);

                // 3. Prominent Start/Stop Hotspot Button
                let btn_height = 42.0;
                if !is_running {
                    let start_btn = egui::Button::new(
                        egui::RichText::new("▶  Start Hotspot")
                            .size(16.0)
                            .strong()
                            .color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(39, 174, 96))
                    .corner_radius(6.0);

                    if ui.add_sized([ui.available_width(), btn_height], start_btn).clicked() {
                        self.start_hotspot(ctx.clone());
                    }
                } else {
                    let stop_btn = egui::Button::new(
                        egui::RichText::new("⏹  Stop Hotspot")
                            .size(16.0)
                            .strong()
                            .color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(192, 57, 43))
                    .corner_radius(6.0);

                    if ui.add_sized([ui.available_width(), btn_height], stop_btn).clicked() {
                        self.stop_hotspot(ctx.clone());
                    }
                }

                ui.add_space(6.0);

                // 4. Status Notification Box
                let status_color = if status.starts_with("Error") {
                    egui::Color32::from_rgb(231, 76, 60)
                } else if status.starts_with("Hotspot running") || status.contains("synchronized") {
                    egui::Color32::from_rgb(46, 204, 113)
                } else {
                    egui::Color32::from_rgb(241, 196, 15)
                };

                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("Status:").strong());
                    ui.label(egui::RichText::new(&status).color(status_color));
                });

                let has_modified_uuid = self.modified_conn_uuid.lock().unwrap().is_some();
                if has_modified_uuid && !is_running {
                    ui.add_space(4.0);
                    if ui.button("🔄 Restore Wi-Fi Connection to Auto")
                        .on_hover_text("Restore Wi-Fi connection band settings back to default Auto")
                        .clicked()
                    {
                        self.restore_wifi_connection(ctx.clone());
                    }
                }

                // 5. LIVE CONNECTED DEVICES (Shown when hotspot is running)
                if is_running {
                    ui.add_space(8.0);
                    let devices = self.connected_devices.lock().unwrap().clone();
                    ui.group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "📱 Connected Devices ({}/{})",
                                    devices.len(),
                                    self.max_connections
                                ))
                                .strong(),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("🔄").on_hover_text("Refresh device list").clicked() {
                                    let devs = Self::parse_connected_devices(&self.ap_interface);
                                    *self.connected_devices.lock().unwrap() = devs;
                                }
                            });
                        });
                        ui.separator();

                        if devices.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new("No devices currently connected")
                                        .italics()
                                        .color(egui::Color32::GRAY),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Broadcasting '{}'. Clients will appear here when connected.",
                                        self.ssid
                                    ))
                                    .small()
                                    .color(egui::Color32::GRAY),
                                );
                                ui.add_space(4.0);
                            });
                        } else {
                            for dev in &devices {
                                ui.group(|ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new(&dev.hostname).strong());
                                                ui.label(
                                                    egui::RichText::new(format!("({})", dev.ip))
                                                        .color(egui::Color32::from_rgb(100, 180, 255)),
                                                );
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    egui::RichText::new(&dev.mac)
                                                        .small()
                                                        .color(egui::Color32::GRAY),
                                                );
                                                let signal_color = if dev.signal_dbm > -60 {
                                                    egui::Color32::GREEN
                                                } else if dev.signal_dbm > -75 {
                                                    egui::Color32::YELLOW
                                                } else {
                                                    egui::Color32::RED
                                                };
                                                ui.label(
                                                    egui::RichText::new(format!("📶 {} dBm", dev.signal_dbm))
                                                        .small()
                                                        .color(signal_color),
                                                );
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "↑ {}  ↓ {}",
                                                        Self::format_bytes(dev.tx_bytes),
                                                        Self::format_bytes(dev.rx_bytes)
                                                    ))
                                                    .small()
                                                    .color(egui::Color32::GRAY),
                                                );
                                            });
                                        });

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            let block_btn = egui::Button::new(
                                                egui::RichText::new("🚫 Block")
                                                    .size(12.0)
                                                    .color(egui::Color32::WHITE),
                                            )
                                            .fill(egui::Color32::from_rgb(192, 57, 43))
                                            .corner_radius(4.0);

                                            if ui
                                                .add(block_btn)
                                                .on_hover_text("Immediately kick and permanently ban this device")
                                                .clicked()
                                            {
                                                Self::block_device(
                                                    &dev.mac,
                                                    &dev.hostname,
                                                    &self.ap_interface,
                                                    &self.blocked_macs,
                                                );
                                                let devs = Self::parse_connected_devices(&self.ap_interface);
                                                *self.connected_devices.lock().unwrap() = devs;
                                            }
                                        });
                                    });
                                });
                            }
                        }
                    });
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                // 6. Collapsible Advanced Settings & Diagnostics
                egui::CollapsingHeader::new(egui::RichText::new("⚙ Advanced Settings & Security").strong())
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.add_space(4.0);

                        // Hotspot Capacity (Max Connections)
                        ui.horizontal(|ui| {
                            ui.label("Max Connections Limit:");
                            ui.add(egui::Slider::new(&mut self.max_connections, 1..=32).text("clients"));
                        });

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);

                        // Blocked Devices Section
                        let blocked = self.blocked_macs.lock().unwrap().clone();
                        ui.label(egui::RichText::new(format!("🚫 Blocked Devices ({})", blocked.len())).strong());
                        ui.group(|ui| {
                            ui.set_width(ui.available_width());
                            if blocked.is_empty() {
                                ui.label(
                                    egui::RichText::new("No devices are currently blocked.")
                                        .italics()
                                        .color(egui::Color32::GRAY),
                                );
                            } else {
                                for b in &blocked {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(&b.mac).strong());
                                        if !b.hostname.is_empty() {
                                            ui.label(
                                                egui::RichText::new(format!("({})", b.hostname))
                                                    .color(egui::Color32::GRAY),
                                            );
                                        }
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.button("✅ Unblock").clicked() {
                                                Self::unblock_device(&b.mac, &self.ap_interface, &self.blocked_macs);
                                            }
                                        });
                                    });
                                }
                            }

                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label("Block by MAC:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.manual_block_mac)
                                        .hint_text("aa:bb:cc:dd:ee:ff")
                                        .desired_width(140.0),
                                );
                                if ui.button("🚫 Ban MAC").clicked() {
                                    if !self.manual_block_mac.trim().is_empty() {
                                        Self::block_device(
                                            &self.manual_block_mac,
                                            "Manual Entry",
                                            &self.ap_interface,
                                            &self.blocked_macs,
                                        );
                                        self.manual_block_mac.clear();
                                    }
                                }
                            });
                        });

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);

                        // Network Interface Selector
                        ui.horizontal(|ui| {
                            ui.label("Wi-Fi Interface:");
                            let prev_iface = self.interface.clone();
                            egui::ComboBox::from_id_salt("iface_combo")
                                .selected_text(&self.interface)
                                .show_ui(ui, |ui| {
                                    for iface in &self.available_interfaces {
                                        ui.selectable_value(&mut self.interface, iface.clone(), iface);
                                    }
                                });
                            if self.interface != prev_iface {
                                self.refresh_wifi_info();
                            }
                            if ui.button("🔄").on_hover_text("Refresh interfaces & connection details").clicked() {
                                self.available_interfaces = Self::detect_wireless_interfaces();
                                if let Some(first) = self.available_interfaces.first() {
                                    self.interface = first.clone();
                                }
                                self.refresh_wifi_info();
                            }
                        });

                        // Virtual AP interface name
                        ui.horizontal(|ui| {
                            ui.label("AP Virtual Interface:");
                            ui.text_edit_singleline(&mut self.ap_interface);
                        });

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);

                        // Frequency Band Selector
                        ui.label(egui::RichText::new("Frequency Band:").strong());
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.band_mode, BandMode::Auto, "Auto (Match Wi-Fi)");
                            ui.selectable_value(&mut self.band_mode, BandMode::Force2_4GHz, "2.4 GHz");
                            ui.selectable_value(&mut self.band_mode, BandMode::Force5GHz, "5 GHz");
                        });

                        ui.add_space(4.0);

                        // Channel Selection
                        ui.horizontal(|ui| {
                            ui.label("Channel:");
                            egui::ComboBox::from_id_salt("channel_combo")
                                .selected_text(&self.custom_channel)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.custom_channel, "Auto".to_string(), "Auto");
                                    match self.band_mode {
                                        BandMode::Force2_4GHz => {
                                            for ch in [1, 6, 11] {
                                                ui.selectable_value(
                                                    &mut self.custom_channel,
                                                    ch.to_string(),
                                                    format!("Channel {} (2.4 GHz)", ch),
                                                );
                                            }
                                        }
                                        BandMode::Force5GHz => {
                                            for ch in [36, 40, 44, 48, 149, 153, 157, 161, 165] {
                                                let band_name = if ch <= 48 { "U-NII-1" } else { "U-NII-3" };
                                                ui.selectable_value(
                                                    &mut self.custom_channel,
                                                    ch.to_string(),
                                                    format!("Channel {} (5 GHz {})", ch, band_name),
                                                );
                                            }
                                        }
                                        BandMode::Auto => {
                                            ui.label("Managed automatically to match Wi-Fi");
                                        }
                                    }
                                });
                        });

                        ui.add_space(4.0);
                        ui.checkbox(
                            &mut self.auto_resolve_dfs,
                            "Auto-switch Wi-Fi when active channel is DFS restricted",
                        )
                        .on_hover_text(
                            "If connected Wi-Fi is on a radar/DFS channel (e.g. 52, 104, 120), automatically switch to a non-DFS channel for the hotspot.",
                        );

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);

                        // Live Active Connection Diagnostics
                        let is_dfs = is_dfs_channel(self.active_channel);
                        ui.label(egui::RichText::new("Live Wi-Fi Connection Details:").strong());
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("Connected Network:");
                                if self.active_ssid.is_empty() {
                                    ui.label(egui::RichText::new("Not connected").italics().color(egui::Color32::GRAY));
                                } else {
                                    let band_label = if self.active_mode == "a" { "5 GHz" } else { "2.4 GHz" };
                                    ui.label(egui::RichText::new(format!(
                                        "{} (Ch {}, {})",
                                        self.active_ssid, self.active_channel, band_label
                                    )).strong());

                                    if is_dfs {
                                        ui.label(
                                            egui::RichText::new("⚠️ DFS Restricted")
                                                .color(egui::Color32::from_rgb(255, 175, 50))
                                                .strong(),
                                        );
                                    } else {
                                        ui.label(egui::RichText::new("✓ AP Compatible").color(egui::Color32::GREEN));
                                    }
                                }
                            });

                            if is_dfs {
                                ui.add_space(2.0);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Channel {} is in the DFS radar range. Wireless regulations forbid AP mode beaconing on DFS channels.",
                                        self.active_channel
                                    ))
                                    .small()
                                    .color(egui::Color32::from_rgb(255, 190, 80)),
                                );

                                if !self.available_network_channels.is_empty() {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(egui::RichText::new("Channels found for network:").small());
                                        for ch_opt in &self.available_network_channels {
                                            if ch_opt.is_dfs {
                                                ui.label(
                                                    egui::RichText::new(format!("Ch {} (DFS)", ch_opt.channel))
                                                        .small()
                                                        .color(egui::Color32::GRAY),
                                                );
                                            } else if ch_opt.is_5ghz {
                                                ui.label(
                                                    egui::RichText::new(format!("Ch {} (5G ✓)", ch_opt.channel))
                                                        .small()
                                                        .color(egui::Color32::LIGHT_GREEN)
                                                        .strong(),
                                                );
                                            } else {
                                                ui.label(
                                                    egui::RichText::new(format!("Ch {} (2.4G ✓)", ch_opt.channel))
                                                        .small()
                                                        .color(egui::Color32::LIGHT_BLUE)
                                                        .strong(),
                                                );
                                            }
                                        }
                                    });
                                }
                            }
                        });

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);

                        // Hardware & Concurrency Information
                        ui.label(egui::RichText::new("Hardware & Driver Diagnostics:").strong());
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("Kernel Driver:");
                                ui.label(egui::RichText::new(&self.driver_name).strong());
                            });

                            let hw_desc = match self.driver_name.as_str() {
                                "iwlwifi" => "Intel Wireless (Single Synthesizer: Hotspot must match Wi-Fi channel; DFS AP mode blocked by firmware)",
                                d if d.starts_with("rtw") || d.starts_with("rtl") => "Realtek Wireless (AP concurrency varies by chipset/driver model)",
                                d if d.starts_with("mt7") => "MediaTek Wireless (Concurrent AP/STA mode supported in modern kernels)",
                                d if d.starts_with("ath") => "Qualcomm Atheros (Virtual interface and AP mode supported)",
                                d if d.starts_with("brcm") || d == "wl" => "Broadcom Wireless (Proprietary wl driver does not support virtual AP)",
                                _ => "Standard Linux nl80211 wireless device",
                            };

                            ui.label(
                                egui::RichText::new(hw_desc)
                                    .small()
                                    .color(egui::Color32::GRAY),
                            );
                        });

                        ui.add_space(4.0);
                    });

                ui.add_space(8.0);
            });
    }
}

impl HotspotApp {
    fn set_status(&self, ctx: egui::Context, msg: &str) {
        let mut status = self.status_msg.lock().unwrap();
        *status = msg.to_string();
        ctx.request_repaint();
    }

    fn format_bytes(bytes: u64) -> String {
        if bytes >= 1024 * 1024 * 1024 {
            format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        } else if bytes >= 1024 * 1024 {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else if bytes >= 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{} B", bytes)
        }
    }

    fn simple_timestamp() -> String {
        if let Ok(duration) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            let secs = duration.as_secs();
            let hours = (secs / 3600) % 24;
            let mins = (secs / 60) % 60;
            let s = secs % 60;
            format!("{:02}:{:02}:{:02} UTC", hours, mins, s)
        } else {
            "Banned".to_string()
        }
    }

    fn detect_driver_name(interface: &str) -> String {
        let path = format!("/sys/class/net/{}/device/driver", interface);
        if let Ok(link) = std::fs::read_link(&path) {
            if let Some(file_name) = link.file_name() {
                return file_name.to_string_lossy().to_string();
            }
        }
        "Unknown".to_string()
    }

    fn is_root() -> bool {
        Command::new("id")
            .arg("-u")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
            .unwrap_or(false)
    }

    fn refresh_wifi_info(&mut self) {
        self.driver_name = Self::detect_driver_name(&self.interface);
        let (ssid, ch, mode, _) = Self::detect_active_wifi_details(&self.interface);
        self.active_ssid = ssid.clone();
        self.active_channel = ch;
        self.active_mode = mode;
        self.available_network_channels = if !ssid.is_empty() {
            Self::scan_channels_for_ssid(&ssid)
        } else {
            Vec::new()
        };
    }

    fn run_cmd(cmd: &str, args: &[&str]) -> bool {
        let output = Command::new(cmd).args(args).output();
        if let Ok(out) = output {
            out.status.success()
        } else {
            false
        }
    }

    fn detect_wireless_interfaces() -> Vec<String> {
        let mut ifaces = Vec::new();
        let output = Command::new("iw").arg("dev").output();
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let line = line.trim();
                if line.starts_with("Interface ") {
                    let name = line["Interface ".len()..].trim();
                    if !name.starts_with("ap") && !name.starts_with("p2p") {
                        ifaces.push(name.to_string());
                    }
                }
            }
        }
        if ifaces.is_empty() {
            ifaces.push("wlan0".to_string());
        }
        ifaces
    }

    fn get_current_channel_and_mode(interface: &str) -> (u32, &'static str) {
        let output = Command::new("iw").args(["dev", interface, "info"]).output();
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                if line.contains("channel") {
                    let parts: Vec<&str> = line.trim().split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(ch) = parts[1].parse::<u32>() {
                            let mode = if ch > 14 { "a" } else { "g" };
                            return (ch, mode);
                        }
                    }
                }
            }
        }
        (6, "g")
    }

    fn detect_active_wifi_details(interface: &str) -> (String, u32, &'static str, bool) {
        let (ch, mode) = Self::get_current_channel_and_mode(interface);
        let mut ssid = String::new();
        let output = Command::new("iw").args(["dev", interface, "link"]).output();
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let line = line.trim();
                if line.starts_with("SSID: ") {
                    ssid = line["SSID: ".len()..].trim().to_string();
                    break;
                }
            }
        }
        let is_dfs = is_dfs_channel(ch);
        (ssid, ch, mode, is_dfs)
    }

    fn scan_channels_for_ssid(target_ssid: &str) -> Vec<AvailableChannel> {
        let mut results = Vec::new();
        let output = Command::new("nmcli")
            .args(["-t", "-f", "BSSID,SSID,CHAN,FREQ", "dev", "wifi", "list"])
            .output();

        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let last_parts: Vec<&str> = line.rsplitn(4, ':').collect();
                if last_parts.len() == 4 {
                    let bssid = last_parts[3].replace('\\', "");
                    let ssid = last_parts[2].to_string();
                    let chan_str = last_parts[1];
                    let freq_str = last_parts[0];

                    if ssid == target_ssid {
                        if let Ok(ch) = chan_str.trim().parse::<u32>() {
                            let freq = freq_str
                                .trim()
                                .split_whitespace()
                                .next()
                                .and_then(|f| f.parse::<u32>().ok())
                                .unwrap_or(0);

                            if !results.iter().any(|c: &AvailableChannel| c.channel == ch) {
                                results.push(AvailableChannel {
                                    channel: ch,
                                    freq_mhz: freq,
                                    is_5ghz: is_5ghz_channel(ch),
                                    is_dfs: is_dfs_channel(ch),
                                    bssid,
                                    ssid: ssid.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Sort prioritizing 5GHz non-DFS, then 2.4GHz non-DFS, then DFS
        results.sort_by_key(|c| {
            if c.is_dfs {
                2
            } else if c.is_5ghz {
                0
            } else {
                1
            }
        });
        results
    }

    fn get_active_connection_uuid(interface: &str) -> Option<String> {
        let output = Command::new("nmcli")
            .args(["-t", "-f", "DEVICE,UUID", "con", "show", "--active"])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 && parts[0] == interface {
                return Some(parts[1].trim().to_string());
            }
        }
        None
    }

    fn switch_connection_channel(uuid: &str, bssid: &str, band: &str) -> bool {
        let mut args = vec!["connection", "modify", uuid];
        if !bssid.is_empty() {
            args.extend_from_slice(&["802-11-wireless.bssid", bssid]);
        }
        if !band.is_empty() {
            args.extend_from_slice(&["802-11-wireless.band", band]);
        }
        let mod_ok = Command::new("nmcli").args(&args).status().map(|s| s.success()).unwrap_or(false);
        if mod_ok {
            let up_ok = Command::new("nmcli").args(["connection", "up", uuid]).status().map(|s| s.success()).unwrap_or(false);
            return up_ok;
        }
        false
    }

    fn restore_wifi_connection(&mut self, ctx: egui::Context) {
        let maybe_uuid = self.modified_conn_uuid.lock().unwrap().clone();
        if let Some(uuid) = maybe_uuid {
            self.set_status(ctx.clone(), "Restoring Wi-Fi to Auto (allowing 5GHz)...");
            let _ = Command::new("nmcli")
                .args(["connection", "modify", &uuid, "802-11-wireless.bssid", "", "802-11-wireless.band", ""])
                .status();
            let _ = Command::new("nmcli").args(["connection", "up", &uuid]).status();
            *self.modified_conn_uuid.lock().unwrap() = None;
            self.refresh_wifi_info();
            self.set_status(ctx.clone(), "Wi-Fi restored to Auto.");
        }
    }

    fn parse_connected_devices(ap_interface: &str) -> Vec<ConnectedDevice> {
        let mut devices_map: HashMap<String, ConnectedDevice> = HashMap::new();

        // 1. Read DHCP leases from /tmp/rust_dnsmasq.leases
        if let Ok(content) = std::fs::read_to_string("/tmp/rust_dnsmasq.leases") {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let mac = parts[1].to_lowercase();
                    let ip = parts[2].to_string();
                    let hostname = if parts[3] == "*" || parts[3].is_empty() {
                        format!("Device ({})", ip)
                    } else {
                        parts[3].to_string()
                    };
                    devices_map.insert(
                        mac.clone(),
                        ConnectedDevice {
                            mac,
                            ip,
                            hostname,
                            signal_dbm: -99,
                            rx_bytes: 0,
                            tx_bytes: 0,
                            connected_time_sec: 0,
                        },
                    );
                }
            }
        }

        // 2. Parse Layer 2 stations from `iw dev <ap_interface> station dump`
        let output = Command::new("iw")
            .args(["dev", ap_interface, "station", "dump"])
            .output();

        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            let mut cur_mac = String::new();
            let mut cur_signal = -99;
            let mut cur_rx = 0;
            let mut cur_tx = 0;
            let mut cur_time = 0;

            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("Station ") {
                    if !cur_mac.is_empty() {
                        let entry = devices_map.entry(cur_mac.clone()).or_insert_with(|| ConnectedDevice {
                            mac: cur_mac.clone(),
                            ip: "Connecting...".to_string(),
                            hostname: format!("Client ({})", cur_mac),
                            signal_dbm: cur_signal,
                            rx_bytes: cur_rx,
                            tx_bytes: cur_tx,
                            connected_time_sec: cur_time,
                        });
                        entry.signal_dbm = cur_signal;
                        entry.rx_bytes = cur_rx;
                        entry.tx_bytes = cur_tx;
                        entry.connected_time_sec = cur_time;
                    }
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 2 {
                        cur_mac = parts[1].to_lowercase();
                    } else {
                        cur_mac.clear();
                    }
                    cur_signal = -99;
                    cur_rx = 0;
                    cur_tx = 0;
                    cur_time = 0;
                } else if trimmed.starts_with("signal:") {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(s) = parts[1].parse::<i32>() {
                            cur_signal = s;
                        }
                    }
                } else if trimmed.starts_with("rx bytes:") {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 3 {
                        if let Ok(b) = parts[2].parse::<u64>() {
                            cur_rx = b;
                        }
                    }
                } else if trimmed.starts_with("tx bytes:") {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 3 {
                        if let Ok(b) = parts[2].parse::<u64>() {
                            cur_tx = b;
                        }
                    }
                } else if trimmed.starts_with("connected time:") {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 3 {
                        if let Ok(t) = parts[2].parse::<u64>() {
                            cur_time = t;
                        }
                    }
                }
            }

            if !cur_mac.is_empty() {
                let entry = devices_map.entry(cur_mac.clone()).or_insert_with(|| ConnectedDevice {
                    mac: cur_mac.clone(),
                    ip: "Connecting...".to_string(),
                    hostname: format!("Client ({})", cur_mac),
                    signal_dbm: cur_signal,
                    rx_bytes: cur_rx,
                    tx_bytes: cur_tx,
                    connected_time_sec: cur_time,
                });
                entry.signal_dbm = cur_signal;
                entry.rx_bytes = cur_rx;
                entry.tx_bytes = cur_tx;
                entry.connected_time_sec = cur_time;
            }
        }

        let mut list: Vec<ConnectedDevice> = devices_map
            .into_values()
            .filter(|d| d.signal_dbm > -99 || d.connected_time_sec > 0)
            .collect();
        list.sort_by(|a, b| a.hostname.cmp(&b.hostname));
        list
    }

    fn sync_deny_file(blocked_macs: &Arc<Mutex<Vec<BlockedMacEntry>>>) {
        let list = blocked_macs.lock().unwrap();
        let mut lines = String::new();
        for item in list.iter() {
            lines.push_str(&item.mac);
            lines.push('\n');
        }
        let _ = std::fs::write("/tmp/rust_hostapd.deny", lines);
    }

    fn block_device(
        mac: &str,
        hostname: &str,
        ap_interface: &str,
        blocked_macs: &Arc<Mutex<Vec<BlockedMacEntry>>>,
    ) {
        let clean_mac = mac.trim().to_lowercase();
        if clean_mac.is_empty() {
            return;
        }

        {
            let mut list = blocked_macs.lock().unwrap();
            if !list.iter().any(|b| b.mac.to_lowercase() == clean_mac) {
                list.push(BlockedMacEntry {
                    mac: clean_mac.clone(),
                    hostname: hostname.to_string(),
                    blocked_at: Self::simple_timestamp(),
                });
            }
        }

        Self::sync_deny_file(blocked_macs);

        // Immediate layer 2 deauthentication via hostapd_cli
        let _ = Command::new("hostapd_cli")
            .args([
                "-p",
                "/tmp/rust_hostapd_ctrl",
                "-i",
                ap_interface,
                "deauthenticate",
                &clean_mac,
            ])
            .output();

        // Immediate layer 3 packet drop via iptables
        let _ = Command::new("iptables")
            .args([
                "-I", "FORWARD", "-i", ap_interface, "-m", "mac", "--mac-source", &clean_mac, "-j", "DROP",
            ])
            .output();
        let _ = Command::new("iptables")
            .args([
                "-I", "INPUT", "-i", ap_interface, "-m", "mac", "--mac-source", &clean_mac, "-j", "DROP",
            ])
            .output();
    }

    fn unblock_device(
        mac: &str,
        ap_interface: &str,
        blocked_macs: &Arc<Mutex<Vec<BlockedMacEntry>>>,
    ) {
        let clean_mac = mac.trim().to_lowercase();
        {
            let mut list = blocked_macs.lock().unwrap();
            list.retain(|b| b.mac.to_lowercase() != clean_mac);
        }

        Self::sync_deny_file(blocked_macs);

        // Remove iptables rules
        let _ = Command::new("iptables")
            .args([
                "-D", "FORWARD", "-i", ap_interface, "-m", "mac", "--mac-source", &clean_mac, "-j", "DROP",
            ])
            .output();
        let _ = Command::new("iptables")
            .args([
                "-D", "INPUT", "-i", ap_interface, "-m", "mac", "--mac-source", &clean_mac, "-j", "DROP",
            ])
            .output();
    }

    fn resync_hostapd_channel(_ap_interface: &str, new_channel: u32, new_mode: &str) -> bool {
        let conf_path = "/tmp/rust_hostapd.conf";
        if let Ok(content) = std::fs::read_to_string(conf_path) {
            let mut new_lines = Vec::new();
            for line in content.lines() {
                if line.starts_with("channel=") {
                    new_lines.push(format!("channel={}", new_channel));
                } else if line.starts_with("hw_mode=") {
                    new_lines.push(format!("hw_mode={}", new_mode));
                } else {
                    new_lines.push(line.to_string());
                }
            }
            if new_mode == "a" && !content.contains("ieee80211n=") {
                new_lines.push("ieee80211n=1".to_string());
            }
            let _ = std::fs::write(conf_path, new_lines.join("\n") + "\n");

            let _ = Command::new("killall").arg("hostapd").output();
            std::thread::sleep(std::time::Duration::from_millis(500));

            let log_file = std::fs::File::create("/tmp/rust_hostapd.log").ok();
            let mut cmd = Command::new("hostapd");
            cmd.arg(conf_path);
            if let Some(f) = log_file {
                if let Ok(f_err) = f.try_clone() {
                    cmd.stdout(std::process::Stdio::from(f));
                    cmd.stderr(std::process::Stdio::from(f_err));
                }
            }
            return cmd.spawn().is_ok();
        }
        false
    }

    fn start_hotspot(&self, ctx: egui::Context) {
        let mut running = self.is_running.lock().unwrap();
        *running = true;
        *self.stop_watchdog.lock().unwrap() = false;

        self.set_status(ctx.clone(), "Starting hotspot (hostapd/dnsmasq)...");

        let interface = self.interface.clone();
        let ssid = self.ssid.clone();
        let password = self.password.clone();
        let ap_interface = self.ap_interface.clone();
        let band_mode = self.band_mode;
        let custom_channel = self.custom_channel.clone();
        let auto_resolve_dfs = self.auto_resolve_dfs;
        let max_connections = self.max_connections;

        let status_arc = self.status_msg.clone();
        let running_arc = self.is_running.clone();
        let modified_uuid_arc = self.modified_conn_uuid.clone();
        let connected_devices_arc = self.connected_devices.clone();
        let blocked_macs_arc = self.blocked_macs.clone();
        let stop_watchdog_arc = self.stop_watchdog.clone();

        thread::spawn(move || {
            // Stop any leftover instances
            let _ = Command::new("killall").arg("dnsmasq").output();
            let _ = Command::new("killall").arg("hostapd").output();
            let _ = Command::new("ip").args(["link", "set", &ap_interface, "down"]).output();
            let _ = Command::new("iw").args(["dev", &ap_interface, "del"]).output();
            let _ = Command::new("iptables").args(["-t", "nat", "-D", "POSTROUTING", "-o", &interface, "-j", "MASQUERADE"]).output();

            // Detect current Wi-Fi details
            let (active_ssid, mut auto_ch, mut auto_mode, _) = Self::detect_active_wifi_details(&interface);

            // Check DFS conflict
            if band_mode == BandMode::Auto && is_dfs_channel(auto_ch) {
                if auto_resolve_dfs {
                    let candidates = Self::scan_channels_for_ssid(&active_ssid);
                    if let Some(target) = candidates.iter().find(|c| !c.is_dfs) {
                        let band_desc = if target.is_5ghz { "5 GHz" } else { "2.4 GHz" };
                        *status_arc.lock().unwrap() = format!(
                            "Channel {} is DFS restricted. Switching Wi-Fi to {} (Ch {}) for '{}'...",
                            auto_ch, band_desc, target.channel, active_ssid
                        );
                        ctx.request_repaint();

                        if let Some(uuid) = Self::get_active_connection_uuid(&interface) {
                            let band_param = if target.is_5ghz { "a" } else { "bg" };
                            if Self::switch_connection_channel(&uuid, &target.bssid, band_param) {
                                *modified_uuid_arc.lock().unwrap() = Some(uuid.clone());
                                // Wait for connection to settle on new channel
                                for _ in 0..16 {
                                    std::thread::sleep(std::time::Duration::from_millis(500));
                                    let (cur_ch, cur_m) = Self::get_current_channel_and_mode(&interface);
                                    if !is_dfs_channel(cur_ch) && (cur_ch == target.channel || cur_ch > 0) {
                                        auto_ch = cur_ch;
                                        auto_mode = cur_m;
                                        break;
                                    }
                                }
                            } else {
                                *status_arc.lock().unwrap() = format!(
                                    "Error: Channel {} is DFS restricted. Failed to switch to alternative channel {}.",
                                    auto_ch, target.channel
                                );
                                *running_arc.lock().unwrap() = false;
                                ctx.request_repaint();
                                return;
                            }
                        }
                    } else {
                        *status_arc.lock().unwrap() = format!(
                            "Error: Channel {} is a DFS radar channel. No working non-DFS channels were found for '{}'.",
                            auto_ch, active_ssid
                        );
                        *running_arc.lock().unwrap() = false;
                        ctx.request_repaint();
                        return;
                    }
                } else {
                    *status_arc.lock().unwrap() = format!(
                        "Error: Channel {} is a DFS radar channel where AP mode is restricted. Enable 'Auto-resolve DFS' or select a non-DFS channel.",
                        auto_ch
                    );
                    *running_arc.lock().unwrap() = false;
                    ctx.request_repaint();
                    return;
                }
            }

            // Create AP interface using iw
            if !Self::run_cmd(
                "iw",
                &[
                    "dev",
                    &interface,
                    "interface",
                    "add",
                    &ap_interface,
                    "type",
                    "__ap",
                ],
            ) {
                let err_detail = if !Self::is_root() {
                    "Administrator (root/pkexec) privileges required to configure wireless interfaces."
                } else {
                    "Failed to create virtual AP interface. Your Wi-Fi adapter or driver may not support concurrent AP+Station mode."
                };
                *status_arc.lock().unwrap() = format!("Error: {}", err_detail);
                *running_arc.lock().unwrap() = false;
                ctx.request_repaint();
                return;
            }

            // Tell NetworkManager to IGNORE this interface
            Self::run_cmd("nmcli", &["device", "set", &ap_interface, "managed", "no"]);

            // Assign unique MAC address
            Self::run_cmd("ip", &["link", "set", "dev", &ap_interface, "address", "12:34:56:78:9a:bc"]);

            // Assign IP to AP interface
            Self::run_cmd("ip", &["addr", "add", "192.168.50.1/24", "broadcast", "192.168.50.255", "dev", &ap_interface]);
            Self::run_cmd("ip", &["link", "set", &ap_interface, "up"]);

            // Determine Channel and Mode
            let (channel, hw_mode) = match band_mode {
                BandMode::Auto => {
                    let ch = if custom_channel != "Auto" {
                        custom_channel.parse::<u32>().unwrap_or(auto_ch)
                    } else {
                        auto_ch
                    };
                    (ch, auto_mode)
                }
                BandMode::Force2_4GHz => {
                    let ch = if custom_channel != "Auto" {
                        custom_channel.parse::<u32>().unwrap_or(6)
                    } else {
                        6
                    };
                    (ch, "g")
                }
                BandMode::Force5GHz => {
                    let ch = if custom_channel != "Auto" {
                        custom_channel.parse::<u32>().unwrap_or(149)
                    } else {
                        149
                    };
                    (ch, "a")
                }
            };

            // Reject DFS channels explicitly
            if is_dfs_channel(channel) {
                *status_arc.lock().unwrap() = format!(
                    "Error: Channel {} is a DFS radar channel. Hotspot (AP mode) is prohibited on DFS channels by regulatory rules. Please choose a non-DFS channel (e.g. 5GHz: 36-48, 149-165, or 2.4GHz: 1, 6, 11).",
                    channel
                );
                *running_arc.lock().unwrap() = false;
                ctx.request_repaint();
                return;
            }

            // Create hostapd control directory
            let _ = std::fs::create_dir_all("/tmp/rust_hostapd_ctrl");

            // Write initial deny file
            Self::sync_deny_file(&blocked_macs_arc);

            // Create hostapd.conf with ACL and max connection limit
            let hostapd_conf_path = "/tmp/rust_hostapd.conf";
            let mut hostapd_config = format!(
                "interface={}\n\
                ssid={}\n\
                hw_mode={}\n\
                channel={}\n\
                macaddr_acl=0\n\
                deny_mac_file=/tmp/rust_hostapd.deny\n\
                ctrl_interface=/tmp/rust_hostapd_ctrl\n\
                ctrl_interface_group=0\n\
                max_num_sta={}\n\
                auth_algs=1\n\
                ignore_broadcast_ssid=0\n\
                wpa=2\n\
                wpa_passphrase={}\n\
                wpa_key_mgmt=WPA-PSK\n\
                wpa_pairwise=TKIP\n\
                rsn_pairwise=CCMP\n",
                ap_interface, ssid, hw_mode, channel, max_connections, password
            );
            if hw_mode == "a" {
                hostapd_config.push_str("ieee80211n=1\n");
            }
            std::fs::write(hostapd_conf_path, hostapd_config).expect("Failed to write hostapd.conf");

            // Create dnsmasq.conf configured for universal Linux distribution compatibility
            let dnsmasq_conf_path = "/tmp/rust_dnsmasq.conf";
            let dnsmasq_config = format!(
                "interface={}\n\
                bind-interfaces\n\
                listen-address=192.168.50.1\n\
                except-interface=lo\n\
                port=53\n\
                dhcp-authoritative\n\
                dhcp-leasefile=/tmp/rust_dnsmasq.leases\n\
                domain-needed\n\
                bogus-priv\n\
                dhcp-range=192.168.50.10,192.168.50.250,12h\n\
                dhcp-option=option:dns-server,8.8.8.8,1.1.1.1\n\
                server=8.8.8.8\n\
                server=1.1.1.1\n",
                ap_interface
            );
            std::fs::write(dnsmasq_conf_path, dnsmasq_config).expect("Failed to write dnsmasq.conf");

            // Enable IP forwarding and NAT
            Self::run_cmd("sysctl", &["-w", "net.ipv4.ip_forward=1"]);
            Self::run_cmd("iptables", &["-t", "nat", "-A", "POSTROUTING", "-o", &interface, "-j", "MASQUERADE"]);

            // Re-apply firewall DROP rules for currently blocked MACs
            {
                let blocked = blocked_macs_arc.lock().unwrap();
                for b in blocked.iter() {
                    let _ = Command::new("iptables")
                        .args(["-I", "FORWARD", "-i", &ap_interface, "-m", "mac", "--mac-source", &b.mac, "-j", "DROP"])
                        .output();
                    let _ = Command::new("iptables")
                        .args(["-I", "INPUT", "-i", &ap_interface, "-m", "mac", "--mac-source", &b.mac, "-j", "DROP"])
                        .output();
                }
            }

            // Start hostapd capturing log output
            let hostapd_log_path = "/tmp/rust_hostapd.log";
            let log_file = std::fs::File::create(hostapd_log_path).ok();
            let mut cmd = Command::new("hostapd");
            cmd.arg(hostapd_conf_path);
            if let Some(f) = log_file {
                if let Ok(f_err) = f.try_clone() {
                    cmd.stdout(std::process::Stdio::from(f));
                    cmd.stderr(std::process::Stdio::from(f_err));
                }
            }

            let mut hostapd_child = cmd.spawn().expect("Failed to start hostapd");
            Command::new("dnsmasq").args(["-C", dnsmasq_conf_path, "-x", "/tmp/rust_dnsmasq.pid"]).spawn().expect("Failed to start dnsmasq");

            // Wait brief moment to check if hostapd succeeded
            std::thread::sleep(std::time::Duration::from_millis(1000));

            if let Ok(Some(status)) = hostapd_child.try_wait() {
                if !status.success() {
                    let log_content = std::fs::read_to_string(hostapd_log_path).unwrap_or_default();
                    let err_reason = if log_content.contains("not allowed for AP mode") || log_content.contains("NO-IR") || log_content.contains("RADAR") {
                        format!("Channel {} is restricted by radar/DFS regulations. AP mode cannot transmit on DFS channels.", channel)
                    } else if log_content.contains("Failed to set beacon parameters") {
                        format!("Channel conflict! Hotspot channel ({}) does not match active Wi-Fi channel ({}). Hardware only supports 1 channel synthesizer.", channel, auto_ch)
                    } else {
                        let last_err = log_content.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("Unknown error");
                        format!("hostapd error: {}", last_err)
                    };

                    *status_arc.lock().unwrap() = format!("Error: {}", err_reason);
                    *running_arc.lock().unwrap() = false;
                    ctx.request_repaint();
                    return;
                }
            }

            *status_arc.lock().unwrap() = format!(
                "Hotspot running successfully! (Mode: {}GHz, Channel: {})",
                if hw_mode == "a" { "5" } else { "2.4" },
                channel
            );
            ctx.request_repaint();

            // -------------------------------------------------------------
            // FAIL-SAFE WATCHDOG THREAD:
            // 1. Live polls connected devices from iw station dump & dnsmasq
            // 2. Detects upstream Wi-Fi / AP / channel switches and auto-resyncs
            // -------------------------------------------------------------
            let watchdog_stop = stop_watchdog_arc.clone();
            let watchdog_iface = interface.clone();
            let watchdog_ap = ap_interface.clone();
            let watchdog_status = status_arc.clone();
            let watchdog_devices = connected_devices_arc.clone();
            let watchdog_ctx = ctx.clone();
            let mut last_station_ch = channel;
            let mut last_station_ssid = active_ssid.clone();

            thread::spawn(move || {
                while !*watchdog_stop.lock().unwrap() {
                    std::thread::sleep(std::time::Duration::from_millis(2000));
                    if *watchdog_stop.lock().unwrap() {
                        break;
                    }

                    // 1. Poll connected devices
                    let devs = Self::parse_connected_devices(&watchdog_ap);
                    {
                        let mut lock = watchdog_devices.lock().unwrap();
                        *lock = devs;
                    }
                    watchdog_ctx.request_repaint();

                    // 2. Upstream Fail-Safe: Check active Wi-Fi channel & SSID
                    let (cur_ssid, cur_ch, cur_mode, is_dfs) = Self::detect_active_wifi_details(&watchdog_iface);
                    if cur_ch > 0 && cur_ch != last_station_ch {
                        // User switched upstream Wi-Fi network or AP!
                        if is_dfs {
                            *watchdog_status.lock().unwrap() = format!(
                                "⚠️ Upstream Wi-Fi switched to '{}' on DFS Channel {}. Hotspot paused to avoid hardware synthesizer conflict.",
                                cur_ssid, cur_ch
                            );
                            watchdog_ctx.request_repaint();
                        } else {
                            *watchdog_status.lock().unwrap() = format!(
                                "Upstream Wi-Fi switched to '{}' (Ch {}). Re-synchronizing hotspot...",
                                cur_ssid, cur_ch
                            );
                            watchdog_ctx.request_repaint();

                            if Self::resync_hostapd_channel(&watchdog_ap, cur_ch, cur_mode) {
                                last_station_ch = cur_ch;
                                last_station_ssid = cur_ssid.clone();
                                *watchdog_status.lock().unwrap() = format!(
                                    "Hotspot synchronized with '{}' ({}GHz, Ch {})",
                                    cur_ssid,
                                    if cur_mode == "a" { "5" } else { "2.4" },
                                    cur_ch
                                );
                                watchdog_ctx.request_repaint();
                            }
                        }
                    } else if !cur_ssid.is_empty() && cur_ssid != last_station_ssid {
                        last_station_ssid = cur_ssid.clone();
                        *watchdog_status.lock().unwrap() = format!(
                            "Hotspot running (Upstream connected to '{}', Ch {})",
                            cur_ssid, cur_ch
                        );
                        watchdog_ctx.request_repaint();
                    }
                }
            });
        });
    }

    fn stop_hotspot(&self, ctx: egui::Context) {
        let mut running = self.is_running.lock().unwrap();
        *running = false;
        *self.stop_watchdog.lock().unwrap() = true;

        self.set_status(ctx.clone(), "Stopping hotspot...");
        let ap_interface = self.ap_interface.clone();
        let interface = self.interface.clone();
        let blocked_macs_arc = self.blocked_macs.clone();
        let connected_devices_arc = self.connected_devices.clone();

        let status_arc = self.status_msg.clone();
        let modified_uuid_arc = self.modified_conn_uuid.clone();

        thread::spawn(move || {
            let _ = Command::new("killall").arg("dnsmasq").output();
            let _ = Command::new("killall").arg("hostapd").output();
            let _ = Command::new("ip").args(["link", "set", &ap_interface, "down"]).output();
            let _ = Command::new("iw").args(["dev", &ap_interface, "del"]).output();
            let _ = Command::new("iptables").args(["-t", "nat", "-D", "POSTROUTING", "-o", &interface, "-j", "MASQUERADE"]).output();
            let _ = Command::new("sysctl").args(["-w", "net.ipv4.ip_forward=0"]).output();

            // Clean up any iptables drop rules for blocked macs
            {
                let blocked = blocked_macs_arc.lock().unwrap();
                for b in blocked.iter() {
                    let _ = Command::new("iptables")
                        .args(["-D", "FORWARD", "-i", &ap_interface, "-m", "mac", "--mac-source", &b.mac, "-j", "DROP"])
                        .output();
                    let _ = Command::new("iptables")
                        .args(["-D", "INPUT", "-i", &ap_interface, "-m", "mac", "--mac-source", &b.mac, "-j", "DROP"])
                        .output();
                }
            }

            // Clear connected devices list
            connected_devices_arc.lock().unwrap().clear();

            // Check if we modified connection band for DFS resolution
            let maybe_uuid = modified_uuid_arc.lock().unwrap().take();
            if let Some(uuid) = maybe_uuid {
                let _ = Command::new("nmcli")
                    .args(["connection", "modify", &uuid, "802-11-wireless.bssid", "", "802-11-wireless.band", ""])
                    .status();
                let _ = Command::new("nmcli").args(["connection", "up", &uuid]).status();
                *status_arc.lock().unwrap() = "Hotspot stopped. Wi-Fi restored to Auto.".to_string();
            } else {
                *status_arc.lock().unwrap() = "Hotspot stopped.".to_string();
            }

            ctx.request_repaint();
        });
    }
}
