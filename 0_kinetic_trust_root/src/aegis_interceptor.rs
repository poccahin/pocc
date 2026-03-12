use axum::{
    body::to_bytes,
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

/// 预置 Owner 公钥（32-byte Ed25519）。
/// 正式环境应在 bootstrap 期间以安全方式注入，而不是硬编码。
const AUTHORIZED_OWNER_PUBKEY: [u8; 32] = [0u8; 32];

const HEADER_SIGNATURE: &str = "X-LifePlus-Signature";
const HEADER_MESSAGE_HASH: &str = "X-LifePlus-Message-Hash";
const HEADER_ERC8004_PASSPORT: &str = "X-ERC8004-Passport";
const HEADER_AP2_INTENT_CID: &str = "X-AP2-Intent-CID";
const HEADER_X402_NONCE: &str = "X-x402-Nonce";

/// 宙斯盾四维拦截中间件 (The Aegis Interceptor)
///
/// 流程：
/// 1) 以 Ed25519 校验 owner 身份签名；
/// 2) 校验 ERC-8004 entitlement；
/// 3) 计算 AP2/PoCC 语义摩擦；
/// 4) 校验 x402 nonce 防重放。
pub async fn strict_intent_firewall(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let signature_bytes = decode_header_hex(&headers, HEADER_SIGNATURE, StatusCode::UNAUTHORIZED)?;
    let msg_hash_bytes =
        decode_header_hex(&headers, HEADER_MESSAGE_HASH, StatusCode::UNAUTHORIZED)?;

    if msg_hash_bytes.len() != 32 {
        return Err(StatusCode::BAD_REQUEST);
    }

    verify_owner_signature(&signature_bytes, &msg_hash_bytes)?;

    // 额外安全强化：确认 header 里的 hash 与真实请求体一致，杜绝签名与 payload 脱钩。
    let (parts, body) = request.into_parts();
    let body_bytes = to_bytes(body, 1024 * 1024)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let computed_hash = Sha256::digest(&body_bytes);
    if computed_hash.as_slice() != msg_hash_bytes.as_slice() {
        return Err(StatusCode::FORBIDDEN);
    }
    let request = Request::from_parts(parts, body_bytes.into());

    let erc8004_passport =
        header_to_str(&headers, HEADER_ERC8004_PASSPORT, StatusCode::UNAUTHORIZED)?;
    if !verify_erc8004_entitlement(erc8004_passport) {
        return Err(StatusCode::PAYMENT_REQUIRED);
    }

    let intent_vector_cid =
        header_to_str(&headers, HEADER_AP2_INTENT_CID, StatusCode::BAD_REQUEST)?;
    let friction = calculate_pocc_friction(intent_vector_cid);
    if friction > 0.05 {
        return Err(StatusCode::NOT_ACCEPTABLE);
    }

    let x402_nonce = header_to_str(&headers, HEADER_X402_NONCE, StatusCode::BAD_REQUEST)?;
    if !validate_x402_channel_nonce(x402_nonce) {
        return Err(StatusCode::CONFLICT);
    }

    Ok(next.run(request).await)
}

fn verify_owner_signature(signature_bytes: &[u8], msg_hash_bytes: &[u8]) -> Result<(), StatusCode> {
    let verifying_key =
        VerifyingKey::from_bytes(&AUTHORIZED_OWNER_PUBKEY).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let signature = Signature::from_slice(signature_bytes).map_err(|_| StatusCode::UNAUTHORIZED)?;

    verifying_key
        .verify_strict(msg_hash_bytes, &signature)
        .map_err(|_| StatusCode::FORBIDDEN)
}

fn decode_header_hex(
    headers: &HeaderMap,
    key: &'static str,
    missing_status: StatusCode,
) -> Result<Vec<u8>, StatusCode> {
    let value = header_to_str(headers, key, missing_status)?;
    hex::decode(value).map_err(|_| StatusCode::BAD_REQUEST)
}

fn header_to_str<'a>(
    headers: &'a HeaderMap,
    key: &'static str,
    missing_status: StatusCode,
) -> Result<&'a str, StatusCode> {
    headers
        .get(key)
        .ok_or(missing_status)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)
}

// ---- 辅助函数（生产中应替换为真实实现） ----
fn verify_erc8004_entitlement(passport: &str) -> bool {
    !passport.trim().is_empty()
}

fn calculate_pocc_friction(cid: &str) -> f64 {
    if cid.trim().is_empty() {
        1.0
    } else {
        0.01
    }
}

fn validate_x402_channel_nonce(nonce: &str) -> bool {
    nonce.parse::<u64>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn entitlement_rejects_empty_passport() {
        assert!(!verify_erc8004_entitlement(""));
        assert!(verify_erc8004_entitlement("did:lifeplus:owner#passport"));
    }

    #[test]
    fn nonce_requires_u64() {
        assert!(validate_x402_channel_nonce("42"));
        assert!(!validate_x402_channel_nonce("4.2"));
        assert!(!validate_x402_channel_nonce("NaN"));
    }

    #[test]
    fn header_decode_hex_handles_invalid_values() {
        let mut headers = HeaderMap::new();
        headers.insert(HEADER_SIGNATURE, HeaderValue::from_static("zz"));

        let result = decode_header_hex(&headers, HEADER_SIGNATURE, StatusCode::UNAUTHORIZED);
        assert_eq!(result, Err(StatusCode::BAD_REQUEST));
    }
}
