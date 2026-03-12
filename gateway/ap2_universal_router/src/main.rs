//! AP2 Open-Source Framework - Universal Agent Gateway
//! Orchestrates ERC-8004, AP2, x402, and AHIN for full-chain CAI compliance.

use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use std::sync::Arc;

/// Maximum allowed size for `physical_payload` (4 KB).
const MAX_PAYLOAD_BYTES: usize = 4096;

/// Maximum requests per second accepted by the gateway (global).
const MAX_REQUESTS_PER_SEC: u32 = 50;

/// 标准化的 AP2 跨域代理请求
#[derive(Deserialize, Serialize, Debug)]
pub struct AP2AgentRequest {
    pub erc8004_did: String,      // 1. 身份：ERC-8004 注册的智能体 DID
    pub intent_schema: AP2Intent, // 2. 协作：AP2 意图描述符
    pub payment_auth: X402Token,  // 3. 价值：x402 支付授权 (兼容 Visa/Alipay/Crypto)
    pub physical_payload: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AP2Intent {
    pub action_type: String,
    pub target_domain: String,
    pub tensor_hash: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct X402Token {
    pub fiat_gateway: Option<String>, // e.g., "VISA", "Alipay", "PayPal"
    pub crypto_amount: Option<f64>,   // e.g., 50.0 LIFE++
    // 预授权签名是 x402 清算的必填字段，用于防止伪造支付请求。
    pub pre_auth_signature: String,
}

/// Configuration for the gateway orchestrator.
pub struct OrchestratorConfig {
    /// When `true`, the gateway rejects any request that does not arrive over
    /// an encrypted (TLS) channel.  Set to `false` only in test environments.
    pub require_tls: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self { require_tls: true }
    }
}

/// 网关核心引擎，统一编排四层协议
pub struct AP2FrameworkOrchestrator {
    erc_registry_client: Erc8004Client,
    payment_gateway: X402SettlementEngine,
    ahin_compliance_node: AhinAuditNode,
    /// Enforce TLS for every incoming request.
    require_tls: bool,
    /// Token-bucket rate limiter: rejects requests above MAX_REQUESTS_PER_SEC.
    rate_limiter: Arc<DefaultDirectRateLimiter>,
}

impl AP2FrameworkOrchestrator {
    pub fn new(config: OrchestratorConfig) -> Self {
        let quota = Quota::per_second(
            NonZeroU32::new(MAX_REQUESTS_PER_SEC)
                .unwrap(),
        );
        Self {
            erc_registry_client: Erc8004Client,
            payment_gateway: X402SettlementEngine,
            ahin_compliance_node: AhinAuditNode,
            require_tls: config.require_tls,
            rate_limiter: Arc::new(RateLimiter::direct(quota)),
        }
    }

    /// 处理一个完整生命周期请求
    pub async fn process_agent_request(
        &self,
        request: AP2AgentRequest,
        tls_context: bool,
    ) -> Result<String, &'static str> {
        // 0a. TRANSPORT SECURITY: reject requests that do not arrive over TLS.
        if self.require_tls && !tls_context {
            return Err("TLS_REQUIRED: Request rejected - unencrypted channel detected.");
        }

        // 0b. RATE LIMITING: token-bucket guard against Layer-7 DDoS.
        if self.rate_limiter.check().is_err() {
            return Err("RATE_LIMIT_EXCEEDED: Too many requests. Please retry later.");
        }

        // 0c. INPUT VALIDATION: hard cap on physical_payload size.
        if request.physical_payload.len() > MAX_PAYLOAD_BYTES {
            return Err("PAYLOAD_TOO_LARGE: physical_payload exceeds the 4 KB limit.");
        }

        println!(
            "🌐 [AP2 Gateway] Incoming request from Agent: {}",
            request.erc8004_did
        );

        // 1. TRUST LAYER (ERC-8004) : 身份与声誉校验
        let reputation_score = self
            .erc_registry_client
            .verify_identity(&request.erc8004_did)
            .await;
        if reputation_score < 60.0 {
            return Err("ERC-8004 REJECTED: Agent reputation too low or blacklisted.");
        }
        println!(
            "✅ [Trust] ERC-8004 Identity verified. Reputation: {}",
            reputation_score
        );

        // 2. COLLABORATION LAYER (AP2) : 意图解析
        if !self.validate_ap2_intent(&request.intent_schema) {
            return Err("AP2 REJECTED: Intent schema does not conform to AP2 standards.");
        }
        println!(
            "✅ [Collaboration] AP2 Intent parsed: {}",
            request.intent_schema.action_type
        );

