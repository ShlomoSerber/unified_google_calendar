//! D-Bus calls to Evolution Data Server. See docs/06-integracion-gnome.md section 4.3 and 4.4.
//!
//! Bus names are discovered with `ListNames` (`Sources5`, `Calendar8` change between EDS
//! versions); `Source_N` paths are resolved through `GetManagedObjects` by the `UID` property.

use std::collections::HashMap;

use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};
use zbus::{Connection, Proxy};

use crate::error::AppError;

pub const SOURCE_MANAGER_PATH: &str = "/org/gnome/evolution/dataserver/SourceManager";
pub const CALENDAR_FACTORY_PATH: &str = "/org/gnome/evolution/dataserver/CalendarFactory";
pub const IFACE_SOURCE: &str = "org.gnome.evolution.dataserver.Source";
pub const IFACE_SOURCE_WRITABLE: &str = "org.gnome.evolution.dataserver.Source.Writable";
pub const IFACE_SOURCE_REMOVABLE: &str = "org.gnome.evolution.dataserver.Source.Removable";
pub const IFACE_SOURCE_MANAGER: &str = "org.gnome.evolution.dataserver.SourceManager";
pub const IFACE_OBJECT_MANAGER: &str = "org.freedesktop.DBus.ObjectManager";
pub const IFACE_CALENDAR_FACTORY: &str = "org.gnome.evolution.dataserver.CalendarFactory";
pub const IFACE_CALENDAR: &str = "org.gnome.evolution.dataserver.Calendar";

fn err(e: zbus::Error) -> AppError {
    AppError::Network(format!("eds dbus: {e}"))
}

/// Pick the highest-numbered bus name with `prefix` (e.g. `org.gnome.evolution.dataserver.Sources`).
pub fn pick_bus<'a>(names: impl IntoIterator<Item = &'a str>, prefix: &str) -> Option<String> {
    names
        .into_iter()
        .filter(|n| n.starts_with(prefix) && n[prefix.len()..].chars().all(|c| c.is_ascii_digit()))
        .max_by_key(|n| n[prefix.len()..].parse::<u32>().unwrap_or(0))
        .map(str::to_string)
}

#[derive(Debug, Clone)]
pub struct SourceInfo {
    pub path: OwnedObjectPath,
    pub uid: String,
    pub data: String,
    pub removable: bool,
}

pub struct Eds {
    pub conn: Connection,
    pub sources_bus: String,
    pub calendar_bus: String,
}

impl Eds {
    /// Connect to the session bus and discover the registry and calendar factory names.
    pub async fn connect() -> Result<Eds, AppError> {
        let conn = Connection::session().await.map_err(err)?;
        let dbus = Proxy::new(
            &conn,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
        )
        .await
        .map_err(err)?;
        let names: Vec<String> = dbus.call("ListNames", &()).await.map_err(err)?;
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let sources_bus = pick_bus(
            refs.iter().copied(),
            "org.gnome.evolution.dataserver.Sources",
        )
        .ok_or_else(|| {
            AppError::Network("evolution-source-registry is not on the session bus".into())
        })?;
        let calendar_bus = pick_bus(
            refs.iter().copied(),
            "org.gnome.evolution.dataserver.Calendar",
        )
        .ok_or_else(|| {
            AppError::Network("evolution-calendar-factory is not on the session bus".into())
        })?;
        Ok(Eds {
            conn,
            sources_bus,
            calendar_bus,
        })
    }

    /// `GetManagedObjects` → sources keyed by UID.
    pub async fn sources(&self) -> Result<HashMap<String, SourceInfo>, AppError> {
        let om = Proxy::new(
            &self.conn,
            self.sources_bus.as_str(),
            SOURCE_MANAGER_PATH,
            IFACE_OBJECT_MANAGER,
        )
        .await
        .map_err(err)?;
        let objects: HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>> =
            om.call("GetManagedObjects", &()).await.map_err(err)?;
        let mut out = HashMap::new();
        for (path, ifaces) in objects {
            let Some(props) = ifaces.get(IFACE_SOURCE) else {
                continue;
            };
            let uid = props
                .get("UID")
                .and_then(|v| String::try_from(v.clone()).ok())
                .unwrap_or_default();
            if uid.is_empty() {
                continue;
            }
            let data = props
                .get("Data")
                .and_then(|v| String::try_from(v.clone()).ok())
                .unwrap_or_default();
            let removable = ifaces.contains_key(IFACE_SOURCE_REMOVABLE);
            out.insert(
                uid.clone(),
                SourceInfo {
                    path,
                    uid,
                    data,
                    removable,
                },
            );
        }
        Ok(out)
    }

