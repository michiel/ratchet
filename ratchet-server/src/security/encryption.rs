//! Encryption services for data-at-rest and data-in-transit security
//!
//! This module provides encryption and decryption capabilities for sensitive
//! data storage and transmission.

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::EncryptionAlgorithm;

/// Encryption service trait for data security
#[async_trait]
pub trait EncryptionService: Send + Sync {
    /// Encrypt data using the configured algorithm
    async fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>>;
    
    /// Decrypt data using the configured algorithm
    async fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>>;
    
    /// Generate a new encryption key
    async fn generate_key(&self) -> Result<Vec<u8>>;
    
    /// Rotate encryption keys
    async fn rotate_key(&self) -> Result<()>;
}

/// Encryption key metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionKeyMetadata {
    /// Key ID
    pub key_id: String,
    /// Key algorithm
    pub algorithm: EncryptionAlgorithm,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last used timestamp
    pub last_used_at: Option<DateTime<Utc>>,
    /// Key rotation timestamp
    pub rotated_at: Option<DateTime<Utc>>,
    /// Key status
    pub status: KeyStatus,
    /// Usage count
    pub usage_count: u64,
}

/// Encryption key status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyStatus {
    Active,
    Deprecated,
    Revoked,
    Expired,
}

/// Encrypted data envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Key ID used for encryption
    pub key_id: String,
    /// Encryption algorithm
    pub algorithm: EncryptionAlgorithm,
    /// Initialization vector
    pub iv: Vec<u8>,
    /// Encrypted data
    pub data: Vec<u8>,
    /// Authentication tag (for AEAD)
    pub tag: Option<Vec<u8>>,
    /// Encrypted timestamp
    pub encrypted_at: DateTime<Utc>,
}

/// AES encryption service implementation
pub struct AesEncryptionService {
    /// Current encryption keys by algorithm
    keys: Arc<RwLock<HashMap<EncryptionAlgorithm, EncryptionKeyMetadata>>>,
    /// Key storage (in practice, this would be in a secure key management system)
    key_storage: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    /// Default algorithm
    default_algorithm: EncryptionAlgorithm,
}

