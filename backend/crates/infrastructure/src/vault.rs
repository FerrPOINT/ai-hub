use aes_gcm::{
    Aes256Gcm, KeyInit,
    aead::{Aead, AeadCore, OsRng, Payload},
};
use aihub_domain::error::HubError;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zeroize::Zeroizing;

pub struct Vault {
    key: Zeroizing<Vec<u8>>,
}
pub struct Sealed {
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

impl Vault {
    pub fn new(key: Vec<u8>) -> Result<Self, HubError> {
        if key.len() != 32 {
            return Err(HubError::Invalid("vault key length"));
        }
        Ok(Self {
            key: Zeroizing::new(key),
        })
    }
    pub fn fingerprint(&self) -> Vec<u8> {
        Sha256::digest(&self.key).to_vec()
    }
    fn aad(installation: Uuid, object: Uuid, generation: i64, purpose: &str) -> Vec<u8> {
        format!("AIHUB-VAULT-V1\0{installation}\0{object}\0{generation}\0{purpose}").into_bytes()
    }
    pub fn seal(
        &self,
        installation: Uuid,
        object: Uuid,
        generation: i64,
        purpose: &str,
        plaintext: &[u8],
    ) -> Result<Sealed, HubError> {
        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|_| HubError::Unavailable)?;
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let aad = Self::aad(installation, object, generation, purpose);
        let ciphertext = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: plaintext,
                    aad: &aad,
                },
            )
            .map_err(|_| HubError::Unavailable)?;
        Ok(Sealed {
            nonce: nonce.into(),
            ciphertext,
        })
    }
    pub fn open(
        &self,
        installation: Uuid,
        object: Uuid,
        generation: i64,
        purpose: &str,
        sealed: &Sealed,
    ) -> Result<Zeroizing<Vec<u8>>, HubError> {
        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|_| HubError::Unavailable)?;
        let aad = Self::aad(installation, object, generation, purpose);
        cipher
            .decrypt(
                (&sealed.nonce).into(),
                Payload {
                    msg: &sealed.ciphertext,
                    aad: &aad,
                },
            )
            .map(Zeroizing::new)
            .map_err(|_| HubError::InstallationMismatch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secrets_are_bound_to_installation_generation_and_purpose() {
        let vault = Vault::new(vec![7; 32]).unwrap();
        let installation = Uuid::new_v4();
        let connection = Uuid::new_v4();
        let sealed = vault
            .seal(
                installation,
                connection,
                1,
                "credential",
                b"synthetic-canary",
            )
            .unwrap();
        assert_ne!(sealed.ciphertext, b"synthetic-canary");
        assert_eq!(
            vault
                .open(installation, connection, 1, "credential", &sealed)
                .unwrap()
                .as_slice(),
            b"synthetic-canary"
        );
        assert!(
            vault
                .open(Uuid::new_v4(), connection, 1, "credential", &sealed)
                .is_err()
        );
        assert!(
            vault
                .open(installation, connection, 2, "credential", &sealed)
                .is_err()
        );
        assert!(
            vault
                .open(installation, connection, 1, "replay", &sealed)
                .is_err()
        );
        assert!(
            Vault::new(vec![8; 32])
                .unwrap()
                .open(installation, connection, 1, "credential", &sealed)
                .is_err()
        );
        assert_ne!(
            sealed.nonce,
            vault
                .seal(
                    installation,
                    connection,
                    1,
                    "credential",
                    b"synthetic-canary"
                )
                .unwrap()
                .nonce
        );
    }
}
