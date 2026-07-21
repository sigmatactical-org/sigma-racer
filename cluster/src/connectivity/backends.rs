//! Aggregation of the two optional D-Bus backends (BlueZ, connman).

use super::bluez::BlueZ;
use super::connman::ConnMan;
use crate::log::log;
use sigma_instrumentation::connectivity::{Action, Snapshot};

/// The connectivity window's view of the system: each backend is `None` when
/// its D-Bus service could not be reached at start-up.
pub(super) struct Backends {
    bluez: Option<BlueZ>,
    connman: Option<ConnMan>,
    /// Pairing agent registered and adapter powered — retried on refresh
    /// until bluetoothd and its adapter are actually up (slow boot).
    bt_ready: std::cell::Cell<bool>,
}

impl Backends {
    /// Connect to both D-Bus services; a failed backend is logged and left
    /// as `None` so the window still works with whatever is available.
    pub(super) fn open() -> Self {
        let bluez = match BlueZ::connect() {
            Ok(b) => Some(b),
            Err(err) => {
                log!("BlueZ D-Bus unavailable: {err}");
                None
            }
        };
        let connman = match ConnMan::connect() {
            Ok(c) => Some(c),
            Err(err) => {
                log!("connman D-Bus unavailable: {err}");
                None
            }
        };
        let this = Self {
            bluez,
            connman,
            bt_ready: std::cell::Cell::new(false),
        };
        this.ensure_bt_ready();
        this
    }

    /// One-shot bring-up: register the pairing agent and power the adapter.
    /// Safe to call repeatedly; does nothing once it has succeeded.
    fn ensure_bt_ready(&self) {
        if self.bt_ready.get() {
            return;
        }
        let Some(bt) = &self.bluez else { return };
        if let Err(err) = bt.register_agent() {
            log!("BlueZ agent registration pending: {err}");
            return;
        }
        if let Err(err) = bt.set_powered(true) {
            log!("BlueZ adapter power-on pending: {err}");
            return;
        }
        self.bt_ready.set(true);
    }

    /// Re-read power state, device and network lists into `snap`, folding
    /// backend errors into the window's status line.
    pub(super) fn refresh(&self, snap: &mut Snapshot) {
        self.ensure_bt_ready();
        snap.available = self.bluez.is_some() || self.connman.is_some();
        let mut notes = Vec::new();

        if let Some(bt) = &self.bluez {
            match bt.powered() {
                Ok(on) => snap.bt_powered = on,
                Err(err) => notes.push(format!("BT: {err}")),
            }
            match bt.devices() {
                Ok((devices, connected)) => {
                    snap.devices = devices;
                    if let Some((name, batt)) = connected {
                        snap.bt_connected = true;
                        snap.bt_device = name;
                        snap.bt_battery = batt;
                    } else {
                        snap.bt_connected = false;
                        if !snap.bt_powered {
                            snap.bt_device.clear();
                        }
                        snap.bt_battery = -1;
                    }
                }
                Err(err) => notes.push(format!("BT devices: {err}")),
            }
        } else {
            notes.push("BlueZ offline".into());
        }

        if let Some(cm) = &self.connman {
            match cm.wifi_powered() {
                Ok(on) => snap.wifi_powered = on,
                Err(err) => notes.push(format!("Wi-Fi: {err}")),
            }
            match cm.networks() {
                Ok((networks, online)) => {
                    snap.networks = networks;
                    if let Some(ssid) = online {
                        snap.wifi_connected = true;
                        snap.wifi_ssid = ssid;
                    } else {
                        snap.wifi_connected = false;
                        if !snap.wifi_powered {
                            snap.wifi_ssid.clear();
                        }
                    }
                }
                Err(err) => notes.push(format!("Wi-Fi nets: {err}")),
            }
        } else {
            notes.push("connman offline".into());
        }

        if !snap.busy {
            if !snap.available {
                snap.status = notes.join(" · ");
            } else if snap.status.contains("offline") || snap.status.starts_with("Starting") {
                snap.status.clear();
            }
        }
    }

    /// Execute a menu action against the owning backend and refresh `snap`
    /// with the outcome (status text on success or failure).
    pub(super) fn run_action(&self, action: Action, snap: &mut Snapshot) {
        snap.busy = true;
        let result = match action {
            Action::ToggleBt => {
                if let Some(bt) = &self.bluez {
                    let on = !snap.bt_powered;
                    bt.set_powered(on)
                        .map(|_| {
                            snap.bt_powered = on;
                            format!("Bluetooth {}", if on { "on" } else { "off" })
                        })
                        .map_err(|e| e.to_string())
                } else {
                    Err("BlueZ unavailable".into())
                }
            }
            Action::OpenBtList | Action::BtScan => {
                if let Some(bt) = &self.bluez {
                    bt.start_discovery()
                        .map(|_| "Scanning for headsets…".into())
                        .map_err(|e| e.to_string())
                } else {
                    Err("BlueZ unavailable".into())
                }
            }
            Action::ToggleWifi => {
                if let Some(cm) = &self.connman {
                    let on = !snap.wifi_powered;
                    cm.set_wifi_powered(on)
                        .map(|_| format!("Wi-Fi {}", if on { "on" } else { "off" }))
                        .map_err(|e| e.to_string())
                } else {
                    Err("connman unavailable".into())
                }
            }
            Action::OpenWifiList | Action::WifiScan => {
                if let Some(cm) = &self.connman {
                    cm.scan_wifi()
                        .map(|_| "Scanning Wi-Fi…".into())
                        .map_err(|e| e.to_string())
                } else {
                    Err("connman unavailable".into())
                }
            }
            Action::SelectDevice(i) => {
                if let Some(bt) = &self.bluez {
                    if let Some(dev) = snap.devices.get(i).cloned() {
                        if dev.connected {
                            bt.disconnect_device(&dev.path)
                                .map(|_| format!("Disconnected {}", dev.title))
                                .map_err(|e| e.to_string())
                        } else {
                            bt.connect_device(&dev.path)
                                .map(|_| format!("Connecting {}", dev.title))
                                .map_err(|e| e.to_string())
                        }
                    } else {
                        Err("No device".into())
                    }
                } else {
                    Err("BlueZ unavailable".into())
                }
            }
            Action::SelectNetwork(i) => {
                if let Some(cm) = &self.connman {
                    if let Some(net) = snap.networks.get(i).cloned() {
                        if net.connected {
                            cm.disconnect_service(&net.path)
                                .map(|_| format!("Disconnected {}", net.title))
                                .map_err(|e| e.to_string())
                        } else if net.badge == "SECURE" && !net.favorite {
                            Err("Password required — provision with connmanctl first".into())
                        } else {
                            cm.connect_service(&net.path)
                                .map(|_| format!("Connecting {}", net.title))
                                .map_err(|e| e.to_string())
                        }
                    } else {
                        Err("No network".into())
                    }
                } else {
                    Err("connman unavailable".into())
                }
            }
        };
        match result {
            Ok(msg) => snap.status = msg,
            Err(err) => snap.status = err,
        }
        snap.busy = false;
        self.refresh(snap);
    }
}
