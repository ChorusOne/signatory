//! ECDSA provider for the `secp256k1` crate (a.k.a. secp256k1-rs)

#![crate_name = "signatory_secp256k1"]
#![crate_type = "lib"]
#![deny(warnings, missing_docs, trivial_casts, trivial_numeric_casts)]
#![deny(unsafe_code, unused_import_braces, unused_qualifications)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/tendermint/signatory/master/img/signatory-rustacean.png",
    html_root_url = "https://docs.rs/signatory-secp256k1/0.10.0"
)]

#[macro_use]
extern crate lazy_static;
extern crate secp256k1;
extern crate signatory;

use signatory::{
    curve::secp256k1::{Asn1Signature, FixedSignature, PublicKey, SecretKey},
    digest::Digest,
    generic_array::typenum::U32,
    DigestSigner, DigestVerifier, Error, PublicKeyed, Signature,
};

lazy_static! {
    /// Lazily initialized secp256k1 engine
    static ref SECP256K1_ENGINE: secp256k1::Secp256k1<secp256k1::All> = secp256k1::Secp256k1::new();
}

/// Create a new error (of a given enum variant) with a formatted message
macro_rules! err {
    ($variant:ident, $msg:expr) => {{
        ::signatory::error::Error::new(
            ::signatory::error::ErrorKind::$variant,
            Some(&format!("{}", $msg)),
        )
    }};
}

/// Create and return an error with a formatted message
#[allow(unused_macros)]
macro_rules! fail {
    ($kind:ident, $msg:expr) => {
        return Err(err!($kind, $msg).into());
    };
}

/// ECDSA signature provider for the secp256k1 crate
pub struct EcdsaSigner(secp256k1::key::SecretKey);

impl<'a> From<&'a SecretKey> for EcdsaSigner {
    /// Create a new secp256k1 signer from the given `SecretKey`
    fn from(secret_key: &'a SecretKey) -> EcdsaSigner {
        let sk =
            secp256k1::key::SecretKey::from_slice(&SECP256K1_ENGINE, secret_key.as_secret_slice())
                .unwrap();

        EcdsaSigner(sk)
    }
}

impl PublicKeyed<PublicKey> for EcdsaSigner {
    /// Return the public key that corresponds to the private key for this signer
    fn public_key(&self) -> Result<PublicKey, Error> {
        let pk = secp256k1::key::PublicKey::from_secret_key(&SECP256K1_ENGINE, &self.0);
        PublicKey::from_bytes(&pk.serialize()[..])
    }
}

impl<D> DigestSigner<D, Asn1Signature> for EcdsaSigner
where
    D: Digest<OutputSize = U32> + Default,
{
    /// Compute an ASN.1 DER-encoded signature of the given 32-byte SHA-256 digest
    fn sign(&self, digest: D) -> Result<Asn1Signature, Error> {
        let m = secp256k1::Message::from_slice(digest.result().as_slice()).unwrap();
        let sig = SECP256K1_ENGINE.sign(&m, &self.0);
        Ok(Asn1Signature::from_bytes(sig.serialize_der(&SECP256K1_ENGINE)).unwrap())
    }
}

impl<D> DigestSigner<D, FixedSignature> for EcdsaSigner
where
    D: Digest<OutputSize = U32> + Default,
{
    /// Compute a compact, fixed-sized signature of the given 32-byte SHA-256 digest
    fn sign(&self, digest: D) -> Result<FixedSignature, Error> {
        let m = secp256k1::Message::from_slice(digest.result().as_slice()).unwrap();
        let sig = SECP256K1_ENGINE.sign(&m, &self.0);
        Ok(FixedSignature::from_bytes(&sig.serialize_compact(&SECP256K1_ENGINE)[..]).unwrap())
    }
}

/// ECDSA verifier provider for the secp256k1 crate
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EcdsaVerifier(secp256k1::key::PublicKey);

impl<'a> From<&'a PublicKey> for EcdsaVerifier {
    fn from(public_key: &'a PublicKey) -> Self {
        EcdsaVerifier(
            secp256k1::key::PublicKey::from_slice(&SECP256K1_ENGINE, public_key.as_bytes())
                .unwrap(),
        )
    }
}

