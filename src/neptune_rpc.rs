use crate::alert_email;
use crate::model::app_state::AppState;
use crate::model::config::Config;
use anyhow::Context;
use chrono::DateTime;
use chrono::TimeDelta;
use chrono::Utc;
use clap::Parser;
use neptune_cash::rpc_server::RPCClient;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use tarpc::client;
use tarpc::context;
use tarpc::tokio_serde::formats::Json as RpcJson;
use tracing::{debug, info, warn};

/// generates RPCClient, for querying neptune-core RPC server.
pub async fn gen_rpc_client() -> Result<RPCClient, anyhow::Error> {
    // Create connection to neptune-core RPC server
    let args: Config = Config::parse();
    let server_socket = SocketAddr::new(
        std::net::IpAddr::V4(Ipv4Addr::LOCALHOST),
        args.neptune_rpc_port,
    );
    let transport = tarpc::serde_transport::tcp::connect(server_socket, RpcJson::default)
        .await
        .with_context(|| {
            format!("Failed to connect to neptune-core rpc service at {server_socket}")
        })?;
    Ok(RPCClient::new(client::Config::default(), transport).spawn())
}

/// a tokio task that periodically pings neptune-core rpc server to ensure the
/// connection is still alive and/or attempts to re-establish connection.
///
/// If not connected, a single connection attempt is made for each timer iteration.
///
/// Whenever the connection changes state a log message is printed and an email
/// alert is sent to admin, if admin_email config field is set.  In this way,
/// the site admin gets notified of both outages and restoration of service.
pub async fn watchdog(app_state: AppState) {
    let app_started = chrono::offset::Utc::now();
    let mut was_connected = true;
    let mut since = chrono::offset::Utc::now();
    let watchdog_secs = app_state.load().config.neptune_rpc_watchdog_secs;

    debug!("neptune-core rpc watchdog started");

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(watchdog_secs)).await;

        let result = app_state
            .load()
            .rpc_client
            .network(context::current())
            .await;

        let now_connected = result.is_ok();
        if now_connected != was_connected {
            // send admin alert of state change.
            let subject = match now_connected {
                true => "alert!  ** RECOVERY ** rpc connection restored",
                false => "alert!  ** OUTAGE ** rpc connection lost.",
            };

            let config = Config::parse();
            let now = chrono::offset::Utc::now();
            let duration = now.signed_duration_since(since);
            let app_duration = now.signed_duration_since(app_started);
            let body = NeptuneRpcAlertEmail {
                config,
                was_connected,
                now_connected,
                now,
                app_started,
                app_duration,
                since,
                duration,
            }
            .to_string();

            let msg = format!("alert: neptune-core rpc connection status change: previous: {was_connected}, now: {now_connected}");
            match now_connected {
                true => info!("{msg}"),
                false => warn!("{msg}"),
            }

            let _ = alert_email::send(&app_state, subject, body).await;

            was_connected = now_connected;
            since = chrono::offset::Utc::now();
        }

        if !now_connected {
            if let Ok(c) = gen_rpc_client().await {
                app_state.set_rpc_client(c);
            }
        }
    }
}

#[derive(boilerplate::Boilerplate)]
#[boilerplate(filename = "email/neptune_rpc_alert.txt")]
pub struct NeptuneRpcAlertEmail {
    config: Config,
    was_connected: bool,
    now_connected: bool,
    app_started: DateTime<Utc>,
    app_duration: TimeDelta,
    since: DateTime<Utc>,
    now: DateTime<Utc>,
    duration: TimeDelta,
}
