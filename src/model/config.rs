#[readonly::make]
#[derive(Debug, clap::Parser, Clone)]
#[clap(name = "neptune-explorer", about = "Neptune Block Explorer")]
pub struct Config {
    /// Sets the server address to connect to.
    #[clap(long, default_value = "9799", value_name = "PORT")]
    pub port: u16,
}
