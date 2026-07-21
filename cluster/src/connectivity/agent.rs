//! BlueZ pairing agent (`org.bluez.Agent1`, capability `NoInputNoOutput`).
//!
//! Headsets pair over Just-Works SSP, but bluetoothd still refuses to pair
//! without a registered agent. This one auto-accepts everything a
//! NoInputNoOutput agent can be asked; the rider confirms intent by
//! selecting the device on the cluster, there is no further UI.

use crate::log::log;
use zbus::blocking::Connection;
use zbus::zvariant::{ObjectPath, OwnedObjectPath};

const BLUEZ: &str = "org.bluez";
const AGENT_MANAGER: &str = "org.bluez.AgentManager1";
const AGENT_PATH: &str = "/io/sigmatactical/cluster/agent";
const CAPABILITY: &str = "NoInputNoOutput";

struct NoIoAgent;

#[zbus::interface(name = "org.bluez.Agent1")]
impl NoIoAgent {
    fn release(&self) {}

    fn request_pin_code(&self, _device: OwnedObjectPath) -> zbus::fdo::Result<String> {
        Ok("0000".into())
    }

    fn display_pin_code(&self, _device: OwnedObjectPath, _pincode: String) {}

    fn request_passkey(&self, _device: OwnedObjectPath) -> zbus::fdo::Result<u32> {
        Ok(0)
    }

    fn display_passkey(&self, _device: OwnedObjectPath, _passkey: u32, _entered: u16) {}

    fn request_confirmation(
        &self,
        _device: OwnedObjectPath,
        _passkey: u32,
    ) -> zbus::fdo::Result<()> {
        Ok(())
    }

    fn request_authorization(&self, _device: OwnedObjectPath) -> zbus::fdo::Result<()> {
        Ok(())
    }

    fn authorize_service(&self, _device: OwnedObjectPath, _uuid: String) -> zbus::fdo::Result<()> {
        Ok(())
    }

    fn cancel(&self) {}
}

/// Export the agent on `conn` and make it bluetoothd's default agent.
/// Idempotent: re-registration after an app restart is not an error.
pub(super) fn register(conn: &Connection) -> zbus::Result<()> {
    let served = conn.object_server().at(AGENT_PATH, NoIoAgent)?;
    if served {
        log!("BlueZ agent exported at {AGENT_PATH}");
    }

    let path = ObjectPath::try_from(AGENT_PATH).expect("static agent path");
    match conn.call_method(
        Some(BLUEZ),
        "/org/bluez",
        Some(AGENT_MANAGER),
        "RegisterAgent",
        &(&path, CAPABILITY),
    ) {
        Ok(_) => {}
        // Already registered from a previous life of this connection.
        Err(zbus::Error::MethodError(ref name, ..))
            if name.as_str() == "org.bluez.Error.AlreadyExists" => {}
        Err(err) => return Err(err),
    }

    conn.call_method(
        Some(BLUEZ),
        "/org/bluez",
        Some(AGENT_MANAGER),
        "RequestDefaultAgent",
        &(&path,),
    )?;
    log!("BlueZ agent registered ({CAPABILITY}) and set as default");
    Ok(())
}
