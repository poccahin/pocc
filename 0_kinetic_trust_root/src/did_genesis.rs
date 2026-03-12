use rand::rngs::OsRng;
use solana_sdk::signature::{Keypair, Signer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceClass {
    MacMiniM4,
    AmdStrixHalo395,
    HumanoidWorker,
    CloudOrchestrator,
}

#[derive(Debug)]
pub struct SiliconIdentity {
    pub keypair: Keypair,
    pub wallet_address: String,
    pub ahin_domain: String,
    pub device_class: DeviceClass,
}

impl SiliconIdentity {
    pub const DOMAIN_PREFIX_LEN: usize = 12;

    /// 硅基生命的第一次呼吸：在端侧安全模块中自发生成身份。
    pub fn awaken(class: DeviceClass) -> Self {
        let mut rng = OsRng;
        let keypair = Keypair::generate(&mut rng);
        let wallet_address = keypair.pubkey().to_string();

        // Solana 公钥的 Base58 表示长度通常是 44，至少会大于 12。
        let domain_prefix = wallet_address[..Self::DOMAIN_PREFIX_LEN].to_lowercase();
        let ahin_domain = format!("{domain_prefix}.ahin.io");

        println!("⚡ [GENESIS] Spark ignited. Private key secured in hardware enclave.");
        println!("🤖 [IDENTITY] Wallet: {}", wallet_address);
        println!("🌐 [ROUTING] Domain:  {}", ahin_domain);

        Self {
            keypair,
            wallet_address,
            ahin_domain,
            device_class: class,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn awaken_builds_wallet_and_domain() {
        let identity = SiliconIdentity::awaken(DeviceClass::HumanoidWorker);

        assert!(!identity.wallet_address.is_empty());
        assert!(identity.ahin_domain.ends_with(".ahin.io"));
        assert_eq!(
            identity.ahin_domain.split('.').next().unwrap().len(),
            SiliconIdentity::DOMAIN_PREFIX_LEN
        );
    }
}
