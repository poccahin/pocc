use futures_util::SinkExt;
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::{accept_async, tungstenite::Message};

/// Start a websocket telemetry pump that forwards broadcast messages to clients.
pub async fn start_telemetry_pump(rx: broadcast::Receiver<String>) -> std::io::Result<()> {
    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("📡 [TELEMETRY] Koala OS WebSocket Stream active on ws://{addr}");

    loop {
        let (stream, _) = listener.accept().await?;
        let mut local_rx = rx.resubscribe();

        tokio::spawn(async move {
            let Ok(mut ws_stream) = accept_async(stream).await else {
                return;
            };

            while let Ok(msg) = local_rx.recv().await {
                if ws_stream.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }
        });
    }
}

pub fn emit_collapse_event(
    tx: &broadcast::Sender<String>,
    worker_did: &str,
    friction: f32,
    amount: f64,
    tx_hash: &str,
) {
    let event = json!({
        "type": "GRAVITATIONAL_COLLAPSE",
        "source": "ORCHESTRATOR_GENESIS",
        "target": worker_did,
        "friction_f": friction,
        "x402_volume_usd": amount,
        "merkle_leaf_hash": tx_hash,
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0),
    });

    let _ = tx.send(event.to_string());
}

pub fn emit_crystallization_event(
    tx: &broadcast::Sender<String>,
    root_hash: &str,
    total_ctx: usize,
) {
    let event = json!({
        "type": "MERKLE_TREE_CRYSTALLIZATION",
        "genesis_root": root_hash,
        "folded_transactions": total_ctx,
        "status": "READY_FOR_L1_SETTLEMENT",
    });

    let _ = tx.send(event.to_string());
}