        // 3. VALUE LAYER (x402) : 跨域支付清算
        let payment_cleared = self
            .payment_gateway
            .process_x402(&request.payment_auth)
            .await;
        if !payment_cleared {
            return Err(
                "x402 REJECTED: Payment Required. Insufficient funds in Visa/Alipay/PayPal or crypto wallet.",
            );
        }
        println!("✅ [Value] x402 Payment secured via authorized gateway.");

        // 4. COMPLIANCE LAYER (AHIN + VC) : 审计锚定
        let vc_audit_bundle = self
            .ahin_compliance_node
            .execute_and_anchor(&request.erc8004_did, request.physical_payload)
            .await;

        println!(
            "✅ [Compliance] Physical execution anchored. VC Audit Bundle: {}",
            vc_audit_bundle.hash
        );

        Ok(format!("SUCCESS_VC_ISSUED: {}", vc_audit_bundle.hash))
    }

    fn validate_ap2_intent(&self, intent: &AP2Intent) -> bool {
        !intent.action_type.is_empty()
            && !intent.target_domain.is_empty()
            && intent.tensor_hash.starts_with("0x")
    }
}

// 模拟底层客户端接口
struct Erc8004Client;
impl Erc8004Client {
    async fn verify_identity(&self, _did: &str) -> f64 {
        95.0
    }
}

struct X402SettlementEngine;
impl X402SettlementEngine {
    async fn process_x402(&self, auth: &X402Token) -> bool {
        let has_valid_signature = !auth.pre_auth_signature.trim().is_empty();
        let has_fiat_gateway = auth
            .fiat_gateway
            .as_ref()
            .is_some_and(|gateway| !gateway.trim().is_empty());
        let has_positive_crypto_amount = auth.crypto_amount.is_some_and(|amount| amount > 0.0);

        has_valid_signature && (has_fiat_gateway || has_positive_crypto_amount)
    }
}

struct AhinAuditNode;
impl AhinAuditNode {
    async fn execute_and_anchor(&self, did: &str, payload: Vec<u8>) -> VcBundle {
        VcBundle {
            hash: format!("0x{}{:X}", did.len(), payload.len()),
        }
    }
}

struct VcBundle {
    hash: String,
}

