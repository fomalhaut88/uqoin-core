//! Implements the `Schema` structure for cryptographic operations
//! based on the EdDSA algorithm using the Ed25519 twisted Edwards curve.
//!
//! The `Schema` encapsulates key generation, digital signature creation,
//! signature verification, and public key recovery functionalities.
//!
//! It is used in the Uqoin protocol to ensure the cryptographic security of 
//! transactions.

use rand::Rng;
use finitelib::prelude::*;
use finitelib::gf::prime::Prime;

use crate::utils::*;
use crate::edwards::TwistedEdwardsCurveProj;


/// Represents a cryptographic scheme based on the Ed25519 twisted Edwards 
/// curve.
///
/// The `Schema` structure encapsulates elliptic curve operations
/// and modular arithmetic required for key management and digital signatures.
pub struct Schema {
    curve: TwistedEdwardsCurveProj,
    field: Prime<U256, R256>,
}


impl Schema {
    /// Creates a new schema instance using the Ed25519 curve parameters.
    pub fn new() -> Self {
        let curve = TwistedEdwardsCurveProj::new_ed25519();
        let field = Prime::new(R256{}, curve.base.order.clone());
        Self { curve, field }
    }

    /// Returns a reference to the underlying elliptic curve.
    pub fn curve(&self) -> &TwistedEdwardsCurveProj {
        &self.curve
    }

    /// Generates a random private key.
    pub fn gen_key<R: Rng>(&self, rng: &mut R) -> U256 {
        &rng.random::<U256>() % &self.curve.base.order
    }

    /// Computes the public key corresponding to a given private key.
    pub fn get_public(&self, key: &U256) -> U256 {
        let point_proj = self.curve.power(key.bit_iter());
        let point = self.curve.convert_from(&point_proj);
        self.point_to_number(&point)
    }

    /// Generates a new key pair (private and public keys).
    pub fn gen_pair<R: Rng>(&self, rng: &mut R) -> (U256, U256) {
        let key = self.gen_key(rng);
        let public = self.get_public(&key);
        (key, public)
    }

    /// Verifies whether the public key matches the given private key.
    pub fn check_pair(&self, key: &U256, public: &U256) -> bool {
        self.get_public(key) == *public
    }

    /// Creates a digital signature for a given message using the private key.
    pub fn build_signature<R: Rng>(&self, rng: &mut R, msg: &U256, 
                                   key: &U256) -> Signature {
        let t = self.gen_key(rng);
        let rj = self.curve.power(t.bit_iter());
        let r = self.curve.convert_from(&rj);
        let sign_r = self.point_to_number(&r);
        let sign_s = self.field.div(
            &self.field.add(msg, &self.field.mul(key, &sign_r)),
            &t
        ).unwrap();
        (sign_r, sign_s)
    }

    /// Verifies a digital signature against a message and a public key.
    pub fn check_signature(&self, msg: &U256, public: &U256, 
                           signature: &Signature) -> bool {
        self.extract_public(msg, signature) == *public
    }

    /// Recovers the public key from a signed message and its signature.
    pub fn extract_public(&self, msg: &U256, signature: &Signature) -> U256 {
        let (sign_r, sign_s) = signature;
        let r = self.point_from_number(&sign_r).unwrap();
        let rj = self.curve.convert_into(&r);

        let u = self.field.div(sign_s, &sign_r).unwrap();
        let v = self.field.div(msg, &sign_r).unwrap();
        let pj = self.curve.sub(
            &self.curve.mul_scalar(&rj, u.bit_iter()),
            &self.curve.power(v.bit_iter())
        );
        let p = self.curve.convert_from(&pj);

        self.point_to_number(&p)
    }

    /// Serializes a point on the elliptic curve into a numeric representation.
    ///
    /// The point is compressed into a single `U256` value
    /// by encoding the y-coordinate and a sign bit indicating the x-coordinate.
    pub fn point_to_number(&self, point: &(U256, U256)) -> U256 {
        let mut y = point.1.clone();
        if point.0.bit_get(0) {
            y.bit_set(255, true);
        }
        y
    }

