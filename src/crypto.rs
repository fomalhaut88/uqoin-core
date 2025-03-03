use rand::Rng;
use finitelib::prelude::*;
use finitelib::gf::prime::Prime;

use crate::utils::*;
use crate::edwards::TwistedEdwardsCurve;


/// Crypto schema object that is responsible for operations over keys,
/// signatures, encryption. It also includes ECDSA algorithms.
pub struct Schema {
    curve: TwistedEdwardsCurve,
    field: Prime<U256, R256>,
}


impl Schema {
    /// Create a new schema object.
    pub fn new() -> Self {
        let curve = TwistedEdwardsCurve::new_ed25519();
        let field = Prime::new(R256{}, curve.order.clone());
        Self { curve, field }
    }

    /// Generate a random private key.
    pub fn gen_key<R: Rng>(&self, rng: &mut R) -> U256 {
        &rng.random::<U256>() % &self.curve.order
    }

    /// Get public key from a private one.
    pub fn get_public(&self, key: &U256) -> U256 {
        let point = self.curve.power(key.bit_iter());
        self.point_to_number(&point)
    }

    /// Generate a key pair.
    pub fn gen_pair<R: Rng>(&self, rng: &mut R) -> (U256, U256) {
        let key = self.gen_key(rng);
        let public = self.get_public(&key);
        (key, public)
    }

    /// Check the key pair.
    pub fn check_pair(&self, key: &U256, public: &U256) -> bool {
        self.get_public(key) == *public
    }

    /// Build ECDSA signature.
    pub fn build_signature<R: Rng>(&self, rng: &mut R, msg: &U256, 
                                   key: &U256) -> (U256, U256) {
        let t = self.gen_key(rng);
        let r = self.curve.power(t.bit_iter());
        let sign_r = self.point_to_number(&r);
        let q = &sign_r % &self.curve.order;
        let sign_s = self.field.div(
            &self.field.add(msg, &self.field.mul(key, &q)),
            &t
        ).unwrap();
        (sign_r, sign_s)
    }

    /// Check ECDSA signature.
    pub fn check_signature(&self, msg: &U256, public: &U256, 
                           signature: &(U256, U256)) -> bool {
        let (sign_r, sign_s) = signature;
        let p = self.point_from_number(public).unwrap();
        let q = sign_r % &self.curve.order;

        let u = self.field.div(msg, sign_s).unwrap();
        let v = self.field.div(&q, sign_s).unwrap();
        let r = self.curve.add(
            &self.curve.power(u.bit_iter()),
            &self.curve.mul_scalar(&p, v.bit_iter()),
        );

        self.point_to_number(&r) == *sign_r
    }

    /// Extract public from ECDSA signature.
    pub fn extract_public(&self, msg: &U256, signature: &(U256, U256)) -> U256 {
        let (sign_r, sign_s) = signature;
        let r = self.point_from_number(&sign_r).unwrap();
        let q = sign_r % &self.curve.order;

        let u = self.field.div(sign_s, &q).unwrap();
        let v = self.field.div(msg, &q).unwrap();
        let p = self.curve.sub(
            &self.curve.mul_scalar(&r, u.bit_iter()),
            &self.curve.power(v.bit_iter())
        );

        self.point_to_number(&p)
    }

    /// Serialize point on the elliptic curve into a number.
    pub fn point_to_number(&self, point: &(U256, U256)) -> U256 {
        let mut y = point.1.clone();
        if point.0.bit_get(0) {
            y.bit_set(255, true);
        }
        y
    }

    /// Deserialize point on the elliptic curve from a number.
    pub fn point_from_number(&self, number: &U256) -> Option<(U256, U256)> {
        let is_odd = number.bit_get(255);

        let y = if is_odd {
            let mut y = number.clone();
            y.bit_set(255, false);
            y
        } else {
            number.clone()
        };

        let mut x = self.curve.calc_x(&y)?;

        if x.bit_get(0) != is_odd {
            x = self.curve.field.neg(&x);
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
