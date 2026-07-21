//! ConnMan D-Bus client (system bus) for Wi-Fi power and association.

use sigma_instrumentation::connectivity::NetworkRow;
use zbus::Result;
use zbus::blocking::Connection;
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};

const CONNMAN: &str = "net.connman";
const MANAGER: &str = "net.connman.Manager";
const TECH: &str = "net.connman.Technology";
const SERVICE: &str = "net.connman.Service";
const ROOT: &str = "/";

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

fn prop_u8(map: &std::collections::HashMap<String, OwnedValue>, key: &str) -> Option<u8> {
    map.get(key).and_then(|v| u8::try_from(v.clone()).ok())
}

fn prop_strings(map: &std::collections::HashMap<String, OwnedValue>, key: &str) -> Vec<String> {
    map.get(key)
        .and_then(|v| Vec::<String>::try_from(v.clone()).ok())
        .unwrap_or_default()
}

type TechList = Vec<(
    OwnedObjectPath,
    std::collections::HashMap<String, OwnedValue>,
)>;
type ServiceList = Vec<(
    OwnedObjectPath,
    std::collections::HashMap<String, OwnedValue>,
)>;

/// Blocking connman client for Wi-Fi technology and service control.
pub struct ConnMan {
    conn: Connection,
}

impl ConnMan {
    /// Connect to the system bus.
    pub fn connect() -> Result<Self> {
        Ok(Self {
            conn: Connection::system()?,
        })
    }

    fn technologies(&self) -> Result<TechList> {
        self.conn
            .call_method(Some(CONNMAN), ROOT, Some(MANAGER), "GetTechnologies", &())?
            .body()
            .deserialize()
    }

    fn services(&self) -> Result<ServiceList> {
        self.conn
            .call_method(Some(CONNMAN), ROOT, Some(MANAGER), "GetServices", &())?
            .body()
            .deserialize()
    }

    fn wifi_tech(
        &self,
    ) -> Result<
        Option<(
            OwnedObjectPath,
            std::collections::HashMap<String, OwnedValue>,
        )>,
    > {
        Ok(self
            .technologies()?
            .into_iter()
            .find(|(_, props)| prop_string(props, "Type") == "wifi"))
    }

    /// Whether the Wi-Fi technology is powered.
    pub fn wifi_powered(&self) -> Result<bool> {
        Ok(self
            .wifi_tech()?
            .map(|(_, p)| prop_bool(&p, "Powered"))
            .unwrap_or(false))
    }

    /// Power Wi-Fi on/off.
    pub fn set_wifi_powered(&self, on: bool) -> Result<()> {
        let Some((path, _)) = self.wifi_tech()? else {
            return Err(zbus::Error::Failure("no Wi-Fi technology".into()));
        };
        self.conn.call_method(
            Some(CONNMAN),
            &path,
            Some(TECH),
            "SetProperty",
            &("Powered", Value::from(on)),
        )?;
        Ok(())
    }

    /// Trigger a Wi-Fi scan (ignores in-progress errors).
    pub fn scan_wifi(&self) -> Result<()> {
        let Some((path, _)) = self.wifi_tech()? else {
            return Err(zbus::Error::Failure("no Wi-Fi technology".into()));
        };
        self.conn
            .call_method(Some(CONNMAN), &path, Some(TECH), "Scan", &())?;
        Ok(())
    }

    /// List Wi-Fi services plus the SSID of the online one, if any.
    pub fn networks(&self) -> Result<(Vec<NetworkRow>, Option<String>)> {
        let mut rows = Vec::new();
        let mut online: Option<String> = None;

        for (path, props) in self.services()? {
            if prop_string(&props, "Type") != "wifi" {
                continue;
            }
            let name = prop_string(&props, "Name");
            if name.is_empty() {
                continue;
            }
            let state = prop_string(&props, "State");
            let strength = prop_u8(&props, "Strength").unwrap_or(0);
            let favorite = prop_bool(&props, "Favorite");
            let security = prop_strings(&props, "Security");
            let secured = security.iter().any(|s| s != "none");
            let connected = matches!(state.as_str(), "online" | "ready");

            let badge = if connected {
                "ONLINE"
            } else if favorite {
                "SAVED"
            } else if secured {
                "SECURE"
            } else {
                "OPEN"
            };
            let bars = match strength {
                0..=20 => "*",
                21..=40 => "**",
                41..=60 => "***",
                61..=80 => "****",
                _ => "*****",
            };
            let detail = format!("{bars}  {state}");

            if connected {
                online = Some(name.clone());
            }

            rows.push(NetworkRow {
                path: path.as_str().to_owned(),
                title: name,
                detail,
                badge: badge.into(),
                connected,
                favorite,
            });
        }

        rows.sort_by(|a, b| {
            b.connected
                .cmp(&a.connected)
                .then(b.favorite.cmp(&a.favorite))
                .then(a.title.cmp(&b.title))
        });

        Ok((rows, online))
    }

    /// Connect the service at the given object `path`.
    pub fn connect_service(&self, path: &str) -> Result<()> {
        let path = ObjectPath::try_from(path).map_err(|e| zbus::Error::Failure(e.to_string()))?;
        self.conn
            .call_method(Some(CONNMAN), &path, Some(SERVICE), "Connect", &())?;
        Ok(())
    }

    /// Disconnect the service at the given object `path`.
    pub fn disconnect_service(&self, path: &str) -> Result<()> {
        let path = ObjectPath::try_from(path).map_err(|e| zbus::Error::Failure(e.to_string()))?;
        self.conn
            .call_method(Some(CONNMAN), &path, Some(SERVICE), "Disconnect", &())?;
        Ok(())
    }
}