#[tokio::main]
async fn main() {
    let orchestrator = AP2FrameworkOrchestrator::new(OrchestratorConfig::default());

    let request = AP2AgentRequest {
        erc8004_did: "did:erc8004:lifeplusplus:agent-0001".into(),
        intent_schema: AP2Intent {
            action_type: "dispatch_physical_task".into(),
            target_domain: "energy_grid".into(),
            tensor_hash: "0xabc123tensor".into(),
        },
        payment_auth: X402Token {
            fiat_gateway: Some("VISA".into()),
            crypto_amount: Some(50.0),
            pre_auth_signature: "signed-preauth".into(),
        },
        physical_payload: vec![1, 2, 3, 4],
    };

    // In a real deployment this flag would come from the TLS handshake context.
    let is_tls = true;
    match orchestrator.process_agent_request(request, is_tls).await {
        Ok(msg) => println!("🚀 [Gateway] {msg}"),
        Err(err) => eprintln!("❌ [Gateway] {err}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_orchestrator(require_tls: bool) -> AP2FrameworkOrchestrator {
        AP2FrameworkOrchestrator::new(OrchestratorConfig { require_tls })
    }

    fn valid_request(payload: Vec<u8>) -> AP2AgentRequest {
        AP2AgentRequest {
            erc8004_did: "did:erc8004:test:agent".into(),
            intent_schema: AP2Intent {
                action_type: "dispatch_physical_task".into(),
                target_domain: "energy_grid".into(),
                tensor_hash: "0xabc123".into(),
            },
            payment_auth: X402Token {
                fiat_gateway: Some("VISA".into()),
                crypto_amount: Some(50.0),
                pre_auth_signature: "signed-preauth".into(),
            },
            physical_payload: payload,
        }
    }

    // ── Existing tests ────────────────────────────────────────────────

    #[test]
    fn validate_ap2_intent_rejects_invalid_tensor_hash() {
        let orchestrator = make_orchestrator(false);

        let invalid_intent = AP2Intent {
            action_type: "dispatch_physical_task".into(),
            target_domain: "energy_grid".into(),
            tensor_hash: "abc123".into(),
        };

        assert!(!orchestrator.validate_ap2_intent(&invalid_intent));
    }

    #[test]
    fn validate_ap2_intent_accepts_prefixed_tensor_hash() {
        let orchestrator = make_orchestrator(false);

        let valid_intent = AP2Intent {
            action_type: "dispatch_physical_task".into(),
            target_domain: "energy_grid".into(),
            tensor_hash: "0xabc123".into(),
        };

        assert!(orchestrator.validate_ap2_intent(&valid_intent));
    }

    #[tokio::test]
    async fn process_x402_requires_signature_and_payment_source() {
        let payment = X402SettlementEngine;

        let missing_signature = X402Token {
            fiat_gateway: Some("VISA".into()),
            crypto_amount: None,
            pre_auth_signature: "   ".into(),
        };
        assert!(!payment.process_x402(&missing_signature).await);

        let missing_payment_source = X402Token {
            fiat_gateway: Some(" ".into()),
            crypto_amount: Some(0.0),
            pre_auth_signature: "signed-preauth".into(),
        };
        assert!(!payment.process_x402(&missing_payment_source).await);

        let valid_crypto_payment = X402Token {
            fiat_gateway: None,
            crypto_amount: Some(0.01),
            pre_auth_signature: "signed-preauth".into(),
        };
        assert!(payment.process_x402(&valid_crypto_payment).await);
    }

    // ── New security-control tests ────────────────────────────────────

    /// Payloads within the 4 KB limit must pass the size guard.
    #[tokio::test]
    async fn payload_within_limit_is_accepted() {
        let orchestrator = make_orchestrator(false);
        let request = valid_request(vec![0u8; MAX_PAYLOAD_BYTES]);
        let result = orchestrator.process_agent_request(request, true).await;
        assert!(result.is_ok(), "expected Ok but got: {:?}", result);
    }

    /// Payloads exceeding 4 KB must be rejected with PAYLOAD_TOO_LARGE.
    #[tokio::test]
    async fn payload_exceeding_limit_is_rejected() {
        let orchestrator = make_orchestrator(false);
        let request = valid_request(vec![0u8; MAX_PAYLOAD_BYTES + 1]);
        let result = orchestrator.process_agent_request(request, true).await;
        assert_eq!(
            result,
            Err("PAYLOAD_TOO_LARGE: physical_payload exceeds the 4 KB limit.")
        );
    }

    /// Requests over an unencrypted channel must be rejected when require_tls is true.
    #[tokio::test]
    async fn unencrypted_request_rejected_when_tls_required() {
        let orchestrator = make_orchestrator(true);
        let request = valid_request(vec![1, 2, 3]);
        let result = orchestrator.process_agent_request(request, false).await;
        assert_eq!(
            result,
            Err("TLS_REQUIRED: Request rejected - unencrypted channel detected.")
        );
    }

    /// Requests over an unencrypted channel are allowed when require_tls is false.
    #[tokio::test]
    async fn unencrypted_request_allowed_when_tls_not_required() {
        let orchestrator = make_orchestrator(false);
        let request = valid_request(vec![1, 2, 3]);
        let result = orchestrator.process_agent_request(request, false).await;
        assert!(result.is_ok(), "expected Ok but got: {:?}", result);
    }

    /// After exhausting the rate-limiter quota, subsequent requests must be
    /// rejected with RATE_LIMIT_EXCEEDED.
    #[tokio::test]
    async fn rate_limiter_rejects_overflow_requests() {
        // Use a quota of 1 req/s so we can exhaust it immediately.
        let quota = Quota::per_second(NonZeroU32::new(1).unwrap());
        let orchestrator = AP2FrameworkOrchestrator {
            erc_registry_client: Erc8004Client,
            payment_gateway: X402SettlementEngine,
            ahin_compliance_node: AhinAuditNode,
            require_tls: false,
            rate_limiter: Arc::new(RateLimiter::direct(quota)),
        };

        // First request consumes the single available token — should succeed.
        let r1 = orchestrator
            .process_agent_request(valid_request(vec![1]), true)
            .await;
        assert!(r1.is_ok(), "first request should pass: {:?}", r1);

        // Second request in the same second must be rate-limited.
        let r2 = orchestrator
            .process_agent_request(valid_request(vec![1]), true)
            .await;
        assert_eq!(
            r2,
            Err("RATE_LIMIT_EXCEEDED: Too many requests. Please retry later.")
        );
    }
}
