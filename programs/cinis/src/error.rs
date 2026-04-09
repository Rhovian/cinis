use quasar_lang::prelude::*;

#[error_code]
pub enum CinisError {
    /// Duel is not in pending status.
    NotPending = 6000,
    /// Duel is not in active status.
    NotActive,
    /// Invalid winner value (must be 0 or 1).
    InvalidWinner,
    /// Fee basis points too high (max 10000).
    FeeTooHigh,
    /// Unauthorized canceller.
    Unauthorized,
}
