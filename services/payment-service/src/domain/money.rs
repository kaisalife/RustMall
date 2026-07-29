//! 金额类型
//!
//! 参考 Nautilus Trader 的定点小数设计，使用 `rust_decimal::Decimal` 作为底层存储。
//!
//! ## 为什么不用 f64？
//!
//! f64 是二进制浮点数，无法精确表示十进制小数：
//! ```text
//! 0.1 + 0.2 = 0.30000000000000004  // f64
//! 0.1 + 0.2 = 0.3                   // Decimal
//! ```
//! 在支付系统中，这种精度丢失会导致资金不一致，是不可接受的。
//!
//! ## 为什么不用裸整数分（i64 cents）？
//!
//! 整数分固定 2 位小数，无法支持：
//! - JPY（日元，0 位小数）
//! - BTC（比特币，8 位小数）
//! - 手续费率计算（0.006 = 0.6%，需要更高精度）
//!
//! ## rust_decimal 的优势
//!
//! - 128-bit 定点小数：96 位整数 + 缩放因子（0-28 位小数）
//! - 纯 Rust 实现，无 C 依赖
//! - 支持 serde 序列化/反序列化
//! - 支持 SQLx PostgreSQL NUMERIC 类型映射
//! - 编译时宏 `dec!` 创建常量

use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use common::{AppError, AppResult};

/// 货币类型（ISO 4217）
///
/// 不同货币的小数位数不同，影响展示和计算精度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    /// 人民币（2 位小数）
    CNY,
    /// 美元（2 位小数）
    USD,
    /// 日元（0 位小数）
    JPY,
    /// 比特币（8 位小数）
    BTC,
}

impl Currency {
    /// 返回货币代码字符串（如 "CNY"）
    pub fn as_str(&self) -> &'static str {
        match self {
            Currency::CNY => "CNY",
            Currency::USD => "USD",
            Currency::JPY => "JPY",
            Currency::BTC => "BTC",
        }
    }

    /// 返回该货币的标准小数位数
    pub fn decimal_places(&self) -> u32 {
        match self {
            Currency::CNY | Currency::USD => 2,
            Currency::JPY => 0,
            Currency::BTC => 8,
        }
    }
}

impl FromStr for Currency {
    type Err = AppError;

    fn from_str(s: &str) -> AppResult<Self> {
        match s.to_uppercase().as_str() {
            "CNY" => Ok(Currency::CNY),
            "USD" => Ok(Currency::USD),
            "JPY" => Ok(Currency::JPY),
            "BTC" => Ok(Currency::BTC),
            _ => Err(AppError::invalid_input(format!("Unknown currency: {}", s))),
        }
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 金额
///
/// 所有资金操作的统一类型。内部使用 `Decimal` 保证精度，永不使用 f64。
/// 携带 `Currency` 信息，防止不同币种金额误操作。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Money {
    /// 金额值，使用 Decimal 精确表示
    pub amount: Decimal,
    /// 货币代码
    pub currency: Currency,
}

impl Money {
    /// 创建金额
    pub fn new(amount: Decimal, currency: Currency) -> Self {
        Self { amount, currency }
    }

    /// 零金额
    pub fn zero(currency: Currency) -> Self {
        Self {
            amount: Decimal::ZERO,
            currency,
        }
    }

    /// 从整数分创建（兼容旧接口或外部系统）
    ///
    /// 例如：`Money::from_cents(9999, Currency::CNY)` = 99.99 CNY
    pub fn from_cents(cents: i64, currency: Currency) -> Self {
        Self {
            amount: Decimal::from(cents) / dec!(100),
            currency,
        }
    }

    /// 加法（同币种）
    ///
    /// 不同币种将返回错误，防止资金混淆。
    pub fn add(&self, other: &Money) -> AppResult<Money> {
        self.assert_same_currency(other)?;
        Ok(Money::new(self.amount + other.amount, self.currency))
    }

    /// 减法（同币种）
    pub fn sub(&self, other: &Money) -> AppResult<Money> {
        self.assert_same_currency(other)?;
        Ok(Money::new(self.amount - other.amount, self.currency))
    }

    /// 乘法（乘以数量或费率）
    ///
    /// 例如：`amount.mul(dec!(0.006))` 计算 0.6% 手续费
    pub fn mul(&self, factor: Decimal) -> Money {
        Money::new(self.amount * factor, self.currency)
    }

    /// 费率计算（手续费等）
    ///
    /// `rate` 为小数形式，如 0.006 表示 0.6%
    pub fn fee(&self, rate: Decimal) -> Money {
        self.mul(rate)
    }

