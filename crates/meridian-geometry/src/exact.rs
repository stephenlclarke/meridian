use crate::{Budget, Failure, FailureKind};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Signed, Zero};

pub type Rational = BigRational;
pub type Point2 = [Rational; 2];
pub type Point3 = [Rational; 3];

pub fn integer(value: i64) -> Rational {
    Rational::from_integer(BigInt::from(value))
}
pub fn fraction(
    numerator: &str,
    denominator: &str,
    budget: &mut Budget,
) -> Result<Rational, Failure> {
    budget.step(1)?;
    let parse = |text: &str| -> Result<BigInt, Failure> {
        if text.len() > 4098 || text.is_empty() {
            return Err(budget.fail(FailureKind::Rational, "source integer text exceeds bound"));
        }
        let value = text
            .parse::<BigInt>()
            .map_err(|_| budget.fail(FailureKind::InvalidInput, "invalid exact integer"))?;
        if value.bits() > budget.limits().rational_bits {
            return Err(budget.fail(
                FailureKind::Rational,
                "source integer exceeds rational bit bound",
            ));
        }
        Ok(value)
    };
    let n = parse(numerator)?;
    let d = parse(denominator)?;
    if d.is_zero() {
        return Err(budget.fail(FailureKind::InvalidInput, "zero rational denominator"));
    }
    let result = Rational::new(n, d);
    checked(&result, budget)?;
    Ok(result)
}

pub(crate) fn checked(value: &Rational, budget: &mut Budget) -> Result<(), Failure> {
    budget.step(1)?;
    if value.numer().bits().max(value.denom().bits()) > budget.limits().rational_bits {
        return Err(budget.fail(
            FailureKind::Rational,
            "coordinate or UV exceeds rational bit bound",
        ));
    }
    Ok(())
}
pub(crate) fn point<const N: usize>(value: &[Rational; N], b: &mut Budget) -> Result<(), Failure> {
    for v in value {
        checked(v, b)?;
    }
    Ok(())
}
pub(crate) fn sub<const N: usize>(a: &[Rational; N], b: &[Rational; N]) -> [Rational; N] {
    std::array::from_fn(|i| &a[i] - &b[i])
}
pub(crate) fn add<const N: usize>(a: &[Rational; N], b: &[Rational; N]) -> [Rational; N] {
    std::array::from_fn(|i| &a[i] + &b[i])
}
pub(crate) fn scale<const N: usize>(p: &[Rational; N], t: &Rational) -> [Rational; N] {
    std::array::from_fn(|i| &p[i] * t)
}
pub(crate) fn dot(a: &Point3, b: &Point3) -> Rational {
    (0..3).map(|i| &a[i] * &b[i]).sum()
}
pub(crate) fn cross(a: &Point3, b: &Point3) -> Point3 {
    [
        &a[1] * &b[2] - &a[2] * &b[1],
        &a[2] * &b[0] - &a[0] * &b[2],
        &a[0] * &b[1] - &a[1] * &b[0],
    ]
}
pub(crate) fn cross2(a: &Point2, b: &Point2) -> Rational {
    &a[0] * &b[1] - &a[1] * &b[0]
}
pub(crate) fn orient(
    a: &Point2,
    b: &Point2,
    p: &Point2,
    budget: &mut Budget,
) -> Result<Rational, Failure> {
    budget.step(1)?;
    Ok(cross2(&sub(b, a), &sub(p, a)))
}
pub(crate) fn area(poly: &[Point2], budget: &mut Budget) -> Result<Rational, Failure> {
    budget.step(poly.len() as u64)?;
    Ok((0..poly.len())
        .map(|i| cross2(&poly[i], &poly[(i + 1) % poly.len()]))
        .sum())
}
pub(crate) fn axis(normal: &Point3) -> usize {
    let mut result = 0;
    for i in 1..3 {
        if normal[i].abs() > normal[result].abs() {
            result = i;
        }
    }
    result
}
pub(crate) fn project(p: &Point3, omitted: usize) -> Point2 {
    let axes: Vec<_> = (0..3).filter(|k| *k != omitted).collect();
    [p[axes[0]].clone(), p[axes[1]].clone()]
}
