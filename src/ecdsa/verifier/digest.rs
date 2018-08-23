use core::fmt::Debug;
use digest::Digest;

use super::RawDigestVerifier;
use curve::WeierstrassCurve;
use ecdsa::{Asn1Signature, FixedSignature, PublicKey};
use error::Error;

/// Verifier for ECDSA signatures which takes a precomputed  `digest::Digest`,
/// whose output size must be equal to the modulus of the curve.
pub trait DigestVerifier<C, D>: Clone + Debug + Eq + PartialEq + Send + Sync
where
    C: WeierstrassCurve,
    D: Digest<OutputSize = C::ScalarSize> + Default,
{
    /// Verify an ASN.1 DER-encoded ECDSA signature for a given pre-hashed
    /// message `Digest` using the given public key.
    fn verify_digest_asn1_signature(
        key: &PublicKey<C>,
        digest: D,
        signature: &Asn1Signature<C>,
    ) -> Result<(), Error> {
        Self::verify_digest_fixed_signature(key, digest, &FixedSignature::from(signature))
    }

    /// Verify a fixed-sized (a.k.a. "compact") ECDSA signature for a given
    /// pre-hashed message using the given public key
    fn verify_digest_fixed_signature(
        key: &PublicKey<C>,
        digest: D,
        signature: &FixedSignature<C>,
    ) -> Result<(), Error> {
        Self::verify_digest_asn1_signature(key, digest, &Asn1Signature::from(signature))
    }
}

impl<C, D, V> DigestVerifier<C, D> for V
where
    C: WeierstrassCurve,
    D: Digest<OutputSize = C::ScalarSize> + Default,
    V: RawDigestVerifier<C>,
{
    fn verify_digest_asn1_signature(
        key: &PublicKey<C>,
        digest: D,
        signature: &Asn1Signature<C>,
    ) -> Result<(), Error> {
        Self::verify_raw_digest_asn1_signature(key, &digest.fixed_result(), signature)
    }

    fn verify_digest_fixed_signature(
        key: &PublicKey<C>,
        digest: D,
        signature: &FixedSignature<C>,
    ) -> Result<(), Error> {
        Self::verify_raw_digest_fixed_signature(key, &digest.fixed_result(), signature)
    }
}