impl AesEncryptionService {
    /// Create a new AES encryption service
    pub fn new(algorithm: EncryptionAlgorithm) -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            key_storage: Arc::new(RwLock::new(HashMap::new())),
            default_algorithm: algorithm,
        }
    }

    /// Initialize with a default key
    pub async fn initialize(&self) -> Result<()> {
        let key = self.generate_key().await?;
        let key_id = self.generate_key_id();
        
        let metadata = EncryptionKeyMetadata {
            key_id: key_id.clone(),
            algorithm: self.default_algorithm.clone(),
            created_at: Utc::now(),
            last_used_at: None,
            rotated_at: None,
            status: KeyStatus::Active,
            usage_count: 0,
        };

        let mut keys = self.keys.write().await;
        keys.insert(self.default_algorithm.clone(), metadata);

        let mut storage = self.key_storage.write().await;
        storage.insert(key_id, key);

        Ok(())
    }

    /// Get the current active key for an algorithm
    async fn get_active_key(&self, algorithm: &EncryptionAlgorithm) -> Result<(String, Vec<u8>)> {
        let keys = self.keys.read().await;
        let storage = self.key_storage.read().await;

        if let Some(metadata) = keys.get(algorithm) {
            if metadata.status == KeyStatus::Active {
                if let Some(key) = storage.get(&metadata.key_id) {
                    return Ok((metadata.key_id.clone(), key.clone()));
                }
            }
        }

        Err(anyhow::anyhow!("No active key found for algorithm {:?}", algorithm))
    }

    /// Generate a unique key ID
    fn generate_key_id(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(Utc::now().timestamp().to_be_bytes());
        hasher.update(rand::random::<u64>().to_be_bytes());
        format!("key_{:x}", hasher.finalize())
    }

    /// Update key usage statistics
    async fn update_key_usage(&self, key_id: &str) -> Result<()> {
        let mut keys = self.keys.write().await;
        for metadata in keys.values_mut() {
            if metadata.key_id == key_id {
                metadata.last_used_at = Some(Utc::now());
                metadata.usage_count += 1;
                break;
            }
        }
        Ok(())
    }

    /// Encrypt data with AES-256-GCM
    async fn encrypt_aes_gcm(&self, data: &[u8], key: &[u8]) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
        use aes_gcm::{Aes256Gcm, Nonce, KeyInit, AeadInPlace};
        
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Create cipher
        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(key);

        // Encrypt data in-place
        let mut buffer = data.to_vec();
        let tag = cipher.encrypt_in_place_detached(nonce, b"", &mut buffer)
            .map_err(|e| anyhow::anyhow!("AES-GCM encryption failed: {}", e))?;

        Ok((buffer, nonce_bytes.to_vec(), tag.to_vec()))
    }

    /// Decrypt data with AES-256-GCM
    async fn decrypt_aes_gcm(&self, encrypted_data: &[u8], nonce: &[u8], tag: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, Nonce, KeyInit, AeadInPlace, Tag};

        let nonce = Nonce::from_slice(nonce);
        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(key);
        let tag = Tag::from_slice(tag);

        // Decrypt data in-place
        let mut buffer = encrypted_data.to_vec();
        cipher.decrypt_in_place_detached(nonce, b"", &mut buffer, tag)
            .map_err(|e| anyhow::anyhow!("AES-GCM decryption failed: {}", e))?;

        Ok(buffer)
    }

    /// Encrypt data with ChaCha20-Poly1305
    async fn encrypt_chacha20(&self, data: &[u8], key: &[u8]) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
        use chacha20poly1305::{ChaCha20Poly1305, Nonce, KeyInit, AeadInPlace};

        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Create cipher
        let key = chacha20poly1305::Key::from_slice(key);
        let cipher = ChaCha20Poly1305::new(key);

        // Encrypt data in-place
        let mut buffer = data.to_vec();
        let tag = cipher.encrypt_in_place_detached(nonce, b"", &mut buffer)
            .map_err(|e| anyhow::anyhow!("ChaCha20-Poly1305 encryption failed: {}", e))?;

        Ok((buffer, nonce_bytes.to_vec(), tag.to_vec()))
    }

    /// Decrypt data with ChaCha20-Poly1305
    async fn decrypt_chacha20(&self, encrypted_data: &[u8], nonce: &[u8], tag: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        use chacha20poly1305::{ChaCha20Poly1305, Nonce, KeyInit, AeadInPlace, Tag};

        let nonce = Nonce::from_slice(nonce);
        let key = chacha20poly1305::Key::from_slice(key);
        let cipher = ChaCha20Poly1305::new(key);
        let tag = Tag::from_slice(tag);

        // Decrypt data in-place
        let mut buffer = encrypted_data.to_vec();
        cipher.decrypt_in_place_detached(nonce, b"", &mut buffer, tag)
            .map_err(|e| anyhow::anyhow!("ChaCha20-Poly1305 decryption failed: {}", e))?;

        Ok(buffer)
    }
}

#[async_trait]
impl EncryptionService for AesEncryptionService {
    async fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let (key_id, key) = self.get_active_key(&self.default_algorithm).await?;

        let (encrypted_data, iv, tag) = match self.default_algorithm {
            EncryptionAlgorithm::AES256 => {
                self.encrypt_aes_gcm(data, &key).await?
            }
            EncryptionAlgorithm::ChaCha20 => {
                self.encrypt_chacha20(data, &key).await?
            }
            _ => {
                return Err(anyhow::anyhow!("Unsupported encryption algorithm: {:?}", self.default_algorithm));
            }
        };

        let envelope = EncryptedData {
            key_id: key_id.clone(),
            algorithm: self.default_algorithm.clone(),
            iv,
            data: encrypted_data,
            tag: Some(tag),
            encrypted_at: Utc::now(),
        };

        // Update key usage
        self.update_key_usage(&key_id).await?;

        // Serialize envelope
        let serialized = serde_json::to_vec(&envelope)
            .context("Failed to serialize encrypted data envelope")?;

