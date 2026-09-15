use crate::state::RMQ_SETTINGS;
use serde::Serialize;
use reqwest::Client;


/// Function to send notification to RabbitMQ using HTTP API
pub async fn send_notification_to_rmq<T>(notification_type: &str, payload: &T) -> Result<(), Box<dyn std::error::Error>>
    where
        T: Serialize,
{
    let client = Client::new();

    let rmq_url = format!(
        "https://{}/api/exchanges/{}/{}/publish",
        RMQ_SETTINGS.domain,
        RMQ_SETTINGS.virtual_host,
        RMQ_SETTINGS.exchange_name
    );

    let response = client.post(&rmq_url)
        .header("authorization", format!("Basic {}", RMQ_SETTINGS.auth_token))
        .header("content-type", "text/plain;charset=UTF-8")
        .body(serde_json::json!({
            "vhost": RMQ_SETTINGS.virtual_host,
            "name": RMQ_SETTINGS.exchange_name,
            "properties": {
                "delivery_mode": 2,
                "headers": {
                    "type": notification_type
                }
            },
            "routing_key": RMQ_SETTINGS.routing_key,
            "delivery_mode": "2",
            "payload": serde_json::to_string(payload)?,
            "payload_encoding": "string",
            "props": {}
        }).to_string())
        .send()
        .await?;

    if response.status().is_success() {
        log::info!("Notification sent successfully to RMQ");
        Ok(())
    } else {
        let error_text = response.text().await?;
        log::error!("Failed to send notification to RMQ: {}", error_text);
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to send notification to RMQ: {}", error_text),
        )))
    }
}
