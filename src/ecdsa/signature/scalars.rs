//! An ECDSA signature comprises 2 scalars: `r` and `s`. The scalars are
//! the same size as the curve's modulus, i.e. for an elliptic curve over
//! a ~256-bit prime field, they will also be 256-bit (i.e. the `ScalarSize`
//! for a particular `WeierstrassCurve`)
//!
//! This type provides a convenient representation for converting between
//! formats, i.e. all of the serialization code is in this module.

use core::marker::PhantomData;
use generic_array::{typenum::Unsigned, GenericArray};

use super::asn1::Asn1Signature;
use super::fixed::FixedSignature;
use curve::WeierstrassCurve;
use encoding::asn1;
use error::Error;
use signature::Signature;

/// ECDSA signature `r` and `s` values, represented as slices which are at
/// most `C::ScalarSize` bytes (but *may* be smaller)
pub(crate) struct ScalarPair<'a, C: WeierstrassCurve> {
    /// `r` scalar value
    r: &'a [u8],

    /// `s` scalar value
    s: &'a [u8],

    /// Placeholder for elliptic curve type
    curve: PhantomData<C>,
}

impl<'a, C> ScalarPair<'a, C>
where
    C: WeierstrassCurve,
{
    /// Parse the given ASN.1 DER-encoded ECDSA signature, obtaining the
    /// `r` and `s` scalar pair
    pub(crate) fn from_asn1_signature(signature: &'a Asn1Signature<C>) -> Result<Self, Error> {
        // Signature format is a SEQUENCE of two INTEGER values. We
        // support only integers of less than 127 bytes each (signed
        // encoding) so the resulting raw signature will have length
        // at most 254 bytes.
        let mut bytes = signature.as_slice();

        // First byte is SEQUENCE tag.
        ensure!(
            bytes[0] == asn1::Tag::Sequence as u8,
            ParseError,
            "ASN.1 error: expected first byte to be a SEQUENCE tag: {}",
            bytes[0]
        );

        // The SEQUENCE length will be encoded over one or two bytes. We
        // limit the total SEQUENCE contents to 255 bytes, because it
        // makes things simpler; this is enough for subgroup orders up
        // to 999 bits.
        let mut zlen = bytes[1] as usize;

        if zlen > 0x80 {
            ensure!(
                zlen == 0x81,
                ParseError,
                "ASN.1 error: overlength signature: {}",
                zlen
            );

            zlen = bytes[2] as usize;
            ensure!(
                zlen == bytes.len().checked_sub(3).unwrap(),
                ParseError,
                "ASN.1 error: sequence length mismatch ({} vs {})",
                zlen,
                bytes.len().checked_sub(3).unwrap()
            );

            bytes = &bytes[3..];
        } else {
            ensure!(
                zlen == bytes.len().checked_sub(2).unwrap(),
                ParseError,
                "ASN.1 error: sequence length mismatch ({} vs {})",
                zlen,
                bytes.len().checked_sub(2).unwrap()
            );

            bytes = &bytes[2..];
        };

        // First INTEGER (r)
        let (mut r, bytes) = Self::asn1_int_parse(bytes)?;

        // Second INTEGER (s)
        let (mut s, bytes) = Self::asn1_int_parse(bytes)?;

        ensure!(
            bytes.is_empty(),
            ParseError,
            "ASN.1 error: trailing data at end of signature"
        );

        let scalar_size = C::ScalarSize::to_usize();

        if r.len() > scalar_size {
            ensure!(
                r.len() == scalar_size.checked_add(1).unwrap(),
                ParseError,
                "ASN.1 error: overlong 'r'"
            );

            ensure!(
                r[0] == 0,
                ParseError,
                "ASN.1 error: expected leading 0 on 'r'"
            );

            r = &r[1..];
        }

        if s.len() > scalar_size {
            ensure!(
                s.len() == scalar_size.checked_add(1).unwrap(),
                ParseError,
                "ASN.1 error: overlong 's'"
            );

            ensure!(
                s[0] == 0,
                ParseError,
                "ASN.1 error: expected leading 0 on 's'"
            );

            s = &s[1..];
        }

        // Removing leading zeros from r and s

        while !r.is_empty() && r[0] == 0 {
            r = &r[1..];
        }

        while !s.is_empty() && s[0] == 0 {
            s = &s[1..];
        }

        Ok(Self {
            r,
            s,
            curve: PhantomData,
        })
    }

    /// Parse the given fixed-size ECDSA signature, obtaining the `r` and `s`
    /// scalar pair
    pub(crate) fn from_fixed_signature(signature: &'a FixedSignature<C>) -> Self {
        let scalar_size = C::ScalarSize::to_usize();

        Self {
            r: &signature.as_ref()[..scalar_size],
            s: &signature.as_ref()[scalar_size..],
            curve: PhantomData,
        }
    }

    /// Serialize this ECDSA signature's `r` and `s` scalar pair as ASN.1 DER
    pub(crate) fn to_asn1_signature(&self) -> Asn1Signature<C> {
        let rlen = Self::asn1_int_length(self.r);
        let slen = Self::asn1_int_length(self.s);
        let mut bytes = GenericArray::default();

        // SEQUENCE header
        bytes[0] = asn1::Tag::Sequence as u8;
        let zlen = rlen.checked_add(slen).unwrap().checked_add(4).unwrap();

        let mut offset = if zlen >= 0x80 {
            bytes[1] = 0x81;
            bytes[2] = zlen as u8;
            3
        } else {
            bytes[1] = zlen as u8;
            2
        };

        // First INTEGER (r)
        Self::asn1_int_serialize(self.r, &mut bytes[offset..], rlen);
        offset = offset.checked_add(2).unwrap().checked_add(rlen).unwrap();

        // Second INTEGER (s)
        Self::asn1_int_serialize(self.s, &mut bytes[offset..], slen);

        let result = Asn1Signature {
            bytes,
            length: offset.checked_add(2).unwrap().checked_add(slen).unwrap(),
            curve: PhantomData,
        };

        // Double-check we produced an ASN.1 signature we can parse ourselves
        #[cfg(debug_assertions)]
        Self::from_asn1_signature(&result).unwrap();

        result
    }

    pub(crate) fn to_fixed_signature(&self) -> FixedSignature<C> {
        let mut bytes = GenericArray::default();

        let scalar_size = C::ScalarSize::to_usize();
        let rbegin = scalar_size.checked_sub(self.r.len()).unwrap();
        bytes.as_mut_slice()[rbegin..scalar_size].copy_from_slice(self.r);

        let sbegin = bytes.len().checked_sub(self.s.len()).unwrap();
        bytes.as_mut_slice()[sbegin..].copy_from_slice(self.s);

        FixedSignature::from(bytes)
    }

    /// Compute ASN.1 DER encoded length for the provided scalar. The ASN.1
    /// encoding is signed, so its leading bit must have value 0; it must also be
    /// of minimal length (so leading bytes of value 0 must be removed, except if
    /// that would contradict the rule about the sign bit).
    fn asn1_int_length(mut x: &[u8]) -> usize {
        while !x.is_empty() && x[0] == 0 {
            x = &x[1..];
        }

        if x.is_empty() || x[0] >= 0x80 {
            x.len().checked_add(1).unwrap()
        } else {
            x.len()
        }
    }

    /// Parse an integer from its ASN.1 DER serialization
    fn asn1_int_parse(bytes: &[u8]) -> Result<(&[u8], &[u8]), Error> {
        ensure!(
            bytes.len() >= 3,
            ParseError,
            "ASN.1 error: truncated INTEGER"
        );

        ensure!(
            bytes[0] == asn1::Tag::Integer as u8,
            ParseError,
            "ASN.1 error: expected INTEGER tag (0x02) (got 0x{:x})",
            bytes[0]
        );

        let len = bytes[1] as usize;

        ensure!(
            len < 0x80 && len.checked_add(2).unwrap() <= bytes.len(),
            ParseError,
            "ASN.1 error: unexpected length for INTEGER: {}",
            len
        );

        let integer = &bytes[2..len.checked_add(2).unwrap()];
        let remaining = &bytes[len.checked_add(2).unwrap()..];

        Ok((integer, remaining))
    }

    /// Serialize scalar as ASN.1 DER
    fn asn1_int_serialize(scalar: &[u8], out: &mut [u8], len: usize) {
        out[0] = asn1::Tag::Integer as u8;
        out[1] = len as u8;

        if len > C::ScalarSize::to_usize() {
            out[2] = 0x00;
            out[3..C::ScalarSize::to_usize().checked_add(3).unwrap()].copy_from_slice(scalar);
        } else {
            out[2..len.checked_add(2).unwrap()]
                .copy_from_slice(&scalar[C::ScalarSize::to_usize().checked_sub(len).unwrap()..]);
        }
    }
}
