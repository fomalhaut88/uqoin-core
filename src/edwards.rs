//! Provides a pure Rust implementation of the Ed25519 elliptic curve,
//! a high-performance, secure, and deterministic digital signature scheme,
//! widely used in modern cryptographic applications.
//!
//! This module enables key generation, signing, and verification processes
//! essential for transaction authentication and network integrity in Uqoin.
//!
//! The equation is
//! `- x^2 + y^2 = 1 - scalar x^2 y^2` where `scalar = 121665/121666`
//! (or `0x2DFC9311D490018C7338BF8688861767FF8FF5B2BEBE27548A14B235ECA6874A`),
//! the modulo is `2^255-19 `
//! (or `0x7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFED`),
//! the generator has `y = 4/5`
//! (or `0x6666666666666666666666666666666666666666666666666666666666666658`)
//! and corresponding positive (even) `x`
//! `0x216936D3CD6E53FEC0A4E231FDD6DC5C692CC7609525A7B2C9562D608F25D51A`.
//! This curve has the order
//! `0x1000000000000000000000000000000014DEF9DEA2F79CD65812631A5CF5D3ED`
//! and the cofactor `8`.
//!
//! Reference: <https://en.wikipedia.org/wiki/EdDSA#Ed25519>

use finitelib::prelude::*;
use finitelib::group::Group;
use finitelib::gf::prime::Prime;
use finitelib::bigi::prime::sqrtrem;

use crate::utils::*;


/// Twisted Edwards curve defined by the equation 
/// `- x^2 + y^2 = 1 - scalar x^2 y^2`.
pub struct TwistedEdwardsCurve {
    /// The finite field that provides all the necessary arithmetic.
    pub field: Prime<U256, R256>,

    /// Modulo of the inner finite field.
    pub modulo: U256,

    /// The curve parameter that controls the "twist" of the curve shape.
    pub scalar: U256,

    /// Order of the curve.
    pub order: U256,

    /// Cofactor of the curve.
    pub cofactor: U256,

    /// Generator (or base point).
    pub generator: (U256, U256),
}


impl TwistedEdwardsCurve {
    /// Constructs a new instance of the curve using the standard parameters for 
    /// Ed25519.
    pub fn new_ed25519() -> Self {
        let modulo = U256::from_hex(
            "7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFED"
        );
        let field = Prime::new(R256{}, modulo.clone());
        let order = U256::from_hex(
            "1000000000000000000000000000000014DEF9DEA2F79CD65812631A5CF5D3ED"
        );
        let cofactor = U256::from(8);
        let scalar = U256::from_hex(
            "2DFC9311D490018C7338BF8688861767FF8FF5B2BEBE27548A14B235ECA6874A"
        );
        let generator_x = U256::from_hex(
            "216936D3CD6E53FEC0A4E231FDD6DC5C692CC7609525A7B2C9562D608F25D51A"
        );
        let generator_y = U256::from_hex(
            "6666666666666666666666666666666666666666666666666666666666666658"   
        );

        Self {
            field,
            modulo,
            scalar,
            order,
            cofactor,
            generator: (generator_x, generator_y),
        }
    }

    /// Checks whether the point `a` lies on the curve defined by this 
    /// instance. Returns `true` if the point satisfies the curve equation, 
    /// otherwise `false`.
    pub fn on_curve(&self, a: &(U256, U256)) -> bool {
        let x2 = self.field.mul(&a.0, &a.0);
        let y2 = self.field.mul(&a.1, &a.1);

        let left = self.field.sub(&y2, &x2);
        let right = self.field.sub(
            &self.field.one(), 
            &self.field.mul(
                &self.scalar,
                &self.field.mul(&x2, &y2)
            )
        );

        left == right
    }

    /// Given a y-coordinate, attempts to compute the corresponding positive 
    /// (even in terms of modulo) x-coordinate on the curve. Returns `Some(x)` 
    /// if such an x exists, otherwise `None` if the calculation fails (no valid
    /// point).
    pub fn calc_x(&self, y: &U256) -> Option<U256> {
        let y2 = self.field.mul(&y, &y);
        let x2 = self.field.div(
            &self.field.sub(&self.field.one(), &y2), 
            &self.field.sub(
                &self.field.mul(&y2, &self.scalar), 
                &self.field.one()
            )
        )?;
        let x = sqrtrem(&x2, &self.modulo)?;
        Some(x)
    }

