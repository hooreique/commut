//! Browser-generated Web Crypto interoperability tests.
//!
//! These tests intentionally consume fixed artifacts generated with Node's Web
//! Crypto API. The goal is to prove that the Rust implementation accepts real
//! browser-shaped wire values, not only values produced by Rust itself.

use anyhow::Result;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use commut::{crypto::AuthorizedKeySet, state::Checkpoint};
use p256::SecretKey;
use p256::pkcs8::DecodePrivateKey;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct WebCryptoFixture {
    authorization_public_key_pem: String,
    nonce_base64: String,
    nonce_signature_base64: String,
    id_base64: String,
    id_signature_base64: String,
    server_ecdh_private_key_pkcs8_base64: String,
    server_ecdh_public_key_spki_base64: String,
    client_ecdh_public_key_spki_base64: String,
    salt_base64: String,
    expected_client_to_server_key_base64: String,
    expected_server_to_client_key_base64: String,
    client_to_server_iv_base64: String,
    client_to_server_plaintext_base64: String,
    client_to_server_ciphertext_base64: String,
    server_to_client_iv_base64: String,
    server_to_client_plaintext_base64: String,
    server_to_client_ciphertext_base64: String,
}

fn fixture() -> WebCryptoFixture {
    serde_json::from_str(include_str!("fixtures/webcrypto_vectors.json"))
        .expect("fixture json must stay valid")
}

fn decode<const N: usize>(value: &str) -> [u8; N] {
    BASE64_STANDARD
        .decode(value.as_bytes())
        .expect("fixture base64 must decode")
        .try_into()
        .expect("fixture bytes must have expected length")
}

#[test]
fn browser_generated_ecdsa_signatures_verify_against_fixture_public_key() -> Result<()> {
    // Contract coverage:
    // - authorization signatures are verified over Base64-decoded wire values
    let fixture = fixture();
    let keys = AuthorizedKeySet::from_public_pem(&fixture.authorization_public_key_pem)?;

    assert!(
        keys.verify_authorization_signature(
            &fixture.nonce_base64,
            &fixture.nonce_signature_base64,
        )?
    );
    assert!(
        keys.verify_authorization_signature(&fixture.id_base64, &fixture.id_signature_base64,)?
    );

    Ok(())
}

#[test]
fn browser_generated_ecdh_fixture_derives_the_exact_expected_directional_keys() -> Result<()> {
    // Contract coverage:
    // - ephemeral ECDH public keys use SPKI encoding
    // - HKDF info strings are fixed protocol constants
    // - the server derives directional AES-128-GCM keys from ECDH + HKDF
    let fixture = fixture();
    let keys = AuthorizedKeySet::from_public_pem(&fixture.authorization_public_key_pem)?;
    let checkpoint = Checkpoint {
        server_ephemeral_private_key: SecretKey::from_pkcs8_der(
            &BASE64_STANDARD.decode(fixture.server_ecdh_private_key_pkcs8_base64.as_bytes())?,
        )?,
        server_ephemeral_public_key_base64: fixture.server_ecdh_public_key_spki_base64.clone(),
    };
    let salt = decode::<32>(&fixture.salt_base64);

    let session = keys.derive_session_crypto_with_salt(
        &checkpoint,
        &fixture.client_ecdh_public_key_spki_base64,
        salt,
    )?;

    assert_eq!(
        BASE64_STANDARD.encode(session.decrypt_key_bytes),
        fixture.expected_client_to_server_key_base64
    );
    assert_eq!(
        BASE64_STANDARD.encode(session.encrypt_key_bytes),
        fixture.expected_server_to_client_key_base64
    );

    Ok(())
}

#[test]
fn rust_decrypts_browser_generated_client_to_server_ciphertext() -> Result<()> {
    // Spec coverage:
    // - section 11.2: type `0` payloads are AES-GCM with a 12-byte IV
    // - section 8.3: browser -> server traffic uses the `client -> server` key
    let fixture = fixture();
    let keys = AuthorizedKeySet::from_public_pem(&fixture.authorization_public_key_pem)?;
    let checkpoint = Checkpoint {
        server_ephemeral_private_key: SecretKey::from_pkcs8_der(
            &BASE64_STANDARD.decode(fixture.server_ecdh_private_key_pkcs8_base64.as_bytes())?,
        )?,
        server_ephemeral_public_key_base64: fixture.server_ecdh_public_key_spki_base64.clone(),
    };
    let salt = decode::<32>(&fixture.salt_base64);
    let iv = decode::<12>(&fixture.client_to_server_iv_base64);
    let ciphertext =
        BASE64_STANDARD.decode(fixture.client_to_server_ciphertext_base64.as_bytes())?;
    let expected_plaintext =
        BASE64_STANDARD.decode(fixture.client_to_server_plaintext_base64.as_bytes())?;

    let session = keys.derive_session_crypto_with_salt(
        &checkpoint,
        &fixture.client_ecdh_public_key_spki_base64,
        salt,
    )?;
    let plaintext = session.decrypt(&iv, &ciphertext)?;

    assert_eq!(plaintext, expected_plaintext);

    Ok(())
}

#[test]
fn rust_encryption_matches_browser_generated_server_to_client_ciphertext_for_fixed_iv() -> Result<()>
{
    // Spec coverage:
    // - section 11.2: type `0` payloads are AES-GCM with a 12-byte IV
    // - section 8.3: server -> browser traffic uses the `server -> client` key
    // - this exact-byte assertion proves browser-compatible AES-GCM output for a fixed IV
    let fixture = fixture();
    let keys = AuthorizedKeySet::from_public_pem(&fixture.authorization_public_key_pem)?;
    let checkpoint = Checkpoint {
        server_ephemeral_private_key: SecretKey::from_pkcs8_der(
            &BASE64_STANDARD.decode(fixture.server_ecdh_private_key_pkcs8_base64.as_bytes())?,
        )?,
        server_ephemeral_public_key_base64: fixture.server_ecdh_public_key_spki_base64.clone(),
    };
    let salt = decode::<32>(&fixture.salt_base64);
    let iv = decode::<12>(&fixture.server_to_client_iv_base64);
    let plaintext = BASE64_STANDARD.decode(fixture.server_to_client_plaintext_base64.as_bytes())?;

    let session = keys.derive_session_crypto_with_salt(
        &checkpoint,
        &fixture.client_ecdh_public_key_spki_base64,
        salt,
    )?;
    let ciphertext = session.encrypt(&iv, &plaintext)?;

    assert_eq!(
        BASE64_STANDARD.encode(ciphertext),
        fixture.server_to_client_ciphertext_base64
    );

    Ok(())
}
