use std::{fs, path::Path};

use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};

use crate::universal_orchestrator::{PoCCTensorClient, TensorCheck, WorkflowError};

/// mTLS-protected connector for the Python PoCC tensor firewall.
pub struct SecureGrpcPythonClient {
    channel: Channel,
}

impl SecureGrpcPythonClient {
    /// Establish an mTLS channel to the Python wind tunnel endpoint.
    ///
    /// `endpoint` must be an `https://` URI and `domain_name` must match the
    /// server certificate SAN/CN.
    pub async fn connect_secure_wind_tunnel(
        endpoint: &str,
        domain_name: &str,
        cert_dir: impl AsRef<Path>,
    ) -> Result<Self, WorkflowError> {
        println!("🔒 [SYS] Initiating mTLS Handshake with Tensor Wind Tunnel...");

        let cert_dir = cert_dir.as_ref();
        let ca_cert_pem = fs::read(cert_dir.join("ca.crt"))
            .map_err(|e| WorkflowError::TensorLayer(format!("failed to read ca.crt: {e}")))?;
        let client_cert_pem = fs::read(cert_dir.join("client.crt")).map_err(|e| {
            WorkflowError::TensorLayer(format!("failed to read client.crt: {e}"))
        })?;
        let client_key_pem = fs::read(cert_dir.join("client.key"))
            .map_err(|e| WorkflowError::TensorLayer(format!("failed to read client.key: {e}")))?;

        let tls_config = ClientTlsConfig::new()
            .domain_name(domain_name.to_string())
            .ca_certificate(Certificate::from_pem(ca_cert_pem))
            .identity(Identity::from_pem(client_cert_pem, client_key_pem));

        let channel = Channel::from_shared(endpoint.to_string())
            .map_err(|e| WorkflowError::TensorLayer(format!("invalid endpoint URI: {e}")))?
            .tls_config(tls_config)
            .map_err(|e| WorkflowError::TensorLayer(format!("tls config failure: {e}")))?
            .connect()
            .await
            .map_err(|e| WorkflowError::TensorLayer(format!("mTLS handshake failed: {e}")))?;

        println!("✅ [SYS] mTLS Tunnel Established. Symmetric encryption keys locked.");

        Ok(Self { channel })
    }

    pub fn channel(&self) -> Channel {
        self.channel.clone()
    }
}

#[tonic::async_trait]
impl PoCCTensorClient for SecureGrpcPythonClient {
    async fn check_semantic_drift(&self, _tensor: &[f32]) -> Result<TensorCheck, WorkflowError> {
        Err(WorkflowError::TensorLayer(
            "gRPC method binding not configured: wire generated TensorFirewall client on top of SecureGrpcPythonClient::channel()".to_string(),
        ))
    }
}
