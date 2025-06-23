// Copyright (C) Moondance Labs Ltd.
// This file is part of Tanssi.

// Tanssi is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Tanssi is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Tanssi.  If not, see <http://www.gnu.org/licenses/>

#![cfg_attr(not(feature = "std"), no_std)]

use {
    sp_core::U256,
    sp_runtime::traits::{CheckedAdd, CheckedMul, CheckedSub, Zero},
};

/// Error returned by math operations which can overflow.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OverflowError;

/// Error returned by math operations which can underflow.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnderflowError;

/// Helper to compute ratios by multiplying then dividing by some values, while
/// performing the intermediary computation using a bigger type to avoid
/// overflows.
pub trait MulDiv: Sized {
    /// Multiply self by `a` then divide the result by `b`.
    /// Computation will be performed in a bigger type to avoid overflows.
    /// After the division, will return `None` if the result is to big for
    /// the real type or if `b` is zero.
    fn mul_div(self, a: Self, b: Self) -> Result<Self, OverflowError>;
}

macro_rules! impl_mul_div {
    ($type:ty, $bigger:ty) => {
        impl MulDiv for $type {
            fn mul_div(self, a: Self, b: Self) -> Result<Self, OverflowError> {
                if b.is_zero() {
                    return Err(OverflowError);
                }

                if self.is_zero() {
                    return Ok(<$type>::zero());
                }

                let s: $bigger = self.into();
                let a: $bigger = a.into();
                let b: $bigger = b.into();

                let r: $bigger = s * a / b;

                r.try_into().map_err(|_| OverflowError)
            }
        }
    };
}

impl_mul_div!(u8, u16);
impl_mul_div!(u16, u32);
impl_mul_div!(u32, u64);
impl_mul_div!(u64, u128);
impl_mul_div!(u128, U256);

/// Returns directly an error on overflow.
pub trait ErrAdd: CheckedAdd {
    /// Returns directly an error on overflow.
    fn err_add(&self, v: &Self) -> Result<Self, OverflowError> {
        self.checked_add(v).ok_or(OverflowError)
    }
}

impl<T: CheckedAdd> ErrAdd for T {}

/// Returns directly an error on underflow.
pub trait ErrSub: CheckedSub {
    /// Returns directly an error on underflow.
    fn err_sub(&self, v: &Self) -> Result<Self, UnderflowError> {
        self.checked_sub(v).ok_or(UnderflowError)
    }
}

impl<T: CheckedSub> ErrSub for T {}

/// Returns directly an error on overflow.
pub trait ErrMul: CheckedMul {
    /// Returns directly an error on overflow.
    fn err_mul(&self, v: &Self) -> Result<Self, OverflowError> {
        self.checked_mul(v).ok_or(OverflowError)
    }
}

impl<T: CheckedMul> ErrMul for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mul_div() {
        assert_eq!(42u128.mul_div(0, 0), Err(OverflowError));
        assert_eq!(42u128.mul_div(1, 0), Err(OverflowError));