    /// Deserializes a numeric representation back into a point on the elliptic 
    /// curve.
    ///
    /// Given a `U256` number, reconstructs the corresponding point
    /// by decoding the y-coordinate and determining the correct x-coordinate.
    pub fn point_from_number(&self, number: &U256) -> Option<(U256, U256)> {
        let is_odd = number.bit_get(255);

        let y = if is_odd {
            let mut y = number.clone();
            y.bit_set(255, false);
            y
        } else {
            number.clone()
        };

        let mut x = self.curve.base.calc_x(&y)?;

        if x.bit_get(0) != is_odd {
            x = self.curve.base.field.neg(&x);
        }

        Some((x, y))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    #[test]
    fn test_point_serialization() {
        let schema = Schema::new();
        let mut rng = rand::rng();

        let y: U256 = rng.random();
        if let Some(p) = schema.point_from_number(&y) {
            let y2: U256 = schema.point_to_number(&p);
            assert_eq!(y, y2);
        }
    }

    #[test]
    fn test_pair() {
        let schema = Schema::new();
        let mut rng = rand::rng();

        let (key, public) = schema.gen_pair(&mut rng);

        assert!(schema.check_pair(&key, &public));
    }

    #[test]
    fn test_signature() {
        let schema = Schema::new();
        let mut rng = rand::rng();
        let (key, public) = schema.gen_pair(&mut rng);
        let msg: U256 = rng.random();

        let signature = schema.build_signature(&mut rng, &msg, &key);
        assert!(schema.check_signature(&msg, &public, &signature));

        let public2 = schema.extract_public(&msg, &signature);
        assert_eq!(public, public2);
    }

    #[bench]
    fn bench_point_serialize(bencher: &mut Bencher) {
        let schema = Schema::new();
        let mut rng = rand::rng();

        let p = {
            let res;
            loop {
                let y: U256 = rng.random();
                if let Some(p) = schema.point_from_number(&y) {
                    res = p;
                    break;
                }
            }
            res
        };

        bencher.iter(|| {
            let _y = schema.point_to_number(&p);
        });
    }

    #[bench]
    fn bench_point_deserialize(bencher: &mut Bencher) {
        let schema = Schema::new();
        let mut rng = rand::rng();

        let y = {
            let res;
            loop {
                let y: U256 = rng.random();
                if schema.point_from_number(&y).is_some() {
                    res = y;
                    break;
                }
            }
            res
        };

        bencher.iter(|| {
            let _p = schema.point_from_number(&y);
        });
    }

    #[bench]
    fn bench_gen_pair(bencher: &mut Bencher) {
        let schema = Schema::new();
        let mut rng = rand::rng();

        bencher.iter(|| {
            let _pair = schema.gen_pair(&mut rng);
        });
    }

    #[bench]
    fn bench_check_pair(bencher: &mut Bencher) {
        let schema = Schema::new();
        let mut rng = rand::rng();
        let (key, public) = schema.gen_pair(&mut rng);

        bencher.iter(|| {
            let _res = schema.check_pair(&key, &public);
        });
    }

    #[bench]
    fn bench_build_signature(bencher: &mut Bencher) {
        let schema = Schema::new();
        let mut rng = rand::rng();
        let (key, _public) = schema.gen_pair(&mut rng);
        let msg: U256 = rng.random();

        bencher.iter(|| {
            let _signature = schema.build_signature(&mut rng, &msg, &key);
        });
    }

    #[bench]
    fn bench_check_signature(bencher: &mut Bencher) {
        let schema = Schema::new();
        let mut rng = rand::rng();
        let (key, public) = schema.gen_pair(&mut rng);
        let msg: U256 = rng.random();

        let signature = schema.build_signature(&mut rng, &msg, &key);

        bencher.iter(|| {
            let _res = schema.check_signature(&msg, &public, &signature);
        });
    }

    #[bench]
    fn bench_extract_public(bencher: &mut Bencher) {
        let schema = Schema::new();
        let mut rng = rand::rng();
        let (key, _public) = schema.gen_pair(&mut rng);
        let msg: U256 = rng.random();

        let signature = schema.build_signature(&mut rng, &msg, &key);

        bencher.iter(|| {
            let _public = schema.extract_public(&msg, &signature);
        });
    }

    #[bench]
    fn bench_signature_together(bencher: &mut Bencher) {
        let schema = Schema::new();
        let mut rng = rand::rng();

        bencher.iter(|| {
            let (key, public) = schema.gen_pair(&mut rng);
            let msg: U256 = rng.random();
            let signature = schema.build_signature(&mut rng, &msg, &key);
            let public_restored = schema.extract_public(&msg, &signature);
            assert_eq!(public, public_restored);
        });
    }
}
