mod caas_registry;
mod eclipse_monitor;
mod gravity_engine;
mod grpc_python;
mod siwa;
mod solana_ledger;
mod tee_enclave;
mod universal_orchestrator;

use std::sync::Arc;

use anyhow::Context;
use caas_registry::{CaaSRegistry, McpServiceRegistration};
use eclipse_monitor::{ActionClass, CrossDomainAnchor};
use gravity_engine::{AgentNodeContext, GravityTensor};
use serde::{Deserialize, Serialize};
use siwa::{SiwaClaims, SiwaVerifier};
use solana_ledger::ThermodynamicLedger;
use std::time::{SystemTime, UNIX_EPOCH};
use tee_enclave::{TeeAttestation, TeeEnvelope};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[derive(Debug, Deserialize)]
struct RoutingIntent {
    agent_id: String,
    target_semantic_distance: f64,
    siwa_nonce: String,
    siwa_signature: String,
    siwa_issued_at_unix: u64,
    tee_measurement: String,
    tee_quote_hash: String,
    register_caas: Option<McpRegistrationIntent>,
}

#[derive(Debug, Deserialize)]
struct McpRegistrationIntent {
    service_id: String,
    mcp_endpoint: String,
    metadata_cid: String,
}

#[derive(Debug, Serialize)]
struct RoutingDecision {
    accepted: bool,
    message: String,
    selected_node: Option<String>,
}

#[derive(Clone)]
struct AppSecurity {
    siwa_verifier: SiwaVerifier,
    tee_envelope: TeeEnvelope,
    caas_registry: CaaSRegistry,
}

impl AppSecurity {
    fn new() -> Self {
        Self {
            siwa_verifier: SiwaVerifier::new(300),
            tee_envelope: TeeEnvelope::new("LIFEPP_EDGE_TEE_V1"),
            caas_registry: CaaSRegistry::new(),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🪐 [Life++ OS] Booting AHIN L1 Daemon with L3 Economic Settlement...");

    let mock_secret = [1u8; 64];
    let ledger = Arc::new(ThermodynamicLedger::new(
        "https://api.devnet.solana.com",
        "7YdwpERJjzw7UVojxLpvu5ycKBRdYaxaKn4HvoHLpump",
        &mock_secret,
    )?);

    let router = Arc::new(GravityTensor::new(1.5, 1.0, 2.0));
    let app_security = Arc::new(AppSecurity::new());
    let cross_domain_anchor = Arc::new(CrossDomainAnchor::new(vec![
        "https://anchor-1.lifeplusplus.global".to_string(),
        "https://anchor-2.lifeplusplus.global".to_string(),
        "sat://leo-ahin-time-signer".to_string(),
    ]));
    cross_domain_anchor.spawn_watchdog().await;

    let listener = TcpListener::bind("0.0.0.0:8000")
        .await
        .context("failed to bind AHIN daemon on :8000")?;

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("📡 [AHIN] Accepted intent stream from {addr}");
        let router_clone = Arc::clone(&router);
        let ledger_clone = Arc::clone(&ledger);
        let app_security_clone = Arc::clone(&app_security);
        let cross_domain_anchor_clone = Arc::clone(&cross_domain_anchor);

        tokio::spawn(async move {
            if let Err(e) = handle_connection(
                socket,
                router_clone,
                ledger_clone,
                app_security_clone,
                cross_domain_anchor_clone,
            )
            .await
            {
                eprintln!("⚠️ [Network Error] {e}");
            }
        });
    }
}

async fn handle_connection(
    mut socket: TcpStream,
    router: Arc<GravityTensor>,
    ledger: Arc<ThermodynamicLedger>,
    app_security: Arc<AppSecurity>,
    cross_domain_anchor: Arc<CrossDomainAnchor>,
) -> anyhow::Result<()> {
    let mut buf = vec![0u8; 4096];
    let n = socket.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }

    let intent: RoutingIntent =
        serde_json::from_slice(&buf[..n]).context("invalid intent payload")?;

    let now_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_secs();

    let siwa_claims = SiwaClaims {
        agent_id: intent.agent_id.clone(),
        issued_at_unix: intent.siwa_issued_at_unix,
        nonce: intent.siwa_nonce.clone(),
        signature: intent.siwa_signature.clone(),
    };

