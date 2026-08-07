#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, PartialEq, Clone, Copy)]
enum BandMode {
    Auto,
    Force2_4GHz,
    Force5GHz,
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([460.0, 480.0])
            .with_title("Linux Wi-Fi Hotspot (Concurrent Mode)"),
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
    ap_interface: String,
    band_mode: BandMode,
    custom_channel: String,
    status_msg: Arc<Mutex<String>>,
    is_running: Arc<Mutex<bool>>,
}

impl Default for HotspotApp {
    fn default() -> Self {
        let detected = Self::detect_wireless_interfaces();
        let default_iface = detected.first().cloned().unwrap_or_else(|| "wlan0".to_string());
        Self {
            interface: default_iface,
            available_interfaces: detected,
            ssid: "RustHotspot".to_owned(),
            password: "password123".to_owned(),
            ap_interface: "ap0".to_owned(),
            band_mode: BandMode::Auto,
            custom_channel: "Auto".to_owned(),
            status_msg: Arc::new(Mutex::new("Ready".to_owned())),
            is_running: Arc::new(Mutex::new(false)),
        }
    }
}

impl eframe::App for HotspotApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        ui.heading("Wi-Fi Hotspot (AP/STA Concurrent)");
        ui.separator();

        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label("Wi-Fi Interface:");
            egui::ComboBox::from_id_salt("iface_combo")
                .selected_text(&self.interface)
                .show_ui(ui, |ui| {
                    for iface in &self.available_interfaces {
                        ui.selectable_value(&mut self.interface, iface.clone(), iface);
                    }
                });
            if ui.button("🔄").on_hover_text("Refresh interfaces").clicked() {
                self.available_interfaces = Self::detect_wireless_interfaces();
                if let Some(first) = self.available_interfaces.first() {
                    self.interface = first.clone();
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label("Hotspot SSID:");
            ui.text_edit_singleline(&mut self.ssid);
        });
        ui.horizontal(|ui| {
            ui.label("Hotspot Password:");
            ui.text_edit_singleline(&mut self.password);
        });

        ui.add_space(10.0);
        ui.label(egui::RichText::new("Frequency Band Config:").strong());
        
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.band_mode, BandMode::Auto, "Auto (Match Wi-Fi)");
            ui.selectable_value(&mut self.band_mode, BandMode::Force2_4GHz, "2.4 GHz");
            ui.selectable_value(&mut self.band_mode, BandMode::Force5GHz, "5 GHz");
        });

        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label("Channel:");
            egui::ComboBox::from_id_salt("channel_combo")
                .selected_text(&self.custom_channel)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.custom_channel, "Auto".to_string(), "Auto");
                    match self.band_mode {
                        BandMode::Force2_4GHz => {
                            for ch in [1, 6, 11] {
                                ui.selectable_value(&mut self.custom_channel, ch.to_string(), ch.to_string());
                            }
                        }
                        BandMode::Force5GHz => {
                            for ch in [36, 40, 44, 48, 149, 153, 157, 161, 165] {
                                ui.selectable_value(&mut self.custom_channel, ch.to_string(), ch.to_string());
                            }
                        }
                        BandMode::Auto => {
                            ui.label("Managed automatically to match Wi-Fi");
                        }
                    }
                });
        });

        ui.add_space(5.0);
        ui.label(
            egui::RichText::new("Note: Concurrent AP/STA requires the Hotspot channel and band to match your connected Wi-Fi channel.")
                .small()
                .italics()
                .color(egui::Color32::GRAY),
        );

        ui.add_space(15.0);

        let is_running = *self.is_running.lock().unwrap();

        if !is_running {
            if ui.button(egui::RichText::new("Start Hotspot").size(16.0)).clicked() {
                self.start_hotspot(ctx.clone());
            }
        } else {
            if ui.button(egui::RichText::new("Stop Hotspot").size(16.0).color(egui::Color32::RED)).clicked() {
                self.stop_hotspot(ctx.clone());
            }
        }

        ui.add_space(15.0);
        ui.label("Status:");

        let status = self.status_msg.lock().unwrap().clone();
        ui.label(egui::RichText::new(status).color(egui::Color32::YELLOW));
    }
}

