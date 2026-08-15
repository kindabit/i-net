use aes_gcm_siv::aead::{Aead, KeyInit};
use aes_gcm_siv::{Aes256GcmSiv, Key, Nonce};

use crate::error_code::ErrorCode;
use crate::util::time_util;

/// 使用 AES-256-GCM-SIV 加密明文数据。
///
/// 加密结果的前 12 字节为随机生成的 nonce，剩余部分为密文。
///
/// # 参数
///
/// * `plaintext` - 需要加密的明文数据。
/// * `key` - 32 字节的加密密钥。
///
/// # 返回值
///
/// 返回包含 nonce 和密文的字节数组；若加密失败则返回 `ErrorCode::FailToEncrypt`。
pub fn encrypt(plaintext: Vec<u8>, key: [u8; 32]) -> Result<Vec<u8>, ErrorCode> {
    let key = Key::<Aes256GcmSiv>::from(key);
    let cipher = Aes256GcmSiv::new(&key);

    let nonce_bytes = construct_nonce();
    let nonce = Nonce::from(nonce_bytes);

    let ciphertext =
        cipher
            .encrypt(&nonce, plaintext.as_ref())
            .map_err(|e| ErrorCode::FailToEncrypt {
                detail: e.to_string(),
            })?;

    let mut result = Vec::with_capacity(12 + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend(ciphertext);
    Ok(result)
}

/// 使用 AES-256-GCM-SIV 解密密文数据。
///
/// 输入数据的前 12 字节被解析为 nonce，剩余部分为密文。
///
/// # 参数
///
/// * `ciphertext_with_nonce` - 包含 nonce 前缀的密文数据。
/// * `key` - 32 字节的解密密钥。
///
/// # 返回值
///
/// 返回解密后的明文数据；若密文无效则返回 `ErrorCode::InvalidCiphertext`，
/// 若解密失败则返回 `ErrorCode::FailToDecrypt`。
pub fn decrypt(ciphertext_with_nonce: Vec<u8>, key: [u8; 32]) -> Result<Vec<u8>, ErrorCode> {
    if ciphertext_with_nonce.len() <= 12 {
        return Err(ErrorCode::InvalidCiphertext);
    }

    let key = Key::<Aes256GcmSiv>::from(key);
    let cipher = Aes256GcmSiv::new(&key);

    let mut nonce_bytes = [0u8; 12];
    nonce_bytes.copy_from_slice(&ciphertext_with_nonce[..12]);
    let nonce = Nonce::from(nonce_bytes);

    let ciphertext = &ciphertext_with_nonce[12..];

    cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|e| ErrorCode::FailToDecrypt {
            detail: e.to_string(),
        })
}

/// 构造 12 字节的 nonce。
///
/// nonce 由当前时间戳和前 4 字节随机数组合而成。
///
/// # 返回值
///
/// 返回 12 字节的 nonce 数组。
fn construct_nonce() -> [u8; 12] {
    let timestamp = time_util::now();
    let random: i32 = rand::random();
    let mut nonce = [0u8; 12];
    nonce[..8].copy_from_slice(&timestamp.to_le_bytes());
    nonce[8..].copy_from_slice(&random.to_le_bytes());
    nonce
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 覆盖 security::aes 模块加密与解密函数的成功与失败路径。
    #[test]
    fn test_aes_all_functions() {
        let key = [0u8; 32];
        let wrong_key = [1u8; 32];
        let plaintext = b"Hello world".to_vec();

        // encrypt 成功路径：密文非空。
        let ciphertext = encrypt(plaintext.clone(), key).unwrap();
        assert!(!ciphertext.is_empty());

        // decrypt 成功路径：解密后明文与原始明文一致。
        let decrypted = decrypt(ciphertext.clone(), key).unwrap();
        assert_eq!(decrypted, plaintext);

        // decrypt 失败路径：使用错误密钥解密返回 FailToDecrypt。
        let result = decrypt(ciphertext.clone(), wrong_key);
        assert!(matches!(result, Err(ErrorCode::FailToDecrypt { .. })));

        // decrypt 失败路径：密文长度不足（<=12 字节）返回 InvalidCiphertext。
        assert!(matches!(
            decrypt(vec![0u8; 12], key),
            Err(ErrorCode::InvalidCiphertext)
        ));
        assert!(matches!(
            decrypt(vec![0u8; 5], key),
            Err(ErrorCode::InvalidCiphertext)
        ));

        // encrypt 属性：相同明文使用不同密钥产生不同密文。
        let same_plaintext = b"same plaintext".to_vec();
        let ciphertext_a = encrypt(same_plaintext.clone(), key).unwrap();
        let ciphertext_b = encrypt(same_plaintext.clone(), wrong_key).unwrap();
        assert_ne!(ciphertext_a, ciphertext_b);

        // encrypt 属性：相同明文和密钥两次加密结果不同（nonce 随机化）。
        let ciphertext_c = encrypt(same_plaintext.clone(), key).unwrap();
        let ciphertext_d = encrypt(same_plaintext, key).unwrap();
        assert_ne!(ciphertext_c, ciphertext_d);
    }
}
