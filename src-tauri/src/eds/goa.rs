//! GNOME Online Accounts: turn Calendar off for Google accounts so the panel does not show
//! events twice. See docs/06-integracion-gnome.md section 4.6. Implemented in F6-T4.

use std::collections::HashMap;

use zbus::zvariant::OwnedObjectPath;
use zbus::{Connection, Proxy};

use crate::error::AppError;

pub const GOA_BUS: &str = "org.gnome.OnlineAccounts";
pub const GOA_MANAGER_PATH: &str = "/org/gnome/OnlineAccounts";
pub const IFACE_ACCOUNT: &str = "org.gnome.OnlineAccounts.Account";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoaGoogleAccount {
    pub path: OwnedObjectPath,
    pub id: String,
    pub identity: String,
    pub calendar_disabled: bool,
}

fn err(e: zbus::Error) -> AppError {
    AppError::Network(format!("goa dbus: {e}"))
}

/// Google accounts registered in Online Accounts.
pub async fn google_accounts() -> Result<Vec<GoaGoogleAccount>, AppError> {
    let conn = Connection::session().await.map_err(err)?;
    let om = Proxy::new(
        &conn,
        GOA_BUS,
        GOA_MANAGER_PATH,
        "org.freedesktop.DBus.ObjectManager",
    )
    .await
    .map_err(err)?;
    let objects: HashMap<
        OwnedObjectPath,
        HashMap<String, HashMap<String, zbus::zvariant::OwnedValue>>,
    > = om.call("GetManagedObjects", &()).await.map_err(err)?;
    let mut out = Vec::new();
    for (path, ifaces) in objects {
        let Some(props) = ifaces.get(IFACE_ACCOUNT) else {
            continue;
        };
        let provider = props
            .get("ProviderType")
            .and_then(|v| String::try_from(v.clone()).ok())
            .unwrap_or_default();
        if provider != "google" {
            continue;
        }
        out.push(GoaGoogleAccount {
            path,
            id: props
                .get("Id")
                .and_then(|v| String::try_from(v.clone()).ok())
                .unwrap_or_default(),
            identity: props
                .get("PresentationIdentity")
                .and_then(|v| String::try_from(v.clone()).ok())
                .unwrap_or_default(),
            calendar_disabled: props
                .get("CalendarDisabled")
                .and_then(|v| bool::try_from(v.clone()).ok())
                .unwrap_or(false),
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// `CalendarDisabled = true` on every Google account (same as the switch in Settings).
pub async fn disable_google_calendars() -> Result<usize, AppError> {
    let conn = Connection::session().await.map_err(err)?;
    let mut n = 0;
    for a in google_accounts()
        .await?
        .into_iter()
        .filter(|a| !a.calendar_disabled)
    {
        let p = Proxy::new(&conn, GOA_BUS, a.path.as_str(), IFACE_ACCOUNT)
            .await
            .map_err(err)?;
        p.set_property("CalendarDisabled", true)
            .await
            .map_err(|e| AppError::Network(format!("goa set-property: {e}")))?;
        tracing::info!(account = %a.identity, "GOA calendar disabled");
        n += 1;
    }
    Ok(n)
}
