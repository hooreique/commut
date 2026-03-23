//! Web Crypto-compatible cryptographic boundary for the Rust port.

use anyhow::{Result, anyhow};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use hkdf::Hkdf;
use p256::elliptic_curve::rand_core::OsRng;
use p256::{
    PublicKey, SecretKey,
    ecdh::diffie_hellman,
    ecdsa::{Signature, VerifyingKey, signature::Verifier},
    pkcs8::{DecodePublicKey, EncodePublicKey},
};
use rand::random;
use sha2::Sha256;

use crate::contract::{HKDF_INFO_DOWN, HKDF_INFO_UP};
use crate::state::{Checkpoint, SessionCrypto};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedKeySet {
    verifying_key: VerifyingKey,
}

impl AuthorizedKeySet {
    /// Parse the authorized ECDSA public key PEM used for handshake validation.
    ///
    /// # Errors
    ///
    /// Returns an error when the PEM cannot be decoded into a P-256 public key.
    pub fn from_public_pem(public_key_pem: &str) -> Result<Self> {
        let verifying_key = VerifyingKey::from_public_key_pem(public_key_pem)?;
        Ok(Self { verifying_key })
    }

    /// Verify a browser-produced ECDSA signature over the base64-decoded wire
    /// value, matching the TypeScript/Web Crypto implementation.
    ///
    /// # Errors
    ///
    /// Returns an error when the signed value or signature is not valid Base64
    /// or cannot be parsed as a supported ECDSA signature encoding.
    pub fn verify_authorization_signature(
        &self,
        signed_base64_value: &str,
        signature_base64: &str,
    ) -> Result<bool> {
        let message = BASE64_STANDARD.decode(signed_base64_value.as_bytes())?;
        let signature_bytes = BASE64_STANDARD.decode(signature_base64.as_bytes())?;
        let signature = Signature::from_slice(&signature_bytes)
            .or_else(|_| Signature::from_der(&signature_bytes))?;
        Ok(self.verifying_key.verify(&message, &signature).is_ok())
    }

    /// Generate a one-shot checkpoint for the `/api/ticket` to `/api/salt`
    /// handshake.
    ///
    /// # Errors
    ///
    /// Returns an error when the generated ephemeral public key cannot be
    /// serialized into DER.
    pub fn generate_checkpoint(&self) -> Result<(String, Checkpoint)> {
        let id = encode_random_bytes::<8>();
        let private_key = SecretKey::random(&mut OsRng);
        let public_key = private_key.public_key();
        let public_key_der = public_key.to_public_key_der()?;
        let checkpoint = Checkpoint {
            server_ephemeral_private_key: private_key,
            server_ephemeral_public_key_base64: BASE64_STANDARD.encode(public_key_der.as_bytes()),
        };
        Ok((id, checkpoint))
    }

    /// Complete the server side of the ECDH + HKDF handshake used by
    /// `POST /api/salt`.
    ///
    /// The returned salt is 32 random bytes encoded as Base64. The derived
    /// session keys are directional AES-128-GCM keys:
    /// - `client -> server` for decrypting browser traffic
    /// - `server -> client` for encrypting PTY output
    ///
    /// # Errors
    ///
    /// Returns an error when the client ephemeral key is malformed or key
    /// derivation fails.
    pub fn derive_session_crypto(
        &self,
        checkpoint: &Checkpoint,
        client_ephemeral_public_key_base64: &str,
    ) -> Result<(String, SessionCrypto)> {
        let salt = random_bytes::<32>();
        let session_crypto = self.derive_session_crypto_with_salt(
            checkpoint,
            client_ephemeral_public_key_base64,
            salt,
        )?;
        let salt_base64 = session_crypto.salt_base64.clone();
        Ok((salt_base64, session_crypto))
    }

    /// Deterministic variant of the ECDH + HKDF handshake used by fixture-based
    /// interoperability tests. Production code should normally call
    /// `derive_session_crypto`, which generates a fresh random salt internally.
    ///
    /// # Errors
    ///
    /// Returns an error when the client public key cannot be decoded or HKDF
    /// expansion fails.
    pub fn derive_session_crypto_with_salt(
        &self,
        checkpoint: &Checkpoint,
        client_ephemeral_public_key_base64: &str,
        salt: [u8; 32],
    ) -> Result<SessionCrypto> {
        let client_public_key_der =
            BASE64_STANDARD.decode(client_ephemeral_public_key_base64.as_bytes())?;
        let client_public_key = PublicKey::from_public_key_der(&client_public_key_der)
            .map_err(|_| anyhow::anyhow!("client public key import failure"))?;
        let shared_secret = diffie_hellman(
            checkpoint.server_ephemeral_private_key.to_nonzero_scalar(),
            client_public_key.as_affine(),
        );
        let hkdf = Hkdf::<Sha256>::new(Some(&salt), shared_secret.raw_secret_bytes().as_slice());
        let decrypt_key_bytes = derive_aes128_key(&hkdf, HKDF_INFO_UP)?;
        let encrypt_key_bytes = derive_aes128_key(&hkdf, HKDF_INFO_DOWN)?;

        Ok(SessionCrypto {
            salt_base64: BASE64_STANDARD.encode(salt),
            decrypt_key_bytes,
            encrypt_key_bytes,
        })
    }
}

