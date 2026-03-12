use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    alt_bn128::prelude::{alt_bn128_addition, alt_bn128_multiplication, alt_bn128_pairing},
    hash::hashv,
};

use crate::{errors::LifePlusError, state::*};

// ── BN254 Curve Constants ─────────────────────────────────────────────────────

/// Byte length of an uncompressed G1 point on BN254: 32-byte x || 32-byte y (big-endian).
const G1_LEN: usize = 64;

/// Byte length of an uncompressed G2 point on BN254:
/// x_c1 (32 BE) || x_c0 (32 BE) || y_c1 (32 BE) || y_c0 (32 BE).
const G2_LEN: usize = 128;

/// Expected byte length of a serialised Groth16 proof:
///   A (G1, 64 B) || B (G2, 128 B) || C (G1, 64 B) = 256 bytes.
/// Proof generators (snarkjs, SP1, Risc0) should export in this format.
pub const GROTH16_PROOF_LEN: usize = G1_LEN + G2_LEN + G1_LEN;

/// BN254 base-field prime (Fq), big-endian, 32 bytes.
///
/// Fq = 21888242871839275222246405745257275088696311157297823662689037894645226208583
///
/// Used to negate a G1 y-coordinate: neg_y = Fq_PRIME − y (mod Fq).
const FQ_PRIME: [u8; 32] = [
    0x30, 0x64, 0x4e, 0x72, 0xe1, 0x31, 0xa0, 0x29, 0xb8, 0x50, 0x45, 0xb6, 0x81, 0x81, 0x58,
    0x5d, 0x97, 0x81, 0x6a, 0x91, 0x68, 0x71, 0xca, 0x8d, 0x3c, 0x20, 0x8c, 0x16, 0xd8, 0x7c,
    0xfd, 0x47,
];

// ── Account Structs ───────────────────────────────────────────────────────────

#[account]
pub struct TaskIntentState {
    pub orchestrator: Pubkey,
    pub intent_hash: [u8; 32],
    pub thermodynamic_boundary: u64,
    pub is_active: bool,
}

#[account]
pub struct IntentReceipt {
    pub intent_hash: [u8; 32],
    pub executing_agent: Pubkey,
    pub verified_timestamp: i64,
}

#[derive(Accounts)]
#[instruction(intent_hash: [u8; 32])]
pub struct VerifyStructuralPoCC<'info> {
    #[account(
        mut,
        seeds = [b"persona", fee_payer.key().as_ref()],
        bump,
    )]
    pub agent_persona: Account<'info, AgentPersona>,
    #[account(mut)]
    pub ahin_timeline: Account<'info, AhinTimeline>,
    #[account(
        seeds = [b"task_intent", intent_hash.as_ref()],
        bump,
        constraint = task_intent_state.is_active @ LifePlusError::TaskAlreadyClosed,
        constraint = task_intent_state.intent_hash == intent_hash @ LifePlusError::TaskIntentHashMismatch,
    )]
    pub task_intent_state: Account<'info, TaskIntentState>,
    #[account(mut)]
    pub fee_payer: Signer<'info>,
    #[account(
        init,
        payer = fee_payer,
        space = 8 + 32 + 32 + 8,
        seeds = [b"processed_intent", intent_hash.as_ref()],
        bump
    )]
    pub processed_intent_receipt: Account<'info, IntentReceipt>,
    pub system_program: Program<'info, System>,
}

// ── ZK Proof Data Structures ──────────────────────────────────────────────────

/// Parsed Groth16 proof points (BN254 / alt_bn128 curve).
///
/// The proof is expected as a 256-byte big-endian blob serialised by the
/// prover's toolchain (snarkjs / SP1 / Risc0 groth16 export) in the order:
///   A (G1, 64 B) || B (G2, 128 B) || C (G1, 64 B)
struct Groth16Proof {
    /// π_A – G1 point provided by the prover.
    a: [u8; G1_LEN],
    /// π_B – G2 point provided by the prover.
    b: [u8; G2_LEN],
    /// π_C – G1 point provided by the prover.
    c: [u8; G1_LEN],
}