        Ok(serialized)
    }

    async fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        // Deserialize envelope
        let envelope: EncryptedData = serde_json::from_slice(encrypted_data)
            .context("Failed to deserialize encrypted data envelope")?;

        // Get the key used for encryption
        let storage = self.key_storage.read().await;
        let key = storage.get(&envelope.key_id)
            .ok_or_else(|| anyhow::anyhow!("Encryption key not found: {}", envelope.key_id))?;

        let tag = envelope.tag
            .ok_or_else(|| anyhow::anyhow!("Authentication tag missing from encrypted data"))?;

        let plaintext = match envelope.algorithm {
            EncryptionAlgorithm::AES256 => {
                self.decrypt_aes_gcm(&envelope.data, &envelope.iv, &tag, key).await?
            }
            EncryptionAlgorithm::ChaCha20 => {
                self.decrypt_chacha20(&envelope.data, &envelope.iv, &tag, key).await?
            }
            _ => {
                return Err(anyhow::anyhow!("Unsupported encryption algorithm: {:?}", envelope.algorithm));
            }
        };

        // Update key usage
        self.update_key_usage(&envelope.key_id).await?;

        Ok(plaintext)
    }

    async fn generate_key(&self) -> Result<Vec<u8>> {
        let key_size = match self.default_algorithm {
            EncryptionAlgorithm::AES128 => 16,
            EncryptionAlgorithm::AES256 => 32,
            EncryptionAlgorithm::ChaCha20 => 32,
            EncryptionAlgorithm::RSA2048 | EncryptionAlgorithm::RSA4096 => {
                return Err(anyhow::anyhow!("RSA key generation not implemented"));
            }
        };

        let mut key = vec![0u8; key_size];
        rand::thread_rng().fill_bytes(&mut key);
        Ok(key)
    }

    async fn rotate_key(&self) -> Result<()> {
        // Generate new key
        let new_key = self.generate_key().await?;
        let new_key_id = self.generate_key_id();

        // Mark current key as deprecated
        let mut keys = self.keys.write().await;
        if let Some(current_metadata) = keys.get_mut(&self.default_algorithm) {
            current_metadata.status = KeyStatus::Deprecated;
            current_metadata.rotated_at = Some(Utc::now());
        }

        // Add new active key
        let new_metadata = EncryptionKeyMetadata {
            key_id: new_key_id.clone(),
            algorithm: self.default_algorithm.clone(),
            created_at: Utc::now(),
            last_used_at: None,
            rotated_at: None,
            status: KeyStatus::Active,
            usage_count: 0,
        };

        keys.insert(self.default_algorithm.clone(), new_metadata);
        drop(keys);

        // Store new key
        let mut storage = self.key_storage.write().await;
        storage.insert(new_key_id, new_key);

        Ok(())
    }
}

/// RSA encryption service for asymmetric encryption
pub struct RsaEncryptionService {
    /// Key pairs by algorithm
    key_pairs: Arc<RwLock<HashMap<EncryptionAlgorithm, rsa::RsaPrivateKey>>>,
    /// Default algorithm
    default_algorithm: EncryptionAlgorithm,
}

impl RsaEncryptionService {
    /// Create a new RSA encryption service
    pub fn new(algorithm: EncryptionAlgorithm) -> Self {
        Self {
            key_pairs: Arc::new(RwLock::new(HashMap::new())),
            default_algorithm: algorithm,
        }
    }

    /// Initialize with a default key pair
    pub async fn initialize(&self) -> Result<()> {
        let key_size = match self.default_algorithm {
            EncryptionAlgorithm::RSA2048 => 2048,
            EncryptionAlgorithm::RSA4096 => 4096,
            _ => return Err(anyhow::anyhow!("Invalid RSA algorithm: {:?}", self.default_algorithm)),
        };

        let mut rng = rand::thread_rng();
        let private_key = rsa::RsaPrivateKey::new(&mut rng, key_size)
            .context("Failed to generate RSA private key")?;

        let mut key_pairs = self.key_pairs.write().await;
        key_pairs.insert(self.default_algorithm.clone(), private_key);

        Ok(())
    }
}

#[async_trait]
impl EncryptionService for RsaEncryptionService {
    async fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        use rsa::{Pkcs1v15Encrypt, RsaPublicKey};

        let key_pairs = self.key_pairs.read().await;
        let private_key = key_pairs.get(&self.default_algorithm)
            .ok_or_else(|| anyhow::anyhow!("RSA key pair not found"))?;

        let public_key = RsaPublicKey::from(private_key);
        let mut rng = rand::thread_rng();

        let encrypted_data = public_key.encrypt(&mut rng, Pkcs1v15Encrypt, data)
            .context("RSA encryption failed")?;

