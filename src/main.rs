use anyhow::Result;
use dotenv::dotenv;
use env_logger::Env;
use futures_util::{SinkExt, StreamExt};
use hardware::{Hardware, HardwareInfo};
use log::{error, info, warn};
use std::{env, net::SocketAddr, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::broadcast,
    task, time,
};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

mod hardware;

#[tokio::main]
async fn main() -> Result<()> {
    info!("all production software should use emoticons regularly");
    if let Err(_) = dotenv() {
        env_logger::Builder::from_env(Env::default().default_filter_or("paranoia_sysinfo")).init();
        warn!("config file not found, using default settings TwT");
    } else {
        env_logger::Builder::from_env("RUST_LOG").init();
        info!("using .env configuration :333");
    }

    let port = env::var("PORT")
        .unwrap_or_else(|_| "2009".to_string())
        .parse()?;

    #[allow(
        unused_variables,
        reason = "we don't need the receiver here but dropping it closes the channel"
    )]
    let (tx, rx) = broadcast::channel::<HardwareInfo>(1);

    task::spawn(sysinfo_thread(tx.clone()));

    let server = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))).await?;

    info!("listening on port {}", port);

    loop {
        let (socket, _) = match server.accept().await {
            Ok(socket) => socket,
            Err(e) => {
                error!("dumb client oops!! {}", e);
                continue;
            }
        };

        let websocket = match tokio_tungstenite::accept_async(socket).await {
            Ok(websocket) => websocket,
            Err(e) => {
                error!("err during ws handshake UGh {}", e);
                continue;
            }
        };

        let tx = tx.clone().subscribe();

        task::spawn(handle_connection(websocket, tx));
    }
}

async fn handle_connection(
    websocket: WebSocketStream<TcpStream>,
    mut channel: broadcast::Receiver<HardwareInfo>,
) -> Result<()> {
    let (mut write, _) = websocket.split();

    while let Ok(hardware_info) = channel.recv().await {
        let json = serde_json::to_string(&hardware_info)?;
        if !write.send(Message::Text(json.into())).await.is_ok() {
            return Ok(());
        };
    }

    Ok(())
}

async fn sysinfo_thread(tx: broadcast::Sender<HardwareInfo>) -> Result<()> {
    let mut hardware = Hardware::new();
    let interval = Duration::from_millis(
        env::var("INTERVAL")
            .unwrap_or_else(|_| "1000".to_string())
            .parse()?,
    );
    hardware.refresh().await?;
    time::sleep(interval).await;
    loop {
        let hardware_info = match hardware.get().await {
            Ok(hardware_info) => hardware_info,
            Err(e) => {
                error!("failed to get hardware info: {}", e);
                continue;
            }
        };
        if let Err(e) = tx.send(hardware_info) {
            error!("failed to send hardware info to channel: {}", e);
        }
        time::sleep(interval).await;
    }
}