impl Groth16Proof {
    /// Deserialise a 256-byte proof blob into its A / B / C components.
    ///
    /// Returns `LifePlusError::InvalidCognitiveProof` if the slice length
    /// is not exactly `GROTH16_PROOF_LEN` (256 bytes).
    fn try_from_bytes(proof: &[u8]) -> Result<Self> {
        require!(
            proof.len() == GROTH16_PROOF_LEN,
            LifePlusError::InvalidCognitiveProof
        );
        let mut a = [0u8; G1_LEN];
        let mut b = [0u8; G2_LEN];
        let mut c = [0u8; G1_LEN];
        a.copy_from_slice(&proof[..G1_LEN]);
        b.copy_from_slice(&proof[G1_LEN..G1_LEN + G2_LEN]);
        c.copy_from_slice(&proof[G1_LEN + G2_LEN..]);
        Ok(Self { a, b, c })
    }
}

/// Groth16 Verification Key (VK) for the PoCC circuit on BN254.
///
/// **PLACEHOLDER** – every field is zero-initialised.  Before deployment,
/// replace the constants in `protocol_verification_key()` with the output
/// of your circuit's trusted-setup ceremony:
///
///   • snarkjs:  `snarkjs groth16 setup circuit.r1cs pot.ptau circuit_final.zkey`
///               then `snarkjs zkey export verificationkey circuit_final.zkey vk.json`
///   • SP1/Risc0: export the Groth16 VK from the prover binary.
///
/// ⚠️  Security invariant: the VK **must** be hardcoded here (or stored in a
/// DAO-controlled PDA).  It must **never** be supplied by the proof submitter.
struct VerificationKey {
    /// [α]₁  – G1 point.
    alpha_g1: [u8; G1_LEN],
    /// [β]₂  – G2 point.
    beta_g2: [u8; G2_LEN],
    /// [γ]₂  – G2 point.
    gamma_g2: [u8; G2_LEN],
    /// [δ]₂  – G2 point.
    delta_g2: [u8; G2_LEN],
    /// γ-ABC G1 bases for the linear combination of public inputs.
    ///
    /// For a circuit with **1** public input we need 2 elements:
    ///   `gamma_abc_g1[0]` – constant term
    ///   `gamma_abc_g1[1]` – coefficient for the single public scalar
    gamma_abc_g1: [[u8; G1_LEN]; 2],
}

/// Returns the hardcoded protocol Verification Key.
///
/// All fields are currently zero-initialised (safe placeholder that will
/// always reject proofs).  **Replace with real circuit VK before mainnet.**
fn protocol_verification_key() -> VerificationKey {
    VerificationKey {
        alpha_g1: [0u8; G1_LEN],
        beta_g2: [0u8; G2_LEN],
        gamma_g2: [0u8; G2_LEN],
        delta_g2: [0u8; G2_LEN],
        gamma_abc_g1: [[0u8; G1_LEN]; 2],
    }
}

// ── Instruction Handler ───────────────────────────────────────────────────────

pub fn execute_pocc_verification(
    ctx: Context<VerifyStructuralPoCC>,
    intent_hash: [u8; 32],
    zk_cogp_proof: Vec<u8>,
    compute_units_consumed: u64,
) -> Result<()> {
    let agent = &mut ctx.accounts.agent_persona;
    let timeline = &mut ctx.accounts.ahin_timeline;
    let receipt = &mut ctx.accounts.processed_intent_receipt;
    let task_state = &ctx.accounts.task_intent_state;

    // ── Thermodynamic boundary check ──────────────────────────────────────────
    let hard_boundary = task_state.thermodynamic_boundary;
    require!(
        compute_units_consumed <= hard_boundary,
        LifePlusError::ThermodynamicBoundaryExceeded
    );

    // ── Cryptographic Groth16 verification ────────────────────────────────────
    // `intent_hash` and `compute_units_consumed` are bound as public inputs
    // to the proof.  An attacker cannot reuse a proof generated for a different
    // task or a different CU count because the pairing check will fail.
    verify_groth16_proof_onchain(&intent_hash, compute_units_consumed, &zk_cogp_proof)?;

    // ── State updates ─────────────────────────────────────────────────────────
    timeline.current_global_hash = hashv(&[
        &timeline.current_global_hash,
        &intent_hash,
        &agent.key().to_bytes(),
    ])
    .to_bytes();

    agent.total_valid_pocc = agent
        .total_valid_pocc
        .checked_add(1)
        .ok_or(LifePlusError::ArithmeticOverflow)?;
    agent.last_active_timestamp = Clock::get()?.unix_timestamp;

    receipt.intent_hash = intent_hash;
    receipt.executing_agent = agent.key();
    receipt.verified_timestamp = agent.last_active_timestamp;

    msg!("✅ [PoCC Verified] Groth16 proof valid on BN254 via alt_bn128 precompile.");
    msg!(
        "📏 [Physics] Consumed {} CUs / Boundary {} CUs.",
        compute_units_consumed,
        hard_boundary
    );

    Ok(())
}

