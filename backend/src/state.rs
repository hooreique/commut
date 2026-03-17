//! In-memory application state for the Rust backend.
//!
//! The HTTP/WebSocket handshake relies on three one-shot stores:
//! - nonce store
//!   - populated by `POST /api/nonce`
//!   - entries expire after `NONCE_TTL_MS`
//! - checkpoint store
//!   - populated by `POST /api/ticket`
//!   - consumed by `POST /api/salt`
//! - crypt store
//!   - populated by `POST /api/salt`
//!   - consumed by `GET /sockets/{id}`
//!
//! All state is process-local and lives only in memory.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use aes_gcm::{
    Aes128Gcm,
    aead::{Aead, KeyInit},
};
use anyhow::Result;
use p256::SecretKey;
use tokio::sync::RwLock;

use crate::contract::NONCE_TTL_MS;

pub type SharedAppState = Arc<AppState>;

#[derive(Debug)]
pub struct AppState {
    nonces: RwLock<HashMap<String, Instant>>,
    checkpoints: RwLock<HashMap<String, Checkpoint>>,
    crypts: RwLock<HashMap<String, SessionCrypto>>,
}

impl AppState {
    #[must_use]
    pub fn new() -> SharedAppState {
        Arc::new(Self {
            nonces: RwLock::new(HashMap::new()),
            checkpoints: RwLock::new(HashMap::new()),
            crypts: RwLock::new(HashMap::new()),
        })
    }

    pub async fn insert_nonce(&self, nonce: String) {
        self.nonces.write().await.insert(nonce, Instant::now());
    }

    pub async fn contains_nonce(&self, nonce: &str) -> bool {
        self.prune_expired_nonces().await;
        self.nonces.read().await.contains_key(nonce)
    }

    pub async fn consume_nonce(&self, nonce: &str) -> bool {
        self.prune_expired_nonces().await;
        self.nonces.write().await.remove(nonce).is_some()
    }

    pub async fn insert_checkpoint(&self, id: String, checkpoint: Checkpoint) {
        self.checkpoints.write().await.insert(id, checkpoint);
    }

    pub async fn checkpoint(&self, id: &str) -> Option<Checkpoint> {
        self.checkpoints.read().await.get(id).cloned()
    }

    pub async fn consume_checkpoint(&self, id: &str) -> Option<Checkpoint> {
        self.checkpoints.write().await.remove(id)
    }

    pub async fn insert_crypt(&self, id: String, crypt: SessionCrypto) {
        self.crypts.write().await.insert(id, crypt);
    }

    pub async fn crypt(&self, id: &str) -> Option<SessionCrypto> {
        self.crypts.read().await.get(id).cloned()
    }

    pub async fn consume_crypt(&self, id: &str) -> Option<SessionCrypto> {
        self.crypts.write().await.remove(id)
    }

    async fn prune_expired_nonces(&self) {
        let ttl = Duration::from_millis(NONCE_TTL_MS);
        self.nonces
            .write()
            .await
            .retain(|_, issued_at| issued_at.elapsed() < ttl);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkpoint {
    pub server_ephemeral_private_key: SecretKey,
    pub server_ephemeral_public_key_base64: String,
}

/// Directional AES-128-GCM session material for a single established session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCrypto {
    pub salt_base64: String,
    pub decrypt_key_bytes: [u8; 16],
    pub encrypt_key_bytes: [u8; 16],
}

impl SessionCrypto {
    /// Decrypt a browser -> server WebSocket type `0` payload using the session's
    /// HKDF-derived AES-128-GCM key material.
    ///
    /// # Errors
    ///
    /// Returns an error when the key material is invalid or authentication
    /// fails.
    pub fn decrypt(&self, iv: &[u8; 12], ciphertext_and_tag: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes128Gcm::new_from_slice(&self.decrypt_key_bytes)?;
        cipher
            .decrypt(iv.into(), ciphertext_and_tag)
            .map_err(|_| anyhow::anyhow!("aes-gcm decrypt failure"))
    }

    /// Encrypt a server -> browser WebSocket type `0` payload using the session's
    /// HKDF-derived AES-128-GCM key material.
    ///
    /// # Errors
    ///
    /// Returns an error when the key material is invalid or encryption fails.
    pub fn encrypt(&self, iv: &[u8; 12], plaintext: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes128Gcm::new_from_slice(&self.encrypt_key_bytes)?;
        cipher
            .encrypt(iv.into(), plaintext)
            .map_err(|_| anyhow::anyhow!("aes-gcm encrypt failure"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    async fn nonce_store_consumes_values_exactly_once() {
        let state = AppState::new();
        state.insert_nonce("nonce-1".to_owned()).await;

        assert!(state.contains_nonce("nonce-1").await);
        assert!(state.consume_nonce("nonce-1").await);
        assert!(!state.consume_nonce("nonce-1").await);
    }

    #[tokio::test]
    async fn checkpoint_and_crypt_stores_are_one_shot_maps() {
        let state = AppState::new();

        state
            .insert_checkpoint(
                "id-1".to_owned(),
                Checkpoint {
                    server_ephemeral_private_key: SecretKey::from_slice(&[7_u8; 32])
                        .expect("test private key"),
                    server_ephemeral_public_key_base64: "server-pub".to_owned(),
                },
            )
            .await;
        state
            .insert_crypt(
                "id-1".to_owned(),
                SessionCrypto {
                    salt_base64: "salt".to_owned(),
                    decrypt_key_bytes: [1_u8; 16],
                    encrypt_key_bytes: [2_u8; 16],
                },
            )
            .await;

        assert!(state.consume_checkpoint("id-1").await.is_some());
        assert!(state.consume_checkpoint("id-1").await.is_none());
        assert!(state.consume_crypt("id-1").await.is_some());
        assert!(state.consume_crypt("id-1").await.is_none());
    }

    #[tokio::test]
    async fn nonce_store_expires_entries_after_the_spec_ttl() {
        let state = AppState::new();
        state.insert_nonce("nonce-1".to_owned()).await;

        sleep(Duration::from_millis(NONCE_TTL_MS + 25)).await;

        assert!(!state.contains_nonce("nonce-1").await);
        assert!(!state.consume_nonce("nonce-1").await);
    }
}