impl HotspotApp {
    fn set_status(&self, ctx: egui::Context, msg: &str) {
        let mut status = self.status_msg.lock().unwrap();
        *status = msg.to_string();
        ctx.request_repaint();
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

    fn start_hotspot(&self, ctx: egui::Context) {
        let mut running = self.is_running.lock().unwrap();
        *running = true;

        self.set_status(ctx.clone(), "Starting hotspot (hostapd/dnsmasq)...");

        let interface = self.interface.clone();
        let ssid = self.ssid.clone();
        let password = self.password.clone();
        let ap_interface = self.ap_interface.clone();
        let band_mode = self.band_mode;
        let custom_channel = self.custom_channel.clone();

        let status_arc = self.status_msg.clone();
        let running_arc = self.is_running.clone();

        thread::spawn(move || {
            // Stop old instances
            let _ = Command::new("killall").arg("dnsmasq").output();
            let _ = Command::new("killall").arg("hostapd").output();
            let _ = Command::new("ip").args(["link", "set", &ap_interface, "down"]).output();
            let _ = Command::new("iw").args(["dev", &ap_interface, "del"]).output();

            // Remove old iptables rules
            let _ = Command::new("iptables").args(["-t", "nat", "-D", "POSTROUTING", "-o", &interface, "-j", "MASQUERADE"]).output();

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
                *status_arc.lock().unwrap() =
                    "Error: Failed to create AP interface. Do you have root privileges?"
                        .to_string();
                *running_arc.lock().unwrap() = false;
                ctx.request_repaint();
                return;
            }

            // Tell NetworkManager to IGNORE this interface so it doesn't lock it!
            Self::run_cmd("nmcli", &["device", "set", &ap_interface, "managed", "no"]);

            // Assign a unique MAC address so it doesn't conflict with the main Wi-Fi
            Self::run_cmd("ip", &["link", "set", "dev", &ap_interface, "address", "12:34:56:78:9a:bc"]);

            // Assign IP to AP interface
            Self::run_cmd("ip", &["addr", "add", "192.168.50.1/24", "broadcast", "192.168.50.255", "dev", &ap_interface]);
            Self::run_cmd("ip", &["link", "set", &ap_interface, "up"]);

            // Determine Channel and Mode
            let (auto_ch, auto_mode) = Self::get_current_channel_and_mode(&interface);

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

            // Create hostapd.conf
            let hostapd_conf_path = "/tmp/rust_hostapd.conf";
            let mut hostapd_config = format!(
                "interface={}\n\
                ssid={}\n\
                hw_mode={}\n\
                channel={}\n\
                macaddr_acl=0\n\
                auth_algs=1\n\
                ignore_broadcast_ssid=0\n\
                wpa=2\n\
                wpa_passphrase={}\n\
                wpa_key_mgmt=WPA-PSK\n\
                wpa_pairwise=TKIP\n\
                rsn_pairwise=CCMP\n",
                ap_interface, ssid, hw_mode, channel, password
            );
            if hw_mode == "a" {
                hostapd_config.push_str("ieee80211n=1\n");
            }
            std::fs::write(hostapd_conf_path, hostapd_config).expect("Failed to write hostapd.conf");

            // Create dnsmasq.conf
            let dnsmasq_conf_path = "/tmp/rust_dnsmasq.conf";
            let dnsmasq_config = format!(
                "interface={}\n\
                bind-interfaces\n\
                server=8.8.8.8\n\
                domain-needed\n\
                bogus-priv\n\
                dhcp-range=192.168.50.10,192.168.50.250,12h\n",
                ap_interface
            );
            std::fs::write(dnsmasq_conf_path, dnsmasq_config).expect("Failed to write dnsmasq.conf");

            // Enable IP forwarding and NAT
            Self::run_cmd("sysctl", &["-w", "net.ipv4.ip_forward=1"]);
            Self::run_cmd("iptables", &["-t", "nat", "-A", "POSTROUTING", "-o", &interface, "-j", "MASQUERADE"]);

            // Start hostapd and dnsmasq in background
            let mut hostapd_child = Command::new("hostapd").arg(hostapd_conf_path).spawn().expect("Failed to start hostapd");
            Command::new("dnsmasq").args(["-C", dnsmasq_conf_path, "-x", "/tmp/rust_dnsmasq.pid"]).spawn().expect("Failed to start dnsmasq");

            // Wait brief moment to check if hostapd crashed (e.g. channel mismatch)
            std::thread::sleep(std::time::Duration::from_millis(800));

            if let Ok(Some(status)) = hostapd_child.try_wait() {
                if !status.success() {
                    *status_arc.lock().unwrap() = format!(
                        "Error: Cannot force {}GHz (Ch {})! Wi-Fi is connected on {}GHz (Ch {}). Hardware only supports 1 band at a time.",
                        if hw_mode == "a" { "5" } else { "2.4" },
                        channel,
                        if auto_mode == "a" { "5" } else { "2.4" },
                        auto_ch
                    );
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
        });
    }

    fn stop_hotspot(&self, ctx: egui::Context) {
        let mut running = self.is_running.lock().unwrap();
        *running = false;

        self.set_status(ctx.clone(), "Stopping hotspot...");
        let ap_interface = self.ap_interface.clone();
        let interface = self.interface.clone();

        let status_arc = self.status_msg.clone();

        thread::spawn(move || {
            let _ = Command::new("killall").arg("dnsmasq").output();
            let _ = Command::new("killall").arg("hostapd").output();
            let _ = Command::new("ip").args(["link", "set", &ap_interface, "down"]).output();
            let _ = Command::new("iw").args(["dev", &ap_interface, "del"]).output();
            let _ = Command::new("iptables").args(["-t", "nat", "-D", "POSTROUTING", "-o", &interface, "-j", "MASQUERADE"]).output();
            let _ = Command::new("sysctl").args(["-w", "net.ipv4.ip_forward=0"]).output();

            *status_arc.lock().unwrap() = "Hotspot stopped.".to_string();
            ctx.request_repaint();
        });
    }
}
