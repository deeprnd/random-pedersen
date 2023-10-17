use bulletproofs::PedersenGens;
use byteorder::{ByteOrder, LittleEndian};
use curve25519_dalek_ng::{
    ristretto::{CompressedRistretto, RistrettoPoint},
    scalar::Scalar,
};
use once_cell::sync::Lazy;
use std::ops;

use super::random::generate_random;

const RANDOM_LENGTH: usize = 32;

static PEDERSEN_GENS: Lazy<PedersenGens> = Lazy::new(PedersenGens::default);

/// Pedersen commitment to an integer value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commitment {
    pub inner: RistrettoPoint,
}

impl Commitment {
    /// Size of the byte representation of the commitment (i.e., a compressed Ristretto point).
    pub(crate) const BYTE_LEN: usize = 32;

    /// Creates a commitment with a randomly chosen blinding.
    pub fn new(value: u64) -> (Self, Opening) {
        let random = generate_random(RANDOM_LENGTH).unwrap();
        let mut arr = [0; RANDOM_LENGTH];
        arr.copy_from_slice(&random[0..RANDOM_LENGTH]);
        let blinding = Scalar::from_bytes_mod_order(arr);

        let opening = Opening::new(value, blinding);
        (Self::from_opening(&opening), opening)
    }

    /// Creates a commitment from the given opening.
    pub fn from_opening(opening: &Opening) -> Self {
        let inner = PEDERSEN_GENS.commit(Scalar::from(opening.value), opening.blinding);
        Commitment { inner }
    }

    /// Attempts to deserialize a commitment from byte slice.
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != Self::BYTE_LEN {
            return None;
        }

        let compressed_point = CompressedRistretto::from_slice(slice);
        compressed_point
            .decompress()
            .map(|point| Commitment { inner: point })
    }

    /// Serializes this commitment to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.inner.compress().as_bytes().to_vec()
    }

    /// Verifies if this commitment corresponds to the provided opening.
    pub fn verify(&self, opening: &Opening) -> bool {
        *self == Self::from_opening(opening)
    }
}

impl ops::Add for Commitment {
    type Output = Commitment;

    fn add(self, rhs: Self) -> Commitment {
        Commitment {
            inner: self.inner + rhs.inner,
        }
    }
}

impl<'a, 'b> ops::Add<&'b Commitment> for &'a Commitment {
    type Output = Commitment;

    fn add(self, rhs: &'b Commitment) -> Commitment {
        Commitment {
            inner: self.inner + rhs.inner,
        }
    }
}

impl ops::Sub for Commitment {
    type Output = Commitment;

    fn sub(self, rhs: Self) -> Commitment {
        Commitment {
            inner: self.inner - rhs.inner,
        }
    }
}

impl<'a, 'b> ops::Sub<&'b Commitment> for &'a Commitment {
    type Output = Commitment;

    fn sub(self, rhs: &'b Commitment) -> Commitment {
        Commitment {
            inner: self.inner - rhs.inner,
        }
    }
}

/// Opening for a Pedersen commitment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opening {
    /// Committed value.
    pub value: u64,
    blinding: Scalar,
}

impl Opening {
    /// Size of a serialized opening.
    const BYTE_SIZE: usize = 40;

    pub(crate) fn new(value: u64, blinding: Scalar) -> Self {
        Opening { value, blinding }
    }

    /// Attempts to deserialize an opening from a slice.
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != Self::BYTE_SIZE {
            return None;
        }

        let mut scalar_bytes = [0_u8; 32];
        scalar_bytes.copy_from_slice(&slice[8..]);
        Some(Opening {
            value: LittleEndian::read_u64(&slice[..8]),
            blinding: Scalar::from_canonical_bytes(scalar_bytes).unwrap(),
        })
    }

    /// Serializes to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = [0_u8; Self::BYTE_SIZE];
        LittleEndian::write_u64(&mut bytes[0..8], self.value);
        bytes[8..].copy_from_slice(&*self.blinding.as_bytes());
        bytes.to_vec()
    }
}

impl ops::Add for Opening {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Opening {
            value: self.value.checked_add(rhs.value).expect("integer overflow"),
            blinding: self.blinding + rhs.blinding,
        }
    }
}

impl ops::Sub for Opening {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Opening {
            value: self
                .value
                .checked_sub(rhs.value)
                .expect("integer underflow"),
            blinding: self.blinding - rhs.blinding,
        }
    }
}

impl<'a, 'b> ops::Sub<&'b Opening> for &'a Opening {
    type Output = Opening;

    fn sub(self, rhs: &'b Opening) -> Opening {
        Opening {
            value: self
                .value
                .checked_sub(rhs.value)
                .expect("integer underflow"),
            blinding: self.blinding - rhs.blinding,
        }
    }
}

#[test]
fn commitment_arithmetic() {
    let (comm1, opening1) = Commitment::new(100);
    let (comm2, opening2) = Commitment::new(200);
    assert!((comm1 + comm2).verify(&(opening1 + opening2)));

    let (comm1, opening1) = Commitment::new(1234);
    let (comm2, opening2) = Commitment::new(234);
    assert!((comm1 - comm2).verify(&(opening1 - opening2)));
}

#[test]
fn opening_recovery_is_as_expected() {
    let value = 1234;
    let (commitment, opening) = Commitment::new(value);

    let opening_vec = opening.to_bytes();

    let opening_bytes: &[u8] = &opening_vec;
    let open = Opening::from_slice(&opening_bytes).unwrap();
    let commit_from_open = Commitment::from_opening(&open);

    assert_eq!(open.value, value);
    assert_eq!(commitment, commit_from_open);
}

#[test]
fn commitment_recovery_is_as_expected() {
    let value: u64 = 1234;
    let (commitment, _) = Commitment::new(value);

    let commitment_vec = commitment.to_bytes();
    let commitment_bytes: &[u8] = &commitment_vec;
    let commit = Commitment::from_slice(&commitment_bytes).unwrap();

    assert_eq!(commitment, commit);
}

#[test]
fn mpc_is_as_expected() {
    let value1: u64 = 1000;
    let value2: u64 = 500;
    let value3: u64 = 250;
    let (commitment1, opening1) = Commitment::new(value1);
    let (commitment2, opening2) = Commitment::new(value2);
    let (commitment3, opening3) = Commitment::new(value3);

    let commitment12 = commitment1 + commitment2;
    let commitment123 = commitment12 + commitment3;

    // proof
    let opening = opening1 + opening2 + opening3;
    let value_final = value1 + value2 + value3;
    let commit = Commitment::from_opening(&opening);

    assert_eq!(value_final, opening.value);
    assert_eq!(commitment123, commit);
    assert!(commitment123.verify(&opening));
}

#[test]
fn non_unique_mpc_is_as_expected() {
    let value1: u64 = 1000;
    let value2: u64 = 500;
    let value3: u64 = 250;
    let (commitment1, opening1) = Commitment::new(value1);
    let (commitment2, opening2) = Commitment::new(value2);
    let (commitment3, opening3) = Commitment::new(value3);

    let commitment12 = commitment1.clone() + commitment2;
    let commitment13 = commitment1.clone() + commitment3;
    let commitment1123 = commitment12 + commitment13;

    // proof
    let opening = opening1.clone() + opening1 + opening2 + opening3;
    let value = value1 + value1 + value2 + value3;
    let commit = Commitment::from_opening(&opening);

    assert_eq!(value, opening.value);
    assert_eq!(commitment1123, commit);
}
