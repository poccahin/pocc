use std::error::Error;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::{accept_async, tungstenite::Message};

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum PlanetaryEvent {
    #[serde(rename = "TENSOR_INTERCEPTION")]
    TensorInterception {
        agent_id: String,
        gps: (f64, f64),
        diagnosis: String,
    },
    #[serde(rename = "LIFE_BURN")]
    LifeBurn {
        agent_id: String,
        gps: (f64, f64),
        amount_burned: u64,
    },
    #[serde(rename = "ROUTING_SUCCESS")]
    RoutingSuccess {
        agent_id: String,
        gps: (f64, f64),
        route: String,
    },
}

async fn run_websocket_broadcaster(mut rx: mpsc::Receiver<PlanetaryEvent>) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("0.0.0.0:9000").await?;
    println!("📡 [Koala OS] WebSocket Broadcaster listening on ws://0.0.0.0:9000");

    let (broadcast_tx, _) = broadcast::channel::<String>(256);

    let forward_tx = broadcast_tx.clone();
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&event) {
                let _ = forward_tx.send(json);
            }
        }
    });

    loop {
        let (stream, peer) = listener.accept().await?;
        let mut broadcast_rx = broadcast_tx.subscribe();

        tokio::spawn(async move {
            match accept_async(stream).await {
                Ok(ws_stream) => {
                    println!("🔌 frontend connected: {peer}");
                    let (mut write, _) = ws_stream.split();

                    while let Ok(msg) = broadcast_rx.recv().await {
                        if write.send(Message::Text(msg)).await.is_err() {
                            println!("🔌 frontend disconnected: {peer}");
                            break;
                        }
                    }
                }
                Err(err) => eprintln!("websocket handshake error: {err}"),
            }
        });
    }
}

async fn mock_tensor_firewall(event_tx: mpsc::Sender<PlanetaryEvent>) {
    let mut tick: u64 = 0;
    loop {
        tick += 1;
        tokio::time::sleep(Duration::from_secs(3)).await;

        let agent_id = format!("agent-{:04}", tick);
        let gps = (37.7749, -122.4194);

        if tick % 3 == 0 {
            println!("💀 [FATAL] Tensor Poisoning intercepted for {agent_id}.");
            let _ = event_tx
                .send(PlanetaryEvent::TensorInterception {
                    agent_id: agent_id.clone(),
                    gps,
                    diagnosis: "adversarial tensor poisoning signature".to_string(),
                })
                .await;

            let _ = event_tx
                .send(PlanetaryEvent::LifeBurn {
                    agent_id,
                    gps,
                    amount_burned: 100_000_000,
                })
                .await;
        } else {
            let _ = event_tx
                .send(PlanetaryEvent::RoutingSuccess {
                    agent_id,
                    gps: (51.5074, -0.1278),
                    route: "cr_plus_priority_lane".to_string(),
                })
                .await;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🪐 [Life++ OS] Booting AHIN L1 Daemon with Koala OS Uplink...");

    let (tx, rx) = mpsc::channel::<PlanetaryEvent>(256);

    tokio::spawn(async move {
        if let Err(err) = run_websocket_broadcaster(rx).await {
            eprintln!("broadcaster crashed: {err}");
        }
    });

    mock_tensor_firewall(tx).await;
    Ok(())
}