    /// Apply iterator as bits of the power for the generator. Typically
    /// bits represent a private key, and the result point (or its y coordinate)
    /// is the corresponding public key.
    pub fn power(&self, it: impl Iterator<Item = bool>) -> (U256, U256) {
        self.mul_scalar(&self.generator, it)
    }
}


impl Group for TwistedEdwardsCurve {
    type Item = (U256, U256);

    fn zero(&self) -> Self::Item {
        (U256::from(0), U256::from(1))
    }

    fn eq(&self, a: &Self::Item, b: &Self::Item) -> bool {
        (a.0 == b.0) && (a.1 == b.1)
    }

    fn neg(&self, a: &Self::Item) -> Self::Item {
        (self.field.neg(&a.0), a.1.clone())
    }

    fn add(&self, a: &Self::Item, b: &Self::Item) -> Self::Item {
        let f = self.field.mul(
            &self.scalar,
            &self.field.mul(
                &self.field.mul(&a.0, &a.1),
                &self.field.mul(&b.0, &b.1),
            )
        );

        let x = self.field.div(
            &self.field.add(
                &self.field.mul(&a.0, &b.1),
                &self.field.mul(&a.1, &b.0),
            ),
            &self.field.sub(&self.field.one(), &f)
        ).unwrap();
        let y = self.field.div(
            &self.field.add(
                &self.field.mul(&a.1, &b.1),
                &self.field.mul(&a.0, &b.0),
            ),
            &self.field.add(&self.field.one(), &f)
        ).unwrap();

        (x, y)
    }
}


/// Projective representation for TwistedEdwardsCurve. Note: it keeps converted
/// generator.
pub struct TwistedEdwardsCurveProj {
    pub base: TwistedEdwardsCurve,
    pub generator: (U256, U256, U256),
}


impl TwistedEdwardsCurveProj {
    /// Create a new curve.
    pub fn new_ed25519() -> Self {
        let base = TwistedEdwardsCurve::new_ed25519();
        let generator = (
            base.generator.0.clone(), 
            base.generator.1.clone(), 
            base.field.one()
        );
        Self { base, generator }
    }

    /// Get base curve.
    pub fn base(&self) -> &TwistedEdwardsCurve {
        &self.base
    }

    /// Perform power.
    pub fn power(&self, it: impl Iterator<Item = bool>) -> (U256, U256, U256) {
        self.mul_scalar(&self.generator, it)
    }

    /// Convert into projective representation.
    pub fn convert_into(&self, a: &(U256, U256)) -> (U256, U256, U256) {
        (a.0.clone(), a.1.clone(), self.base.field.one())
    }

    /// Convert from projective representation.
    pub fn convert_from(&self, p: &(U256, U256, U256)) -> (U256, U256) {
        let iz = self.base.field.inv(&p.2).unwrap();
        let x = self.base.field.mul(&p.0, &iz);
        let y = self.base.field.mul(&p.1, &iz);
        (x, y)
    }
}


impl Group for TwistedEdwardsCurveProj {
    type Item = (U256, U256, U256);

    fn zero(&self) -> Self::Item {
        self.convert_into(&self.base.zero())
    }

    fn eq(&self, a: &Self::Item, b: &Self::Item) -> bool {
        (self.base.field.mul(&a.0, &b.2) == 
         self.base.field.mul(&b.0, &a.2)) && 
        (self.base.field.mul(&a.1, &b.2) == 
         self.base.field.mul(&b.1, &a.2))
    }

    fn neg(&self, a: &Self::Item) -> Self::Item {
        (self.base.field.neg(&a.0), a.1.clone(), a.2.clone())
    }