        Ok(encrypted_data)
    }

    async fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        use rsa::Pkcs1v15Encrypt;

        let key_pairs = self.key_pairs.read().await;
        let private_key = key_pairs.get(&self.default_algorithm)
            .ok_or_else(|| anyhow::anyhow!("RSA key pair not found"))?;

        let decrypted_data = private_key.decrypt(Pkcs1v15Encrypt, encrypted_data)
            .context("RSA decryption failed")?;

        Ok(decrypted_data)
    }

    async fn generate_key(&self) -> Result<Vec<u8>> {
        // For RSA, we return the private key in PKCS#8 format
        let key_size = match self.default_algorithm {
            EncryptionAlgorithm::RSA2048 => 2048,
            EncryptionAlgorithm::RSA4096 => 4096,
            _ => return Err(anyhow::anyhow!("Invalid RSA algorithm: {:?}", self.default_algorithm)),
        };

        let mut rng = rand::thread_rng();
        let private_key = rsa::RsaPrivateKey::new(&mut rng, key_size)
            .context("Failed to generate RSA private key")?;

        use rsa::pkcs8::EncodePrivateKey;
        let pkcs8_bytes = private_key.to_pkcs8_der()
            .context("Failed to encode RSA private key to PKCS#8")?;

        Ok(pkcs8_bytes.as_bytes().to_vec())
    }

    async fn rotate_key(&self) -> Result<()> {
        let key_size = match self.default_algorithm {
            EncryptionAlgorithm::RSA2048 => 2048,
            EncryptionAlgorithm::RSA4096 => 4096,
            _ => return Err(anyhow::anyhow!("Invalid RSA algorithm: {:?}", self.default_algorithm)),
        };

        // Generate key without holding across await
        let new_private_key = {
            let mut rng = rand::thread_rng();
            rsa::RsaPrivateKey::new(&mut rng, key_size)
                .context("Failed to generate new RSA private key")?
        };

        let mut key_pairs = self.key_pairs.write().await;
        key_pairs.insert(self.default_algorithm.clone(), new_private_key);

        Ok(())
    }
}

/// Encryption service factory
pub struct EncryptionServiceFactory;

impl EncryptionServiceFactory {
    /// Create an encryption service based on the algorithm
    pub fn create_service(algorithm: EncryptionAlgorithm) -> Box<dyn EncryptionService> {
        match algorithm {
            EncryptionAlgorithm::AES128 | EncryptionAlgorithm::AES256 | EncryptionAlgorithm::ChaCha20 => {
                Box::new(AesEncryptionService::new(algorithm))
            }
            EncryptionAlgorithm::RSA2048 | EncryptionAlgorithm::RSA4096 => {
                Box::new(RsaEncryptionService::new(algorithm))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_aes_encryption_service() {
        let service = AesEncryptionService::new(EncryptionAlgorithm::AES256);
        service.initialize().await.unwrap();

        let data = b"Hello, World!";
        let encrypted = service.encrypt(data).await.unwrap();
        let decrypted = service.decrypt(&encrypted).await.unwrap();

        assert_eq!(data, decrypted.as_slice());
    }

    #[tokio::test]
    async fn test_chacha20_encryption_service() {
        let service = AesEncryptionService::new(EncryptionAlgorithm::ChaCha20);
        service.initialize().await.unwrap();

        let data = b"Hello, ChaCha20!";
        let encrypted = service.encrypt(data).await.unwrap();
        let decrypted = service.decrypt(&encrypted).await.unwrap();

        assert_eq!(data, decrypted.as_slice());
    }

    #[tokio::test]
    async fn test_key_rotation() {
        let service = AesEncryptionService::new(EncryptionAlgorithm::AES256);
        service.initialize().await.unwrap();

        let data = b"Test data";
        let encrypted_before = service.encrypt(data).await.unwrap();
        
        // Rotate key
        service.rotate_key().await.unwrap();
        
        let encrypted_after = service.encrypt(data).await.unwrap();
        
        // Both should decrypt correctly
        let decrypted_before = service.decrypt(&encrypted_before).await.unwrap();
        let decrypted_after = service.decrypt(&encrypted_after).await.unwrap();
        
        assert_eq!(data, decrypted_before.as_slice());
        assert_eq!(data, decrypted_after.as_slice());
    }

    #[tokio::test]
    async fn test_rsa_encryption_service() {
        let service = RsaEncryptionService::new(EncryptionAlgorithm::RSA2048);
        service.initialize().await.unwrap();

        let data = b"Hello, RSA!";
        let encrypted = service.encrypt(data).await.unwrap();
        let decrypted = service.decrypt(&encrypted).await.unwrap();

        assert_eq!(data, decrypted.as_slice());
    }
}