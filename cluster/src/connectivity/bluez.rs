//! BlueZ D-Bus client (system bus) for headset / intercom control.

use sigma_instrumentation::connectivity::DeviceRow;
use zbus::Result;
use zbus::blocking::Connection;
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};

const BLUEZ: &str = "org.bluez";
const OM: &str = "org.freedesktop.DBus.ObjectManager";
const PROPS: &str = "org.freedesktop.DBus.Properties";
const ADAPTER: &str = "org.bluez.Adapter1";
const DEVICE: &str = "org.bluez.Device1";
const BATTERY: &str = "org.bluez.Battery1";

type Managed = std::collections::HashMap<
    OwnedObjectPath,
    std::collections::HashMap<String, std::collections::HashMap<String, OwnedValue>>,
>;

type ConnectedDevice = Option<(String, i32)>;

fn managed_objects(conn: &Connection) -> Result<Managed> {
    conn.call_method(Some(BLUEZ), "/", Some(OM), "GetManagedObjects", &())?
        .body()
        .deserialize()
}

fn prop_bool(map: &std::collections::HashMap<String, OwnedValue>, key: &str) -> bool {
    map.get(key)
        .and_then(|v| bool::try_from(v.clone()).ok())
        .unwrap_or(false)
}

fn prop_string(map: &std::collections::HashMap<String, OwnedValue>, key: &str) -> String {
    map.get(key)
        .and_then(|v| String::try_from(v.clone()).ok())
        .unwrap_or_default()
}

fn prop_i16(map: &std::collections::HashMap<String, OwnedValue>, key: &str) -> Option<i16> {
    map.get(key).and_then(|v| i16::try_from(v.clone()).ok())
}

fn prop_u8(map: &std::collections::HashMap<String, OwnedValue>, key: &str) -> Option<u8> {
    map.get(key).and_then(|v| u8::try_from(v.clone()).ok())
}

fn find_adapter(
    objects: &Managed,
) -> Option<(
    OwnedObjectPath,
    &std::collections::HashMap<String, OwnedValue>,
)> {
    for (path, ifaces) in objects {
        if let Some(props) = ifaces.get(ADAPTER) {
            return Some((path.clone(), props));
        }
    }
    None
}

/// Blocking BlueZ client for the adapter at [`ADAPTER_PATH`].
pub struct BlueZ {
    conn: Connection,
}

impl BlueZ {
    /// Connect to the system bus.
    pub fn connect() -> Result<Self> {
        Ok(Self {
            conn: Connection::system()?,
        })
    }

    /// Register the pairing agent on this connection (see [`super::agent`]).
    pub fn register_agent(&self) -> Result<()> {
        super::agent::register(&self.conn)
    }

    /// Whether the adapter is powered.
    pub fn powered(&self) -> Result<bool> {
        let objects = managed_objects(&self.conn)?;
        Ok(find_adapter(&objects)
            .map(|(_, p)| prop_bool(p, "Powered"))
            .unwrap_or(false))
    }

    /// Power the adapter on/off.
    pub fn set_powered(&self, on: bool) -> Result<()> {
        let objects = managed_objects(&self.conn)?;
        let Some((path, _)) = find_adapter(&objects) else {
            return Err(zbus::Error::Failure("no BlueZ adapter".into()));
        };
        self.conn.call_method(
            Some(BLUEZ),
            &path,
            Some(PROPS),
            "Set",
            &(ADAPTER, "Powered", Value::from(on)),
        )?;
        Ok(())
    }

    /// Begin device discovery (ignores "already discovering").
    pub fn start_discovery(&self) -> Result<()> {
        let objects = managed_objects(&self.conn)?;
        let Some((path, _)) = find_adapter(&objects) else {
            return Err(zbus::Error::Failure("no BlueZ adapter".into()));
        };
        // Ignore "InProgress".
        let _ = self
            .conn
            .call_method(Some(BLUEZ), &path, Some(ADAPTER), "StartDiscovery", &());
        Ok(())
    }

    /// List known devices plus the currently connected one (name, battery).
    pub fn devices(&self) -> Result<(Vec<DeviceRow>, ConnectedDevice)> {
        let objects = managed_objects(&self.conn)?;
        let mut rows = Vec::new();
        let mut connected: Option<(String, i32)> = None;

        for (path, ifaces) in &objects {
            let Some(dev) = ifaces.get(DEVICE) else {
                continue;
            };
            let alias = prop_string(dev, "Alias");
            let address = prop_string(dev, "Address");
            let title = if alias.is_empty() {
                address.clone()
            } else {
                alias
            };
            let paired = prop_bool(dev, "Paired");
            let is_connected = prop_bool(dev, "Connected");
            let rssi = prop_i16(dev, "RSSI");
            let battery = ifaces
                .get(BATTERY)
                .and_then(|b| prop_u8(b, "Percentage"))
                .map(i32::from)
                .unwrap_or(-1);

            let badge = if is_connected {
                "CONNECTED"
            } else if paired {
                "PAIRED"
            } else {
                "AVAIL"
            };
            let detail = match rssi {
                Some(r) => format!("{address}  {r} dBm"),
                None => address.clone(),
            };

            if is_connected {
                connected = Some((title.clone(), battery));
            }

            rows.push(DeviceRow {
                path: path.as_str().to_owned(),
                title,
                detail,
                badge: badge.into(),
                connected: is_connected,
                paired,
            });
        }

        rows.sort_by(|a, b| {
            b.connected
                .cmp(&a.connected)
                .then(b.paired.cmp(&a.paired))
                .then(a.title.cmp(&b.title))
        });

        Ok((rows, connected))
    }

    /// Connect the device at the given object `path`, pairing (and trusting,
    /// so it can reconnect on its own) first when needed.
    pub fn connect_device(&self, path: &str) -> Result<()> {
        let path = ObjectPath::try_from(path).map_err(|e| zbus::Error::Failure(e.to_string()))?;
        let objects = managed_objects(&self.conn)?;
        let owned = OwnedObjectPath::from(path.clone());
        if let Some(ifaces) = objects.get(&owned)
            && let Some(dev) = ifaces.get(DEVICE)
            && !prop_bool(dev, "Paired")
        {
            let _ = self
                .conn
                .call_method(Some(BLUEZ), &path, Some(DEVICE), "Pair", &());
        }
        // Trusted lets the headset re-establish profiles without a fresh
        // authorization round-trip on every ignition cycle.
        let _ = self.conn.call_method(
            Some(BLUEZ),
            &path,
            Some(PROPS),
            "Set",
            &(DEVICE, "Trusted", Value::from(true)),
        );
        self.conn
            .call_method(Some(BLUEZ), &path, Some(DEVICE), "Connect", &())?;
        Ok(())
    }

    /// Disconnect the device at the given object `path`.
    pub fn disconnect_device(&self, path: &str) -> Result<()> {
        let path = ObjectPath::try_from(path).map_err(|e| zbus::Error::Failure(e.to_string()))?;
        self.conn
            .call_method(Some(BLUEZ), &path, Some(DEVICE), "Disconnect", &())?;
        Ok(())
    }
}
