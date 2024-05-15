#[readonly::make]
#[derive(Debug, clap::Parser, Clone)]
#[clap(name = "neptune-explorer", about = "Neptune Block Explorer")]
pub struct Config {
    /// Sets the website name
    #[clap(long, default_value = "Neptune Explorer", value_name = "site-name")]
    pub site_name: String,

    /// Sets the server address to connect to.
    #[clap(long, default_value = "9799", value_name = "PORT")]
    pub port: u16,
}