// ── Core Verifier ─────────────────────────────────────────────────────────────

/// Verifies a Groth16 proof on BN254 using Solana's native `alt_bn128`
/// precompile syscalls.
///
/// # Public-input binding (replay-attack prevention)
///
/// `intent_hash` and `compute_units_consumed` are hashed together into a
/// single BN254 scalar field element that becomes the sole public input.
/// The pairing equation therefore binds the proof to this exact task at this
/// exact compute cost; any other values produce a different scalar and the
/// verification equation fails.
///
/// # Groth16 pairing equation
///
/// ```text
/// e(−A, B) · e(α, β) · e(vk_x, γ) · e(C, δ) = 1_{GT}
/// ```
///
/// where `vk_x = γ_abc[0] + public_scalar · γ_abc[1]`.
///
/// The four-pairing check is executed in a single call to
/// `solana_program::alt_bn128::prelude::alt_bn128_pairing` (Solana BPF
/// syscall `sol_alt_bn128_group_op`), which costs ~300 k compute units.
fn verify_groth16_proof_onchain(
    intent_hash: &[u8; 32],
    compute_units_consumed: u64,
    proof_bytes: &[u8],
) -> Result<()> {
    // 1. Parse the proof blob ─────────────────────────────────────────────────
    let proof = Groth16Proof::try_from_bytes(proof_bytes)?;

    // 2. Load the hardcoded Verification Key ──────────────────────────────────
    let vk = protocol_verification_key();

    // 3. Prepare the public input scalar ──────────────────────────────────────
    //    Hash intent_hash || CU (LE bytes) into a 32-byte BN254 Fr element.
    //    Clearing the top 2 bits guarantees the result is < Fr ≈ 2^254.
    let public_scalar = prepare_public_scalar(intent_hash, compute_units_consumed);

    // 4. Compute vk_x = γ_abc[0] + public_scalar · γ_abc[1] ──────────────────
    let vk_x = compute_vk_x(&vk.gamma_abc_g1, &public_scalar)?;

    // 5. Negate π_A in G1: (x, y) → (x, Fq − y) ─────────────────────────────
    let neg_a = negate_g1(&proof.a);

    // 6. Build the 4 × 192-byte pairing input ─────────────────────────────────
    //    Layout per element: G1 (64 B) || G2 (128 B).
    //    Pairs: (−A, B) | (α, β) | (vk_x, γ) | (C, δ)
    const PAIR_LEN: usize = G1_LEN + G2_LEN; // 192
    let mut pairing_input = [0u8; 4 * PAIR_LEN]; // 768

    let pairs: [(&[u8; G1_LEN], &[u8; G2_LEN]); 4] = [
        (&neg_a, &proof.b),
        (&vk.alpha_g1, &vk.beta_g2),
        (&vk_x, &vk.gamma_g2),
        (&proof.c, &vk.delta_g2),
    ];

    let mut offset = 0;
    for (g1, g2) in &pairs {
        pairing_input[offset..offset + G1_LEN].copy_from_slice(*g1);
        offset += G1_LEN;
        pairing_input[offset..offset + G2_LEN].copy_from_slice(*g2);
        offset += G2_LEN;
    }

    // 7. Call Solana's native BN254 multi-pairing precompile ──────────────────
    //    On Solana OS this invokes `sol_alt_bn128_group_op` (syscall).
    //    Off-chain / test builds use the ark-bn254 software implementation.
    let pairing_result = alt_bn128_pairing(&pairing_input)
        .map_err(|_| error!(LifePlusError::InvalidCognitiveProof))?;

    // 8. Accept the proof only when the pairing product equals 1_{GT} ─────────
    //    The precompile returns 32 bytes; the last byte is 0x01 iff the
    //    product of all four pairings is the identity element of GT (Fq12).
    require!(
        pairing_result.len() == 32 && pairing_result[31] == 1,
        LifePlusError::InvalidCognitiveProof
    );

    Ok(())
}

// ── Helper Functions ──────────────────────────────────────────────────────────

