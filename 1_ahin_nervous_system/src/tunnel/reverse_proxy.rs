use std::convert::Infallible;
use std::sync::Arc;

use hyper::body::to_bytes;
use hyper::{Body, Request, Response, StatusCode};
use libp2p::swarm::Stream;
use libp2p::{PeerId, StreamProtocol, Swarm};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

use crate::AhinBehaviour;

/// 定义专属的底层 P2P 隧道协议
pub const AHIN_TUNNEL_PROTOCOL: StreamProtocol = StreamProtocol::new("/ahin/tunnel/1.0.0");

pub struct GatewayRouter {
    pub swarm: Arc<Mutex<Swarm<AhinBehaviour>>>,
}

impl GatewayRouter {
    /// 拦截 Web2 流量，执行空间折叠，将其塞入 P2P 隧道。
    pub async fn handle_web2_request(
        req: Request<Body>,
        swarm: Arc<Mutex<Swarm<AhinBehaviour>>>,
    ) -> Result<Response<Body>, Infallible> {
        let host_header = req
            .headers()
            .get("host")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();

        // 1. 从 Host 中提取机器人的 DID
        let did_prefix = match host_header.strip_suffix(".ahin.io") {
            Some(prefix) if !prefix.is_empty() => prefix,
            _ => {
                return Ok(Self::simple_response(
                    StatusCode::BAD_REQUEST,
                    "Invalid AHIN Domain",
                ))
            }
        };

        println!(
            "🔍 [GATEWAY] Intercepted Web2 request for Entity: {}",
            did_prefix
        );

        // 2. 通过 DHT/索引层寻址 DID 对应 PeerId
        let target_peer_id = match resolve_did_to_peer_id(did_prefix, swarm.clone()).await {
            Some(peer_id) => peer_id,
            None => {
                return Ok(Self::simple_response(
                    StatusCode::NOT_FOUND,
                    "Entity Offline or Slain",
                ))
            }
        };

        println!(
            "⚡ [TUNNEL] Target located (PeerId: {}). Opening sub-space stream...",
            target_peer_id
        );

        // 3. 向 NAT 后设备发起多路复用 Stream 请求
        let mut p2p_stream = match open_tunnel_stream(swarm.clone(), target_peer_id).await {
            Ok(stream) => stream,
            Err(_) => {
                return Ok(Self::simple_response(
                    StatusCode::GATEWAY_TIMEOUT,
                    "P2P Tunnel Creation Failed",
                ))
            }
        };

        // 4. 序列化 HTTP Request 为字节流并写入隧道
        if let Err(e) = write_http_request_into_tunnel(req, &mut p2p_stream).await {
            println!("❌ [TUNNEL] Failed to push request into tunnel: {e:?}");
            return Ok(Self::simple_response(
                StatusCode::BAD_GATEWAY,
                "Failed to write request into tunnel",
            ));
        }

        // 5. 读取边缘设备回传字节并返回 Web2 客户端
        let mut response_buf = Vec::new();
        match p2p_stream.read_to_end(&mut response_buf).await {
            Ok(_) => {
                println!(
                    "✅ [TUNNEL] Response received from physical entity. Relaying to Web2 client."
                );
                Ok(Response::new(Body::from(response_buf)))
            }
            Err(e) => {
                println!("❌ [TUNNEL] Failed to read tunnel response: {e:?}");
                Ok(Self::simple_response(
                    StatusCode::BAD_GATEWAY,
                    "Failed to read response from tunnel",
                ))
            }
        }
    }

    fn simple_response(status: StatusCode, message: &str) -> Response<Body> {
        Response::builder()
            .status(status)
            .body(Body::from(message.to_owned()))
            .unwrap_or_else(|_| Response::new(Body::from(message.to_owned())))
    }
}

async fn open_tunnel_stream(
    swarm: Arc<Mutex<Swarm<AhinBehaviour>>>,
    peer_id: PeerId,
) -> Result<Stream, String> {
    swarm
        .lock()
        .await
        .behaviour_mut()
        .request_stream(peer_id, AHIN_TUNNEL_PROTOCOL)
        .map_err(|e| format!("{e:?}"))
}

async fn write_http_request_into_tunnel(
    req: Request<Body>,
    stream: &mut Stream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (parts, body) = req.into_parts();
    let body_bytes = to_bytes(body).await?;

    // 这里是轻量序列化示例，生产环境可切换为完整 HTTP codec。
    let request_line = format!("{} {} HTTP/1.1\r\n", parts.method, parts.uri);
    stream.write_all(request_line.as_bytes()).await?;

    for (header, value) in parts.headers.iter() {
        stream
            .write_all(format!("{}: {}\r\n", header, value.to_str().unwrap_or_default()).as_bytes())
            .await?;
    }

    stream
        .write_all(format!("content-length: {}\r\n\r\n", body_bytes.len()).as_bytes())
        .await?;

    if !body_bytes.is_empty() {
        stream.write_all(&body_bytes).await?;
    }

    stream.flush().await?;
    Ok(())
}

/// 从 DID 前缀解析到目标 PeerId。
///
/// 当前是接入点，具体逻辑可由 Kademlia DHT 或链上索引实现。
async fn resolve_did_to_peer_id(
    _did_prefix: &str,
    _swarm: Arc<Mutex<Swarm<AhinBehaviour>>>,
) -> Option<PeerId> {
    None
}