    if let Err(reason) = app_security.siwa_verifier.verify(&siwa_claims, now_unix) {
        let response = RoutingDecision {
            accepted: false,
            message: reason.to_string(),
            selected_node: None,
        };
        let payload = serde_json::to_vec(&response)?;
        socket.write_all(&payload).await?;
        return Ok(());
    }

    let attestation = TeeAttestation {
        measurement: intent.tee_measurement.clone(),
        quote_hash: intent.tee_quote_hash.clone(),
    };

    if let Err(reason) = app_security.tee_envelope.verify_attestation(&attestation) {
        let response = RoutingDecision {
            accepted: false,
            message: reason.to_string(),
            selected_node: None,
        };
        let payload = serde_json::to_vec(&response)?;
        socket.write_all(&payload).await?;
        return Ok(());
    }

    let is_robust = !intent.agent_id.contains("poison");

    let action_class = determine_action_class(&intent);
    if let Err(reason) = cross_domain_anchor.validate_intent_execution(action_class) {
        println!("🚫 Execution Denied: {reason}");
        let response = RoutingDecision {
            accepted: false,
            message:
                "Node is in an eclipsed geographic subnet. Only survival operations are permitted."
                    .to_string(),
            selected_node: None,
        };

        let payload = serde_json::to_vec(&response)?;
        socket.write_all(&payload).await?;
        return Ok(());
    }

    let decision = if !is_robust {
        println!(
            "💀 [FATAL] Tensor Poisoning intercepted for {}",
            intent.agent_id
        );
        println!("🔥 Initiating Solana Slashing Protocol...");

        if let Err(e) = ledger.execute_slashing_burn(&intent.agent_id, 100_000_000) {
            eprintln!("⚠️ [SLASHING ERROR] {e}");
        }

        RoutingDecision {
            accepted: false,
            message: "Rejected: tensor poisoning detected; slashing attempted".to_string(),
            selected_node: None,
        }
    } else {
        println!("✅ [PoCC] Tensor robust. Verifying On-Chain Stakes...");

        let mut best_node = None;
        let mut max_gravity = f64::NEG_INFINITY;
        let mock_candidates = [
            "8QBv56rD6BYpcE6vR5f8b4j3M2M9gXTXmW2iRkoP5Ayt",
            "2Z8Uo6z4xMuRAkBn8pEGD9Y5CnQiiHecfNf6t7MvmUgQ",
        ];

        for node_account in mock_candidates {
            let real_stake = ledger.get_staked_balance(node_account);
            let temp_node = AgentNodeContext {
                node_id: [0u8; 32],
                entropy_reduction_joules: 5000.0,
                life_plus_staked: real_stake,
                topological_entropy: 0.5,
            };

            let gravity =
                router.compute_gravity_pull(&temp_node, intent.target_semantic_distance)?;
            if gravity > max_gravity {
                max_gravity = gravity;
                best_node = Some(node_account.to_string());
            }
        }

        if let Some(registration) = intent.register_caas.as_ref() {
            let tx = app_security
                .caas_registry
                .register_service(McpServiceRegistration {
                    service_id: registration.service_id.clone(),
                    mcp_endpoint: registration.mcp_endpoint.clone(),
                    metadata_cid: registration.metadata_cid.clone(),
                    registrant_agent_id: intent.agent_id.clone(),
                });

            match tx {
                Ok(tx_hash) => {
                    println!("🧭 [ERC-8004 MCP Registry] CaaS MCP advertised with tx {tx_hash}")
                }
                Err(reason) => eprintln!("⚠️ [MCP Registry Reject] {reason}"),
            }
        }

        let sealed_routing_digest = app_security
            .tee_envelope
            .sign_inside_enclave(format!("{}:{:.4}", intent.agent_id, max_gravity).as_bytes());

        RoutingDecision {
            accepted: true,
            message: format!(
                "Route accepted with CR+ gravity score {:.4}; TEE routing digest {}",
                max_gravity,
                &sealed_routing_digest[..16]
            ),
            selected_node: best_node,
        }
    };

    let payload = serde_json::to_vec(&decision)?;
    socket.write_all(&payload).await?;
    Ok(())
}

fn determine_action_class(intent: &RoutingIntent) -> ActionClass {
    let normalized = intent.agent_id.to_ascii_lowercase();
    if normalized.contains("rescue")
        || normalized.contains("survival")
        || normalized.contains("evac")
        || normalized.contains("medic")
    {
        ActionClass::SurvivalKinematic
    } else {
        ActionClass::EconomicThermodynamic
    }
}
