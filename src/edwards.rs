//! Implementation of the curve Ed25519 taken from 
//! https://en.wikipedia.org/wiki/EdDSA#Ed25519. The equation is
//! `- x^2 + y^2 = 1 - scalar x^2 y^2` where scalar = 121665/121666
//! (or 0x2DFC9311D490018C7338BF8688861767FF8FF5B2BEBE27548A14B235ECA6874A),
//! the modulo is 2^255-19 
//! (or 0x7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFED),
//! the generator has y = 4/5
//! (or 0x6666666666666666666666666666666666666666666666666666666666666658)
//! and corresponding positive (even x)
//! 0x216936D3CD6E53FEC0A4E231FDD6DC5C692CC7609525A7B2C9562D608F25D51A.
//! This curve has the order
//! 0x1000000000000000000000000000000014DEF9DEA2F79CD65812631A5CF5D3ED
//! and cofactor 8.

use finitelib::prelude::*;
use finitelib::group::Group;
use finitelib::gf::prime::Prime;
use finitelib::bigi::prime::sqrtrem;

use crate::utils::*;


/// Twisted Edwards curve defined by the equation 
/// `- x^2 + y^2 = 1 - scalar x^2 y^2`.
pub struct TwistedEdwardsCurve {
    pub field: Prime<U256, R256>,
    pub modulo: U256,
    pub scalar: U256,
    pub order: U256,
    pub cofactor: U256,
    pub generator: (U256, U256),
}


impl TwistedEdwardsCurve {
    // Create a new curve instance.
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

    /// Check the point on the curve.
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

    /// Calculate positive (even) x coordinate of the point on the curve
    /// by given y coordinate.
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
        let y = U256::from_decimal(
            "1199956463082827061555360551106195666022978064759116914037829679333165248376"
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

        // Random generator
        let mut rng = rand::rng();

        // Benchmark
        bencher.iter(|| {
            let k: U256 = rng.random();
            let _ = ed25519.power(k.bit_iter());
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
