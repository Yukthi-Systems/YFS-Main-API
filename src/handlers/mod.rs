use chrono::Datelike;

pub mod storage_api;
pub mod deletion;
pub mod access;
pub mod jobs;
pub mod auth;
mod rmq;



pub async fn send_sms_2fa_notification(phone_number: String, otp_code: String) {
    // Send notification to RabbitMQ (In background)
    rmq::send_notification_to_rmq(
        "sms",
        &serde_json::json!({
            "to": phone_number,
            "template": "aio_otp",
            "variables": {
                "to": phone_number,
                "otp": otp_code
            }
        })
    ).await.unwrap_or_else(|err| {
        log::error!("Failed to send SMS 2FA notification: {}", err);
    });
}


pub async fn send_email_2fa_notification(email: String, otp_code: String, org_name: String, user_name: String) {
    // Send notification to RabbitMQ (In background)
    rmq::send_notification_to_rmq(
        "email",
        &serde_json::json!({
            "to": email,
            "template": "otp_verification",
            "variables": {
                "otp": otp_code,
                "organization_name": org_name,
                "name": user_name,
                "year": chrono::Utc::now().year()
            }
        })
    ).await.unwrap_or_else(|err| {
        log::error!("Failed to send Email 2FA notification: {}", err);
    });
}
