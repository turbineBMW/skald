//! Amazon device registration + request signing (the same scheme the Audible
//! mobile apps use). Tokens live in `~/.config/skald/auth.json`.
pub mod store;
pub mod signer;
pub mod login;