        assert_eq!(u128::MAX.mul_div(2, 4), Ok(u128::MAX / 2));
        assert_eq!(u128::MAX.mul_div(2, 2), Ok(u128::MAX));
        assert_eq!(u128::MAX.mul_div(4, 2), Err(OverflowError));
    }

    #[test]
    fn test_mul_div_edge_cases() {
        // Test with zero self
        assert_eq!(0u64.mul_div(100, 50), Ok(0));
        assert_eq!(0u32.mul_div(u32::MAX, 1), Ok(0));

        // Test division by 1
        assert_eq!(100u64.mul_div(50, 1), Ok(5000));
        assert_eq!(u64::MAX.mul_div(1, 1), Ok(u64::MAX));

        // Test multiplication by 1
        assert_eq!(100u64.mul_div(1, 2), Ok(50));
        assert_eq!(100u64.mul_div(1, 100), Ok(1));

        // Test all types
        assert_eq!(10u8.mul_div(5, 2), Ok(25));
        assert_eq!(10u16.mul_div(5, 2), Ok(25));
        assert_eq!(10u32.mul_div(5, 2), Ok(25));
        assert_eq!(10u64.mul_div(5, 2), Ok(25));
        assert_eq!(10u128.mul_div(5, 2), Ok(25));

        // Test precision
        assert_eq!(100u64.mul_div(3, 7), Ok(42)); // 300/7 = 42.857... truncated to 42
        assert_eq!(100u64.mul_div(7, 3), Ok(233)); // 700/3 = 233.333... truncated to 233

        // Test near-overflow scenarios for different types
        assert_eq!(u8::MAX.mul_div(2, 3), Ok(170)); // 255*2/3 = 170
        assert_eq!(u16::MAX.mul_div(2, 3), Ok(43690)); // 65535*2/3 = 43690

        // Test overflow scenarios
        assert_eq!(u8::MAX.mul_div(u8::MAX, 1), Err(OverflowError));
        assert_eq!(u16::MAX.mul_div(u16::MAX, 1), Err(OverflowError));
    }

    #[test]
    fn test_err_add() {
        // Normal addition
        assert_eq!(5u32.err_add(&10), Ok(15));
        assert_eq!(0u64.err_add(&100), Ok(100));

        // Overflow cases
        assert_eq!(u32::MAX.err_add(&1), Err(OverflowError));
        assert_eq!(u64::MAX.err_add(&u64::MAX), Err(OverflowError));
        assert_eq!(u128::MAX.err_add(&1), Err(OverflowError));

        // Edge cases
        assert_eq!(u32::MAX.err_add(&0), Ok(u32::MAX));
        assert_eq!((u32::MAX - 1).err_add(&1), Ok(u32::MAX));

        // Different types
        assert_eq!(200u8.err_add(&55), Ok(255));
        assert_eq!(200u8.err_add(&56), Err(OverflowError));
    }

    #[test]
    fn test_err_sub() {
        // Normal subtraction
        assert_eq!(10u32.err_sub(&5), Ok(5));
        assert_eq!(100u64.err_sub(&100), Ok(0));

        // Underflow cases
        assert_eq!(0u32.err_sub(&1), Err(UnderflowError));
        assert_eq!(10u64.err_sub(&11), Err(UnderflowError));
        assert_eq!(0u128.err_sub(&u128::MAX), Err(UnderflowError));

        // Edge cases
        assert_eq!(u32::MAX.err_sub(&0), Ok(u32::MAX));
        assert_eq!(1u32.err_sub(&1), Ok(0));

        // Different types
        assert_eq!(255u8.err_sub(&255), Ok(0));
        assert_eq!(10u8.err_sub(&20), Err(UnderflowError));
    }

    #[test]
    fn test_err_mul() {
        // Normal multiplication
        assert_eq!(5u32.err_mul(&10), Ok(50));
        assert_eq!(0u64.err_mul(&100), Ok(0));
        assert_eq!(100u64.err_mul(&0), Ok(0));

        // Overflow cases
        assert_eq!(u32::MAX.err_mul(&2), Err(OverflowError));
        assert_eq!(u64::MAX.err_mul(&u64::MAX), Err(OverflowError));
        assert_eq!((u128::MAX / 2 + 1).err_mul(&2), Err(OverflowError));

        // Edge cases
        assert_eq!(u32::MAX.err_mul(&1), Ok(u32::MAX));
        assert_eq!(u32::MAX.err_mul(&0), Ok(0));
        assert_eq!(1u32.err_mul(&1), Ok(1));

        // Different types - testing boundary conditions
        assert_eq!(16u8.err_mul(&16), Err(OverflowError)); // 256 > u8::MAX
        assert_eq!(15u8.err_mul(&17), Ok(255)); // Exactly u8::MAX
        assert_eq!(256u16.err_mul(&256), Err(OverflowError)); // 65536 > u16::MAX
        assert_eq!(255u16.err_mul(&257), Ok(65535)); // Exactly u16::MAX

        // Test with large numbers that don't overflow
        let large = u64::MAX / 3;
        assert_eq!(large.err_mul(&2), Ok(large * 2));
        assert_eq!(large.err_mul(&3), Ok(u64::MAX / 3 * 3));
        assert_eq!(large.err_mul(&4), Err(OverflowError));
    }

    #[test]
    fn test_error_types() {
        // Test OverflowError
        let overflow1 = OverflowError;
        let overflow2 = OverflowError;
        assert_eq!(overflow1, overflow2);

        // Test UnderflowError
        let underflow1 = UnderflowError;
        let underflow2 = UnderflowError;
        assert_eq!(underflow1, underflow2);

        // Test Debug trait
        assert_eq!(format!("{:?}", OverflowError), "OverflowError");
        assert_eq!(format!("{:?}", UnderflowError), "UnderflowError");

        // Test ordering
        assert!(overflow1 >= overflow2);
        assert!(underflow1 >= underflow2);
    }
}
