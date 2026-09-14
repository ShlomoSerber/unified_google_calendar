//! `channels.stop`. See docs/05-sincronizacion.md section 3.3.

use reqwest::Method;
use serde_json::json;

use crate::error::AppError;
use crate::google::client::Client;

impl Client {
    pub async fn channel_stop(
        &self,
        token: &str,
        id: &str,
        resource_id: &str,
    ) -> Result<(), AppError> {
        self.send_text(
            token,
            Method::POST,
            "/channels/stop",
            &[],
            Some(&json!({ "id": id, "resourceId": resource_id })),
        )
        .await?;
        Ok(())
    }
}
