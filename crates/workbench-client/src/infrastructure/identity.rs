//! Standard HMAC verification of the merged044 identity contract.
use crate::ports::{ClientError, Credential};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

pub fn verify_proof(
    credential: &Credential,
    nonce: &str,
    instance: &str,
    proof: &str,
) -> Result<(), ClientError> {
    if nonce.is_empty()
        || instance.is_empty()
        || proof.len() != 64
        || !proof
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(ClientError::Identity);
    }
    let mut bytes = [0u8; 32];
    for (index, pair) in proof.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let digit = |b: u8| {
            if b.is_ascii_digit() {
                b - b'0'
            } else {
                b - b'a' + 10
            }
        };
        bytes[index] = digit(pair[0]) * 16 + digit(pair[1]);
    }
    let digest = Sha256::digest(credential.expose().as_bytes());
    let mut mac = Hmac::<Sha256>::new_from_slice(&digest).map_err(|_| ClientError::Identity)?;
    mac.update(nonce.as_bytes());
    mac.update(b"\n");
    mac.update(instance.as_bytes());
    mac.verify_slice(&bytes).map_err(|_| ClientError::Identity)
}
