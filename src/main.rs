use anyhow::Result;
use dotenv::dotenv;
use env_logger::Env;
use futures_util::{SinkExt, StreamExt};
use hardware::Hardware;
use log::{error, info, warn};
use std::{env, net::SocketAddr, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    task, time,
};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

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

    let server = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))).await?;

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

        task::spawn(handle_connection(websocket));
    }
}

async fn handle_connection(websocket: WebSocketStream<TcpStream>) -> Result<()> {
    let (mut write, _) = websocket.split();
    let mut hardware = Hardware::new();
    let interval = Duration::from_millis(
        env::var("INTERVAL")
            .unwrap_or_else(|_| "1000".to_string())
            .parse()?,
    );
    hardware.refresh().await?;

    loop {
        let hardware_info = hardware.get().await?;
        let json = serde_json::to_string(&hardware_info)?;
        write.send(Message::Text(json.into())).await?;
        time::sleep(interval).await;
    }
}