impl<D> DigestVerifier<D, Asn1Signature> for EcdsaVerifier
where
    D: Digest<OutputSize = U32> + Default,
{
    fn verify(&self, digest: D, signature: &Asn1Signature) -> Result<(), Error> {
        let sig = secp256k1::Signature::from_der(&SECP256K1_ENGINE, signature.as_slice())
            .map_err(|e| err!(SignatureInvalid, e))?;

        SECP256K1_ENGINE
            .verify(
                &secp256k1::Message::from_slice(digest.result().as_slice()).unwrap(),
                &sig,
                &self.0,
            ).map_err(|e| err!(SignatureInvalid, e))
    }
}

impl<D> DigestVerifier<D, FixedSignature> for EcdsaVerifier
where
    D: Digest<OutputSize = U32> + Default,
{
    fn verify(&self, digest: D, signature: &FixedSignature) -> Result<(), Error> {
        let sig =
            secp256k1::Signature::from_compact(&SECP256K1_ENGINE, signature.as_slice()).unwrap();

        SECP256K1_ENGINE
            .verify(
                &secp256k1::Message::from_slice(digest.result().as_slice()).unwrap(),
                &sig,
                &self.0,
            ).map_err(|e| err!(SignatureInvalid, e))
    }
}

// TODO: test against actual test vectors, rather than just checking if signatures roundtrip
#[cfg(test)]
mod tests {
    use super::{EcdsaSigner, EcdsaVerifier};
    use signatory::{
        self,
        curve::secp256k1::{
            Asn1Signature, FixedSignature, PublicKey, SecretKey, SHA256_FIXED_SIZE_TEST_VECTORS,
        },
        PublicKeyed, Sha256Verifier, Signature,
    };

    #[test]
    pub fn asn1_signature_roundtrip() {
        let vector = &SHA256_FIXED_SIZE_TEST_VECTORS[0];

        let signer = EcdsaSigner::from(&SecretKey::from_bytes(vector.sk).unwrap());
        let signature: Asn1Signature = signatory::sign_sha256(&signer, vector.msg).unwrap();

        let verifier = EcdsaVerifier::from(&signer.public_key().unwrap());
        assert!(verifier.verify_sha256(vector.msg, &signature).is_ok());
    }

    #[test]
    pub fn rejects_tweaked_asn1_signature() {
        let vector = &SHA256_FIXED_SIZE_TEST_VECTORS[0];

        let signer = EcdsaSigner::from(&SecretKey::from_bytes(vector.sk).unwrap());
        let signature: Asn1Signature = signatory::sign_sha256(&signer, vector.msg).unwrap();
        let mut tweaked_signature = signature.into_vec();
        *tweaked_signature.iter_mut().last().unwrap() ^= 42;

        let verifier = EcdsaVerifier::from(&signer.public_key().unwrap());
        let result = verifier.verify_sha256(
            vector.msg,
            &Asn1Signature::from_bytes(tweaked_signature).unwrap(),
        );

        assert!(
            result.is_err(),
            "expected bad signature to cause validation error!"
        );
    }

    #[test]
    pub fn fixed_signature_vectors() {
        for vector in SHA256_FIXED_SIZE_TEST_VECTORS {
            let signer = EcdsaSigner::from(&SecretKey::from_bytes(vector.sk).unwrap());
            let public_key = PublicKey::from_bytes(vector.pk).unwrap();
            assert_eq!(signer.public_key().unwrap(), public_key);

            let signature: FixedSignature = signatory::sign_sha256(&signer, vector.msg).unwrap();
            assert_eq!(signature.as_ref(), vector.sig);

            EcdsaVerifier::from(&public_key)
                .verify_sha256(vector.msg, &signature)
                .unwrap();
        }
    }

    #[test]
    pub fn rejects_tweaked_fixed_signature() {
        let vector = &SHA256_FIXED_SIZE_TEST_VECTORS[0];

        let signer = EcdsaSigner::from(&SecretKey::from_bytes(vector.sk).unwrap());
        let signature: FixedSignature = signatory::sign_sha256(&signer, vector.msg).unwrap();
        let mut tweaked_signature = signature.into_vec();
        *tweaked_signature.iter_mut().last().unwrap() ^= 42;

        let verifier = EcdsaVerifier::from(&signer.public_key().unwrap());
        let result = verifier.verify_sha256(
            vector.msg,
            &FixedSignature::from_bytes(tweaked_signature).unwrap(),
        );

        assert!(
            result.is_err(),
            "expected bad signature to cause validation error!"
        );
    }
}