fn encode_random_bytes<const N: usize>() -> String {
    BASE64_STANDARD.encode(random_bytes::<N>())
}

fn random_bytes<const N: usize>() -> [u8; N] {
    random::<[u8; N]>()
}

fn derive_aes128_key(hkdf: &Hkdf<Sha256>, info: &[u8]) -> Result<[u8; 16]> {
    let mut key = [0_u8; 16];
    hkdf.expand(info, &mut key)
        .map_err(|_| anyhow!("hkdf expand failure"))?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::{
        PublicKey, SecretKey,
        ecdsa::{SigningKey, signature::Signer},
        pkcs8::{DecodePublicKey, EncodePublicKey, LineEnding},
    };

    #[test]
    fn signature_verifier_accepts_webcrypto_compatible_fixed_width_signatures() {
        let signing_key = SigningKey::from_bytes((&[3_u8; 32]).into()).expect("signing key");
        let pem = signing_key
            .verifying_key()
            .to_public_key_pem(LineEnding::default())
            .expect("public key pem");
        let keys = AuthorizedKeySet::from_public_pem(&pem).expect("valid public key");
        let value = BASE64_STANDARD.encode(b"nonce");
        let signature: Signature = signing_key.sign(b"nonce");
        let signature_base64 = BASE64_STANDARD.encode(signature.to_bytes());
        let wrong_signature: Signature = signing_key.sign(b"other");
        let wrong_signature_base64 = BASE64_STANDARD.encode(wrong_signature.to_bytes());

        assert!(
            keys.verify_authorization_signature(&value, &signature_base64)
                .expect("verification should succeed")
        );
        assert!(
            !keys
                .verify_authorization_signature(&value, &wrong_signature_base64)
                .expect("verification should succeed")
        );
    }

    #[test]
    fn checkpoint_and_session_crypto_generation_return_base64_wire_values() {
        let signing_key = SigningKey::from_bytes((&[9_u8; 32]).into()).expect("signing key");
        let pem = signing_key
            .verifying_key()
            .to_public_key_pem(LineEnding::default())
            .expect("public key pem");
        let keys = AuthorizedKeySet::from_public_pem(&pem).expect("valid public key");
        let (id, checkpoint) = keys.generate_checkpoint().expect("checkpoint generation");
        assert!(BASE64_STANDARD.decode(id.as_bytes()).is_ok());
        let server_pub_der = BASE64_STANDARD
            .decode(checkpoint.server_ephemeral_public_key_base64.as_bytes())
            .expect("server public key der");
        PublicKey::from_public_key_der(&server_pub_der).expect("valid server public key");

        let client_secret = SecretKey::from_slice(&[5_u8; 32]).expect("client secret");
        let client_pub = BASE64_STANDARD.encode(
            client_secret
                .public_key()
                .to_public_key_der()
                .expect("client public key der")
                .as_bytes(),
        );
        let (salt_base64, session) = keys
            .derive_session_crypto(&checkpoint, &client_pub)
            .expect("session crypto derivation");
        assert_eq!(
            BASE64_STANDARD
                .decode(salt_base64.as_bytes())
                .expect("salt")
                .len(),
            32,
            "salt must decode to 32 bytes"
        );
        assert_ne!(session.decrypt_key_bytes, session.encrypt_key_bytes);
    }

    #[test]
    fn session_crypto_encrypt_and_decrypt_round_trip() {
        let session = SessionCrypto {
            salt_base64: "salt".to_owned(),
            decrypt_key_bytes: [7_u8; 16],
            encrypt_key_bytes: [7_u8; 16],
        };
        let iv = [4_u8; 12];
        let ciphertext = session.encrypt(&iv, b"hello").expect("encrypt");
        let plaintext = session.decrypt(&iv, &ciphertext).expect("decrypt");
        assert_eq!(plaintext, b"hello");
    }
}
