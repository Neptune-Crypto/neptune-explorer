// Config and AlertConfig seem more complex than they should be.
// The requirement is that alert params are all-or-none, but
// Clap makes it non-obvious / difficult to express that.
//
// There is a discussion about it:
// https://github.com/clap-rs/clap/discussions/5506
//
// Hopefully we can simplify it soon / eventually.

#[readonly::make]
#[derive(Debug, clap::Parser, Clone)]
#[clap(name = "neptune-explorer", about = "Neptune Block Explorer")]
#[clap(group(
    clap::ArgGroup::new("value")
        .required(false)
        .multiple(true)
        .requires_all(&["admin_email", "smtp_host", "smtp_port", "smtp_user", "smtp_pass", "smtp_from_email"])
        .args(&["admin_email", "smtp_host", "smtp_port", "smtp_user", "smtp_pass", "smtp_from_email"])
))]
pub struct Config {
    /// Sets the website name
    #[clap(long, default_value = "Neptune Explorer", value_name = "site-name")]
    pub site_name: String,

    /// Sets the website domain, eg 'explorer.mydomain.com'. used for alert emails, etc.
    #[clap(long, value_name = "domain")]
    pub site_domain: String,

    /// Sets the port to listen for http requests.
    #[clap(long, default_value = "3000", value_name = "port")]
    pub listen_port: u16,

    /// Sets the neptune-core rpc server address to connect to.
    #[clap(long, default_value = "9799", value_name = "port")]
    pub neptune_rpc_port: u16,

    /// Sets interval in seconds to ping neptune-core rpc connection
    #[clap(long, default_value = "10", value_name = "seconds")]
    pub neptune_rpc_watchdog_secs: u64,

    /// admin email for receiving alert emails
    #[arg(long, value_name = "email")]
    pub admin_email: Option<String>,

    /// smtp host for alert emails
    #[arg(long, value_name = "host")]
    pub smtp_host: Option<String>,

    /// smtp port for alert emails
    #[arg(long, value_name = "port", default_value = "25")]
    pub smtp_port: Option<u16>,

    /// smtp username for alert emails
    #[arg(long, value_name = "user")]
    pub smtp_user: Option<String>,

    /// smtp password for alert emails
    #[arg(long, value_name = "pass")]
    pub smtp_pass: Option<String>,

    /// sender email for alerts
    #[arg(long, value_name = "email")]
    pub smtp_from_email: Option<String>,

    /// connect with smtps or starttls
    #[arg(long, value_enum, value_name = "mode", default_value = "smtps")]
    pub smtp_mode: SmtpMode,
}

impl Config {
    pub fn alert_config(&self) -> Option<AlertConfig> {
        match (
            &self.admin_email,
            &self.smtp_host,
            &self.smtp_port,
            &self.smtp_user,
            &self.smtp_pass,
            &self.smtp_from_email,
            &self.smtp_mode,
        ) {
            (
                Some(admin_email),
                Some(smtp_host),
                Some(smtp_port),
                Some(smtp_user),
                Some(smtp_pass),
                Some(smtp_from_email),
                smtp_mode,
            ) => Some(AlertConfig {
                admin_email: admin_email.clone(),
                smtp_host: smtp_host.clone(),
                smtp_port: *smtp_port,
                smtp_user: smtp_user.clone(),
                smtp_pass: smtp_pass.clone(),
                smtp_from_email: smtp_from_email.clone(),
                smtp_mode: smtp_mode.clone(),
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, clap::Args)]
pub struct AlertConfig {
    /// admin email for receiving alert emails
    pub admin_email: String,

    /// smtp host for alert emails
    pub smtp_host: String,

    /// smtp host for alert emails
    pub smtp_port: u16,

    /// smtp username for alert emails
    pub smtp_user: String,

    /// smtp password for alert emails
    pub smtp_pass: String,

    /// sender email for alerts
    pub smtp_from_email: String,

    /// connect with smtps or starttls
    pub smtp_mode: SmtpMode,
}

#[derive(Debug, Clone, clap::ValueEnum)]
/// Determines SMTP encryption mode.
/// See: https://docs.rs/lettre/0.11.7/lettre/transport/smtp/struct.AsyncSmtpTransport.html#method.from_url
pub enum SmtpMode {
    /// smtps
    Smtps,

    /// starttls required
    Starttls,

    /// use starttls if available.  insecure.
    Opportunistic,

    /// plain text.  insecure.
    Plaintext,
}