    /// 判断是否为正数
    pub fn is_positive(&self) -> bool {
        self.amount > Decimal::ZERO
    }

    /// 判断是否为零
    pub fn is_zero(&self) -> bool {
        self.amount == Decimal::ZERO
    }

    /// 转为整数分（仅用于与外部系统交互）
    ///
    /// 注意：高精度币种（如 BTC）会截断为 2 位小数。
    pub fn to_cents(&self) -> i64 {
        (self.amount * dec!(100)).try_into().unwrap_or(0)
    }

    /// 校验同币种
    fn assert_same_currency(&self, other: &Money) -> AppResult<()> {
        if self.currency != other.currency {
            return Err(AppError::invalid_input(format!(
                "Currency mismatch: {} vs {}",
                self.currency, other.currency
            )));
        }
        Ok(())
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.amount, self.currency)
    }
}

impl FromStr for Money {
    type Err = AppError;

    /// 从字符串解析，格式："99.99 CNY"
    fn from_str(s: &str) -> AppResult<Self> {
        let parts: Vec<&str> = s.trim().split_whitespace().collect();
        if parts.len() != 2 {
            return Err(AppError::invalid_input(format!(
                "Invalid money format: {}, expected 'amount currency'",
                s
            )));
        }
        let amount = Decimal::from_str(parts[0])
            .map_err(|e| AppError::invalid_input(format!("Invalid amount: {}", e)))?;
        let currency = Currency::from_str(parts[1])?;
        Ok(Money::new(amount, currency))
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_money_create() {
        let m = Money::new(dec!(99.99), Currency::CNY);
        assert_eq!(m.amount, dec!(99.99));
        assert_eq!(m.currency, Currency::CNY);
    }

    #[test]
    fn test_money_from_cents() {
        let m = Money::from_cents(9999, Currency::CNY);
        assert_eq!(m.amount, dec!(99.99));
    }

    #[test]
    fn test_money_to_cents() {
        let m = Money::new(dec!(99.99), Currency::CNY);
        assert_eq!(m.to_cents(), 9999);
    }

    #[test]
    fn test_money_add_same_currency() {
        let a = Money::new(dec!(10.00), Currency::CNY);
        let b = Money::new(dec!(5.50), Currency::CNY);
        let result = a.add(&b).unwrap();
        assert_eq!(result.amount, dec!(15.50));
    }

    #[test]
    fn test_money_add_different_currency_fails() {
        let a = Money::new(dec!(10.00), Currency::CNY);
        let b = Money::new(dec!(5.00), Currency::USD);
        assert!(a.add(&b).is_err());
    }

    #[test]
    fn test_money_sub() {
        let a = Money::new(dec!(10.00), Currency::CNY);
        let b = Money::new(dec!(3.50), Currency::CNY);
        let result = a.sub(&b).unwrap();
        assert_eq!(result.amount, dec!(6.50));
    }

    #[test]
    fn test_money_mul() {
        let a = Money::new(dec!(100.00), Currency::CNY);
        let result = a.mul(dec!(0.006)); // 0.6% 手续费
        assert_eq!(result.amount, dec!(0.600));
    }

    #[test]
    fn test_money_fee() {
        let a = Money::new(dec!(10000.00), Currency::CNY);
        let fee = a.fee(dec!(0.006)); // 0.6%
        assert_eq!(fee.amount, dec!(60.000));
    }

    #[test]
    fn test_currency_decimal_places() {
        assert_eq!(Currency::CNY.decimal_places(), 2);
        assert_eq!(Currency::JPY.decimal_places(), 0);
        assert_eq!(Currency::BTC.decimal_places(), 8);
    }

    #[test]
    fn test_currency_from_str() {
        assert_eq!(Currency::from_str("CNY").unwrap(), Currency::CNY);
        assert_eq!(Currency::from_str("usd").unwrap(), Currency::USD);
        assert!(Currency::from_str("XYZ").is_err());
    }

    #[test]
    fn test_money_from_str() {
        let m = Money::from_str("99.99 CNY").unwrap();
        assert_eq!(m.amount, dec!(99.99));
        assert_eq!(m.currency, Currency::CNY);
    }

    #[test]
    fn test_money_is_positive() {
        assert!(Money::new(dec!(0.01), Currency::CNY).is_positive());
        assert!(!Money::zero(Currency::CNY).is_positive());
        assert!(!Money::new(dec!(-1.0), Currency::CNY).is_positive());
    }
}