    fn add(&self, p: &Self::Item, q: &Self::Item) -> Self::Item {
        let a = self.base.field.mul(&p.0, &q.0);
        let b = self.base.field.mul(&p.1, &q.1);
        let c = self.base.field.mul(&self.base.scalar, 
                                    &self.base.field.mul(&a, &b));
        let w = self.base.field.mul(&p.2, &q.2);
        let d = self.base.field.mul(&w, &w);
        let u = self.base.field.add(&d, &c);
        let v = self.base.field.sub(&d, &c);
        let x = self.base.field.mul(
            &self.base.field.mul(&w, &u),
            &self.base.field.add(
                &self.base.field.mul(&p.0, &q.1),
                &self.base.field.mul(&p.1, &q.0),
            ),
        );
        let y = self.base.field.mul(
            &self.base.field.mul(&w, &v),
            &self.base.field.add(&a, &b),
        );
        let z = self.base.field.mul(&u, &v);
        (x, y, z)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    use rand::Rng;

    #[test]
    fn test_eq25519() {
        // Create a curve instance
        let ed25519 = TwistedEdwardsCurve::new_ed25519();

        // Check for random power
        let mut rng = rand::rng();
        let k: U256 = rng.random();
        let p = ed25519.power(k.bit_iter());
        assert!(ed25519.on_curve(&p));

        // Check the order
        let e = ed25519.power(ed25519.order.bit_iter());
        assert_eq!(e, ed25519.zero());
    }

    #[test]
    fn test_calc_x() {
        // Create a curve instance
        let ed25519 = TwistedEdwardsCurve::new_ed25519();

        // Test y
        let y = U256::from_hex(
            "57646626CB303A9EEBAAD078ACD56328DC4BFFC745FD5063738D9E10BF513204"
        );

        // Calculate x
        let x = ed25519.calc_x(&y).unwrap();

        // Check
        assert!(ed25519.on_curve(&(x, y)));
    }

    #[bench]
    fn bench_on_curve(bencher: &mut Bencher) {
        // Create a curve instance
        let ed25519 = TwistedEdwardsCurve::new_ed25519();

        // Take a random point on the curve
        let mut rng = rand::rng();
        let k: U256 = rng.random();
        let p = ed25519.power(k.bit_iter());

        // Benchmark
        bencher.iter(|| {
            let _ = ed25519.on_curve(&p);
        });
    }

    #[bench]
    fn bench_power(bencher: &mut Bencher) {
        // Create a curve instance
        let ed25519 = TwistedEdwardsCurve::new_ed25519();

        // Power (private key)
        let k = U256::from_hex(
            "0C9C3CC559450A34CF3A1CFBC109672CAC8E3DFA115A3F62ADBB321102CAC9DC"
        );

        // Point (public key)
        let px = U256::from_hex(
            "3E1D4C338BAB6EA001454D81C8AB62E73199864E4A0FAC45505330314BF40344"
        );
        let py = U256::from_hex(
            "2F3FA51805B460E07A5AC480E3260FC9C3F4F6F09A91339260A0E81BF4FB2488"
        );

        // Benchmark
        bencher.iter(|| {
            let p = ed25519.power(k.bit_iter());
            assert_eq!(p.0, px);
            assert_eq!(p.1, py);
        });
    }

    #[bench]
    fn bench_power_proj(bencher: &mut Bencher) {
        // Create a curve instance
        let curve = TwistedEdwardsCurveProj::new_ed25519();

        // Power (private key)
        let k = U256::from_hex(
            "0C9C3CC559450A34CF3A1CFBC109672CAC8E3DFA115A3F62ADBB321102CAC9DC"
        );

        // Point (public key)
        let px = U256::from_hex(
            "3E1D4C338BAB6EA001454D81C8AB62E73199864E4A0FAC45505330314BF40344"
        );
        let py = U256::from_hex(
            "2F3FA51805B460E07A5AC480E3260FC9C3F4F6F09A91339260A0E81BF4FB2488"
        );

        // Benchmark
        bencher.iter(|| {
            let s = curve.power(k.bit_iter());

            let (qx, qy) = curve.convert_from(&s);
            assert_eq!(qx, px);
            assert_eq!(qy, py);
        });
    }

    #[bench]
    fn bench_calc_x(bencher: &mut Bencher) {
        // Create a curve instance
        let ed25519 = TwistedEdwardsCurve::new_ed25519();

        // Random generator
        let mut rng = rand::rng();

        // Benchmark
        bencher.iter(|| {
            let y: U256 = &rng.random::<U256>() % &ed25519.modulo;
            let _ = ed25519.calc_x(&y);
        });
    }
}