    pub async fn create_source(&self, uid: &str, keyfile: &str) -> Result<(), AppError> {
        let sm = Proxy::new(
            &self.conn,
            self.sources_bus.as_str(),
            SOURCE_MANAGER_PATH,
            IFACE_SOURCE_MANAGER,
        )
        .await
        .map_err(err)?;
        let mut map: HashMap<&str, &str> = HashMap::new();
        map.insert(uid, keyfile);
        sm.call::<_, _, ()>("CreateSources", &(map,))
            .await
            .map_err(err)?;
        Ok(())
    }

    pub async fn write_source(
        &self,
        path: &OwnedObjectPath,
        keyfile: &str,
    ) -> Result<(), AppError> {
        let p = Proxy::new(
            &self.conn,
            self.sources_bus.as_str(),
            path.as_str(),
            IFACE_SOURCE_WRITABLE,
        )
        .await
        .map_err(err)?;
        p.call::<_, _, ()>("Write", &(keyfile,))
            .await
            .map_err(err)?;
        Ok(())
    }

    pub async fn remove_source(&self, path: &OwnedObjectPath) -> Result<(), AppError> {
        let p = Proxy::new(
            &self.conn,
            self.sources_bus.as_str(),
            path.as_str(),
            IFACE_SOURCE_REMOVABLE,
        )
        .await
        .map_err(err)?;
        p.call::<_, _, ()>("Remove", &()).await.map_err(err)?;
        Ok(())
    }

    /// `OpenCalendar` + `Open`. Returns a handle for object calls.
    pub async fn open_calendar(&self, uid: &str) -> Result<OpenCalendar, AppError> {
        let f = Proxy::new(
            &self.conn,
            self.calendar_bus.as_str(),
            CALENDAR_FACTORY_PATH,
            IFACE_CALENDAR_FACTORY,
        )
        .await
        .map_err(err)?;
        let (path, bus): (String, String) = f.call("OpenCalendar", &(uid,)).await.map_err(err)?;
        let proxy = Proxy::new(&self.conn, bus.clone(), path.clone(), IFACE_CALENDAR)
            .await
            .map_err(err)?;
        let _props: Vec<String> = proxy.call("Open", &()).await.map_err(err)?;
        Ok(OpenCalendar { proxy })
    }
}

pub struct OpenCalendar {
    proxy: Proxy<'static>,
}

impl OpenCalendar {
    pub async fn objects(&self, query: &str) -> Result<Vec<String>, AppError> {
        self.proxy
            .call("GetObjectList", &(query,))
            .await
            .map_err(err)
    }

    pub async fn create(&self, vevents: &[String]) -> Result<Vec<String>, AppError> {
        if vevents.is_empty() {
            return Ok(vec![]);
        }
        self.proxy
            .call("CreateObjects", &(vevents, 0u32))
            .await
            .map_err(err)
    }

    pub async fn modify(&self, vevents: &[String]) -> Result<(), AppError> {
        if vevents.is_empty() {
            return Ok(());
        }
        self.proxy
            .call::<_, _, ()>("ModifyObjects", &(vevents, "all", 0u32))
            .await
            .map_err(err)
    }

    pub async fn remove(&self, uids: &[String]) -> Result<(), AppError> {
        if uids.is_empty() {
            return Ok(());
        }
        let pairs: Vec<(String, String)> =
            uids.iter().map(|u| (u.clone(), String::new())).collect();
        self.proxy
            .call::<_, _, ()>("RemoveObjects", &(pairs, "all", 0u32))
            .await
            .map_err(err)
    }

    pub async fn close(self) {
        let _ = self.proxy.call::<_, _, ()>("Close", &()).await;
    }
}

/// Helper for tests of value handling.
pub fn value_to_string(v: &Value<'_>) -> Option<String> {
    match v {
        Value::Str(s) => Some(s.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_highest_suffix() {
        let names = [
            "org.gnome.evolution.dataserver.Sources5",
            "org.gnome.evolution.dataserver.Sources12",
            "org.gnome.evolution.dataserver.Calendar8",
            "org.gnome.Shell",
        ];
        assert_eq!(
            pick_bus(names, "org.gnome.evolution.dataserver.Sources").as_deref(),
            Some("org.gnome.evolution.dataserver.Sources12")
        );
        assert_eq!(
            pick_bus(names, "org.gnome.evolution.dataserver.Calendar").as_deref(),
            Some("org.gnome.evolution.dataserver.Calendar8")
        );
        assert!(pick_bus(names, "org.gnome.evolution.dataserver.AddressBook").is_none());
    }
}
