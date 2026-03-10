use anchor_lang::prelude::*;

#[error_code]
pub enum LifePlusError {
    #[msg("Thermodynamic constraint violated: Compute limit exceeded.")]
    ThermodynamicBoundaryExceeded,
    #[msg("Zero-Knowledge Cognitive Proof verification failed.")]
    InvalidCognitiveProof,
    #[msg("Subtask has not passed PoCC verification yet.")]
    SubtaskNotVerified,
    #[msg("This CTx settlement has already been claimed.")]
    DoubleSpendingAttempt,
    #[msg("Subtask ID not found in the agreed state registry.")]
    SubtaskNotFound,
    #[msg("Agent identity is already marked as slashed/dead.")]
    AgentAlreadyDead,
    #[msg("Arithmetic overflow while processing protocol math.")]
    ArithmeticOverflow,
    #[msg("Invalid payment ratio in bips.")]
    InvalidPaymentBips,
    #[msg("Settlement currently locked to prevent reentrancy-like double execution.")]
    SettlementLocked,
    #[msg("Unauthorized settlement authority.")]
    UnauthorizedAuthority,
    #[msg("Escrow and worker token mints must match.")]
    MintMismatch,
}
