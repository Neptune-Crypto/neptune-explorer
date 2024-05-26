use crate::model::app_state::AppState;
use crate::model::config::Config;
use crate::model::config::SmtpMode;
use clap::Parser;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use tracing::{info, warn};

pub fn can_send_alerts() -> bool {
    Config::parse().alert_config().is_some()
}

pub async fn send(
    state: &AppState,
    subject: &str,
    body: String,
) -> std::result::Result<bool, anyhow::Error> {
    match state.read().await.config.alert_config() {
        None => {
            warn!("Alert emails disabled.  alert not sent.  consider confiuring smtp parameters.  subject: {subject}");
            Ok(false)
        }
        Some(alert) => {
            let email = Message::builder()
                .from(alert.smtp_from_email.parse()?)
                .to(alert.admin_email.parse()?)
                .subject(subject)
                .body(body)?;

            // Create SMTP client credentials using username and password
            let creds = Credentials::new(alert.smtp_user.to_string(), alert.smtp_pass.to_string());

            // Open a secure connection to the SMTP server, possibly using STARTTLS
            let relay = match alert.smtp_mode {
                SmtpMode::Starttls => {
                    AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&alert.smtp_host)?
                }
                SmtpMode::Smtps => AsyncSmtpTransport::<Tokio1Executor>::relay(&alert.smtp_host)?,
            };
            let mailer = relay.credentials(creds).build();

            // Attempt to send the email via the SMTP transport
            match mailer.send(email).await {
                Ok(_) => info!(
                    "alert email sent successfully.  to: {}, subject: {subject}, smtp_host: {}",
                    alert.admin_email, alert.smtp_host
                ),
                Err(e) => warn!(
                    "error send alert email. to: {}, subject: {subject}, smtp_host: {}.  error: {:?}",
                    alert.admin_email, alert.smtp_host, e
                ),
            }

            Ok(true)
        }
    }
}
