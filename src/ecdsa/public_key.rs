//! ECDSA public keys: compressed or uncompressed Weierstrass elliptic
//! curve points.

#[cfg(feature = "encoding")]
use crate::encoding::Decode;
use crate::{
    ecdsa::curve::{
        point::{CompressedCurvePoint, UncompressedCurvePoint},
        WeierstrassCurve,
    },
    util::fmt_colon_delimited_hex,
};
#[cfg(all(feature = "alloc", feature = "encoding"))]
use crate::{encoding::Encode, prelude::*};
use core::fmt::{self, Debug};
use generic_array::{typenum::Unsigned, ArrayLength, GenericArray};
#[cfg(feature = "encoding")]
use subtle_encoding::Encoding;

/// ECDSA public keys
#[derive(Clone, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub enum PublicKey<C: WeierstrassCurve> {
    /// Compressed Weierstrass elliptic curve point
    Compressed(CompressedCurvePoint<C>),

    /// Uncompressed Weierstrass elliptic curve point
    Uncompressed(UncompressedCurvePoint<C>),
}

impl<C> PublicKey<C>
where
    C: WeierstrassCurve,
{
    /// Create an ECDSA public key from an elliptic curve point
    /// (compressed or uncompressed) encoded using the
    /// `Elliptic-Curve-Point-to-Octet-String` algorithm described in
    /// SEC 1: Elliptic Curve Cryptography (Version 2.0) section
    /// 2.3.3 (page 10).
    ///
    /// <http://www.secg.org/sec1-v2.pdf>
    pub fn from_bytes<B: AsRef<[u8]>>(bytes: B) -> Option<Self> {
        let slice = bytes.as_ref();
        let length = slice.len();

        if length == C::CompressedPointSize::to_usize() {
            let array = GenericArray::clone_from_slice(slice);
            let point = CompressedCurvePoint::from_bytes(array)?;
            Some(PublicKey::Compressed(point))
        } else if length == C::UncompressedPointSize::to_usize() {
            let array = GenericArray::clone_from_slice(slice);
            let point = UncompressedCurvePoint::from_bytes(array)?;
            Some(PublicKey::Uncompressed(point))
        } else {
            None
        }
    }

    /// Create an ECDSA public key from an compressed elliptic curve point
    /// encoded using the `Elliptic-Curve-Point-to-Octet-String` algorithm
    /// described in SEC 1: Elliptic Curve Cryptography (Version 2.0) section
    /// 2.3.3 (page 10).
    ///
    /// <http://www.secg.org/sec1-v2.pdf>
    pub fn from_compressed_point<B>(into_bytes: B) -> Option<Self>
    where
        B: Into<GenericArray<u8, C::CompressedPointSize>>,
    {
        CompressedCurvePoint::from_bytes(into_bytes).map(PublicKey::Compressed)
    }

    /// Create an ECDSA public key from a raw uncompressed point serialized
    /// as a bytestring, without a `0x04`-byte tag.
    ///
    /// This will be twice the modulus size, or 1-byte smaller than the
    /// `Elliptic-Curve-Point-to-Octet-String` encoding i.e
    /// with the leading `0x04` byte in that encoding removed.
    pub fn from_untagged_point(bytes: &GenericArray<u8, C::UntaggedPointSize>) -> Self {
        let mut tagged_bytes = GenericArray::default();
        tagged_bytes.as_mut_slice()[0] = 0x04;
        tagged_bytes.as_mut_slice()[1..].copy_from_slice(bytes.as_ref());

        PublicKey::Uncompressed(UncompressedCurvePoint::from_bytes(tagged_bytes).unwrap())
    }

    /// Obtain public key as a byte array reference
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            PublicKey::Compressed(ref point) => point.as_bytes(),
            PublicKey::Uncompressed(ref point) => point.as_bytes(),
        }
    }
}

impl<C> AsRef<[u8]> for PublicKey<C>
where
    C: WeierstrassCurve,
{
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<C> Copy for PublicKey<C>
where
    C: WeierstrassCurve,
    <<C as WeierstrassCurve>::CompressedPointSize as ArrayLength<u8>>::ArrayType: Copy,
    <<C as WeierstrassCurve>::UncompressedPointSize as ArrayLength<u8>>::ArrayType: Copy,
{
}

impl<C> Debug for PublicKey<C>
where
    C: WeierstrassCurve,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "signatory::ecdsa::PublicKey<{:?}>(", C::default())?;
        fmt_colon_delimited_hex(f, self.as_ref())?;
        write!(f, ")")
    }
}

#[cfg(feature = "encoding")]
impl<C> Decode for PublicKey<C>
where
    C: WeierstrassCurve,
{
    /// Decode an ECDSA public key from an elliptic curve point
    /// (compressed or uncompressed) encoded using given `Encoding`
    /// with the underlying bytes serialized using the
    /// `Elliptic-Curve-Point-to-Octet-String` algorithm described in
    /// SEC 1: Elliptic Curve Cryptography (Version 2.0) section
    /// 2.3.3 (page 10).
    ///
    /// <http://www.secg.org/sec1-v2.pdf>
    fn decode<E: Encoding>(
        encoded_signature: &[u8],
        encoding: &E,
    ) -> Result<Self, crate::encoding::Error> {
        let mut array: GenericArray<u8, C::UncompressedPointSize> = GenericArray::default();

        let decoded_len = encoding.decode_to_slice(encoded_signature, array.as_mut_slice())?;

        Self::from_bytes(&array.as_ref()[..decoded_len])
            .ok_or_else(|| crate::encoding::error::ErrorKind::Decode.into())
    }
}

#[cfg(all(feature = "encoding", feature = "alloc"))]
impl<C> Encode for PublicKey<C>
where
    C: WeierstrassCurve,
{
    /// Encode this ECDSA public key (compressed or uncompressed) encoded
    /// using given `Encoding` with the underlying bytes serialized using the
    /// `Elliptic-Curve-Point-to-Octet-String` algorithm described in
    /// SEC 1: Elliptic Curve Cryptography (Version 2.0) section
    /// 2.3.3 (page 10).
    ///
    /// <http://www.secg.org/sec1-v2.pdf>
    fn encode<E: Encoding>(&self, encoding: &E) -> Vec<u8> {
        encoding.encode(self.as_ref())
    }
}

impl<C: WeierstrassCurve> crate::public_key::PublicKey for PublicKey<C> {}
