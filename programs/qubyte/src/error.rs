use anchor_lang::prelude::*;

#[error_code]
pub enum QubyteError {
    #[msg("Falcon public key is malformed")]
    InvalidFalconPubkey,
    #[msg("Falcon public key does not match the supplied hash")]
    PubkeyHashMismatch,
    #[msg("Falcon signature is malformed")]
    InvalidSignatureLength,
    #[msg("Falcon signature verification failed")]
    InvalidSignature,
    #[msg("Nonce does not match the identity's next nonce")]
    InvalidNonce,
    #[msg("The signed action has expired")]
    SignatureExpired,
    #[msg("Metadata string exceeds its maximum length")]
    MetadataTooLong,
    #[msg("Metadata does not match the authorised launch")]
    MetadataMismatch,
    #[msg("pump.fun account does not match the expected key")]
    PumpAccountMismatch,
    #[msg("Not enough creator fees in fee_vault to buy back")]
    NothingToClaim,
    #[msg("Buyback venue is not the coin's canonical pump.fun curve or PumpSwap pool")]
    InvalidVenue,
    #[msg("Buyback returned no tokens")]
    NothingBought,
}
