use clap::Parser;
use lettre::AsyncSmtpTransport;
use lettre::AsyncTransport;
use lettre::Message;
use lettre::Tokio1Executor;
use tracing::info;
use tracing::warn;

use crate::model::app_state::AppState;
use crate::model::config::AlertConfig;
use crate::model::config::Config;
use crate::model::config::SmtpMode;

pub fn check_alert_params() -> bool {
    match Config::parse().alert_config() {
        Some(alert) => match gen_smtp_transport(&alert) {
            Ok(_) => true,
            Err(e) => {
                warn!(
                    "invalid smtp parameters. alert emails disabled. error: {:?}",
                    e.to_string()
                );
                false
            }
        },
        None => {
            warn!("alert emails disabled.  consider configuring smtp parameters.");
            false
        }
    }
}

pub fn gen_smtp_transport(
    alert: &AlertConfig,
) -> Result<AsyncSmtpTransport<Tokio1Executor>, lettre::transport::smtp::Error> {
    // corresponds to table at:
    // https://docs.rs/lettre/0.11.7/lettre/transport/smtp/struct.AsyncSmtpTransport.html#method.from_url
    let (scheme, tls_arg) = match alert.smtp_mode {
        SmtpMode::Smtps => ("smtps", ""),
        SmtpMode::Starttls => ("smtp", "?tls=required"),
        SmtpMode::Opportunistic => ("smtp", "?tls=opportunistic"),
        SmtpMode::Plaintext => ("smtp", ""),
    };

    let user = &alert.smtp_user;
    let pass = &alert.smtp_pass;
    let host = &alert.smtp_host;
    let port = alert.smtp_port;

    let smtp_url = format!("{scheme}://{user}:{pass}@{host}:{port}{tls_arg}");

    Ok(AsyncSmtpTransport::<Tokio1Executor>::from_url(&smtp_url)?.build())
}

pub async fn send(
    state: &AppState,
    subject: &str,
    body: String,
) -> std::result::Result<bool, anyhow::Error> {
    // this will log warnings if smtp not configured or mis-configured.
    check_alert_params();

    if let Some(alert) = state.load().config.alert_config() {
        let email = Message::builder()
            .from(alert.smtp_from_email.parse()?)
            .to(alert.admin_email.parse()?)
            .subject(subject)
            .body(body)?;

        // note: this error case is already checked/logged by check_alert_params.
        let mailer = gen_smtp_transport(&alert)?;

        // Attempt to send the email via the SMTP transport
        match mailer.send(email).await {
            Ok(_) => info!(
                "alert email sent successfully.  to: {}, subject: {subject}, smtp_host: {}:{}",
                alert.admin_email, alert.smtp_host, alert.smtp_port
            ),
            Err(e) => warn!(
                "error sending alert email. to: {}, subject: {subject}, smtp_host: {}:{}.  error: {:?}",
                alert.admin_email, alert.smtp_host, alert.smtp_port, e
            ),
        }

        Ok(true)
    } else {
        Ok(false)
    }
}
