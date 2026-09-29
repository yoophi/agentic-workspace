use workbench_client::{infrastructure::identity::verify_proof, ports::Credential};
const TOKEN: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";
const NONCE: &str = "3f2c9a1e7b6d4c5a8e9f0a1b2c3d4e5f";
const INSTANCE: &str = "6f1a2b3c-4d5e-4f60-8a7b-9c0d1e2f3a4b";
const PROOF: &str = "ec5b0b7d634e793e9ee94819c6219a3bd9df033afd49cb30d302f6076f6ab079";
#[test]
fn merged044_host_literal_vector_is_verified() {
    verify_proof(
        &Credential::new(TOKEN.into()).unwrap(),
        NONCE,
        INSTANCE,
        PROOF,
    )
    .unwrap();
}
#[test]
fn proof_rejects_wrong_nonce_instance_token_hex_length_and_encoding() {
    let credential = Credential::new(TOKEN.into()).unwrap();
    for proof in ["", "00", &PROOF.to_uppercase(), &"z".repeat(64)] {
        assert!(verify_proof(&credential, NONCE, INSTANCE, proof).is_err());
    }
    assert!(verify_proof(&credential, "other", INSTANCE, PROOF).is_err());
    assert!(verify_proof(&credential, NONCE, "other", PROOF).is_err());
    assert!(verify_proof(
        &Credential::new("other".into()).unwrap(),
        NONCE,
        INSTANCE,
        PROOF
    )
    .is_err());
}
#[test]
fn credential_rejects_header_injection_and_debug_redacts() {
    for token in ["", "token\r\nAuthorization: forged", "a b", "유니코드"] {
        assert!(Credential::new(token.into()).is_err());
    }
    assert!(!format!("{:?}", Credential::new(TOKEN.into()).unwrap()).contains(TOKEN));
}