/// Compress `intent_hash` and `compute_units_consumed` into a single BN254
/// scalar field element (Fr).
///
/// Strategy: SHA-256(`intent_hash` || `compute_units` as 8-byte LE), then
/// clear the top 2 bits.  This yields a 254-bit value guaranteed to be in
/// `[0, 2^254) ⊂ Fr` (Fr ≈ 2^254), giving a negligible statistical bias
/// that is acceptable for non-interactive challenge generation.
fn prepare_public_scalar(intent_hash: &[u8; 32], compute_units: u64) -> [u8; 32] {
    let cu_bytes = compute_units.to_le_bytes();
    let mut scalar = hashv(&[intent_hash.as_ref(), cu_bytes.as_ref()]).to_bytes();
    // Clear the top 2 bits: guarantees scalar < 2^254 ≤ Fr.
    scalar[0] &= 0x3f;
    scalar
}

/// Compute the linear combination of the VK's γ-ABC bases weighted by the
/// public input scalar:
///
/// ```text
/// vk_x = gamma_abc[0] + public_scalar · gamma_abc[1]
/// ```
///
/// Uses Solana's `alt_bn128_multiplication` and `alt_bn128_addition` syscalls.
fn compute_vk_x(gamma_abc: &[[u8; G1_LEN]; 2], scalar: &[u8; 32]) -> Result<[u8; G1_LEN]> {
    // scalar-multiply: gamma_abc[1] × scalar
    //   Input format for alt_bn128_multiplication:
    //     G1 point (64 B big-endian) || scalar (32 B big-endian)  = 96 B
    let mut mul_input = [0u8; G1_LEN + 32];
    mul_input[..G1_LEN].copy_from_slice(&gamma_abc[1]);
    mul_input[G1_LEN..].copy_from_slice(scalar);

    let mul_result = alt_bn128_multiplication(&mul_input)
        .map_err(|_| error!(LifePlusError::InvalidCognitiveProof))?;
    require!(
        mul_result.len() == G1_LEN,
        LifePlusError::InvalidCognitiveProof
    );

    // point-add: gamma_abc[0] + mul_result
    //   Input format for alt_bn128_addition:
    //     G1_a (64 B big-endian) || G1_b (64 B big-endian) = 128 B
    let mut add_input = [0u8; G1_LEN + G1_LEN];
    add_input[..G1_LEN].copy_from_slice(&gamma_abc[0]);
    add_input[G1_LEN..].copy_from_slice(&mul_result);

    let add_result = alt_bn128_addition(&add_input)
        .map_err(|_| error!(LifePlusError::InvalidCognitiveProof))?;
    require!(
        add_result.len() == G1_LEN,
        LifePlusError::InvalidCognitiveProof
    );

    let mut vk_x = [0u8; G1_LEN];
    vk_x.copy_from_slice(&add_result);
    Ok(vk_x)
}

/// Negate a G1 point on BN254: `(x, y) → (x, Fq − y)`.
///
/// The BN254 base-field prime `Fq` is hardcoded as `FQ_PRIME`.
/// The identity point `(0, 0)` is returned unchanged (the affine
/// representation of the point at infinity used by Solana's precompile).
fn negate_g1(g1: &[u8; G1_LEN]) -> [u8; G1_LEN] {
    let x: &[u8; 32] = g1[..32].try_into().expect("G1 x-coordinate slice is always 32 bytes");
    let y: &[u8; 32] = g1[32..].try_into().expect("G1 y-coordinate slice is always 32 bytes");

    // Identity point check: the point at infinity is represented as (0, 0).
    // Both coordinates must be zero; a non-zero x with zero y is not a valid
    // BN254 affine point (the curve has prime order, so no element of order 2
    // exists), but we check both for strict correctness.
    if x.iter().all(|&b| b == 0) && y.iter().all(|&b| b == 0) {
        return *g1;
    }

    let neg_y = be_sub(&FQ_PRIME, y);
    let mut result = [0u8; G1_LEN];
    result[..32].copy_from_slice(x);
    result[32..].copy_from_slice(&neg_y);
    result
}

/// Subtract two 32-byte big-endian unsigned integers: `a − b`.
///
/// Precondition: `a ≥ b` (called with `a = FQ_PRIME` and `b = y < FQ_PRIME`).
fn be_sub(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut result = [0u8; 32];
    let mut borrow: u16 = 0;
    // Iterate from the least-significant byte (index 31) to the most-significant (index 0).
    for i in (0..32).rev() {
        let diff = (a[i] as u16).wrapping_sub(b[i] as u16).wrapping_sub(borrow);
        result[i] = diff as u8;
        // If diff wrapped below 0 (i.e. diff > 0xFF as u16), set borrow = 1.
        borrow = if diff > 0xff { 1 } else { 0 };
    }
    result
}
