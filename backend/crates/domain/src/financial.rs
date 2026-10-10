//! Exact codecs and disjoint usage. No rate, currency, or missing-fee defaults.
use crate::error::HubError;
use bigdecimal::num_bigint::BigInt;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, str::FromStr};
use utoipa::ToSchema;

fn parts(
    value: &str,
    precision: usize,
    scale: usize,
    signed: bool,
) -> Result<(bool, &str, &str), HubError> {
    if value.is_empty() || value.len() > precision + 3 {
        return Err(HubError::Invalid("decimal range"));
    }
    let negative = value.starts_with('-');
    if negative && !signed {
        return Err(HubError::Invalid("negative amount"));
    }
    let raw = if negative { &value[1..] } else { value };
    let (whole, fraction) = raw.split_once('.').unwrap_or((raw, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
        || fraction.len() > scale
        || whole.trim_start_matches('0').len() > precision - scale
        || (raw.contains('.') && fraction.is_empty())
    {
        return Err(HubError::Invalid("decimal precision/scale"));
    }
    Ok((negative, whole, fraction))
}
fn parse_units(
    value: &str,
    precision: usize,
    scale: usize,
    signed: bool,
) -> Result<i128, HubError> {
    let (negative, whole, fraction) = parts(value, precision, scale, signed)?;
    let whole = whole
        .parse::<i128>()
        .map_err(|_| HubError::Invalid("decimal overflow"))?;
    let tail = if fraction.is_empty() {
        0
    } else {
        fraction
            .parse::<i128>()
            .map_err(|_| HubError::Invalid("decimal"))?
    };
    let units = whole
        .checked_mul(10_i128.pow(scale as u32))
        .and_then(|v| {
            tail.checked_mul(10_i128.pow((scale - fraction.len()) as u32))
                .and_then(|tail| v.checked_add(tail))
        })
        .ok_or(HubError::Invalid("decimal overflow"))?;
    Ok(if negative { -units } else { units })
}
fn format_units(units: i128, scale: usize) -> String {
    let value = units.abs();
    let base = 10_i128.pow(scale as u32);
    format!(
        "{}{whole}.{fraction:0width$}",
        if units < 0 { "-" } else { "" },
        whole = value / base,
        fraction = value % base,
        width = scale
    )
}
macro_rules! fixed {
    ($name:ident,$precision:literal,$scale:literal,$signed:literal) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ToSchema,
        )]
        #[serde(try_from = "String", into = "String")]
        #[schema(value_type=String)]
        pub struct $name(i128);
        impl $name {
            pub fn parse(value: &str) -> Result<Self, HubError> {
                Ok(Self(parse_units(value, $precision, $scale, $signed)?))
            }
            pub fn from_units(units: i128) -> Result<Self, HubError> {
                if units
                    .checked_abs()
                    .is_none_or(|n| n >= 10_i128.pow($precision))
                    || (!$signed && units < 0)
                {
                    return Err(HubError::Invalid("decimal overflow"));
                }
                Ok(Self(units))
            }
            pub fn units(self) -> i128 {
                self.0
            }
            pub fn checked_add(self, other: Self) -> Result<Self, HubError> {
                Self::from_units(
                    self.0
                        .checked_add(other.0)
                        .ok_or(HubError::Invalid("decimal overflow"))?,
                )
            }
        }
        impl TryFrom<String> for $name {
            type Error = HubError;
            fn try_from(v: String) -> Result<Self, Self::Error> {
                Self::parse(&v)
            }
        }
        impl From<$name> for String {
            fn from(v: $name) -> Self {
                format_units(v.0, $scale)
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&String::from(*self))
            }
        }
    };
}
fixed!(Amount, 38, 18, false);
fixed!(Adjustment, 38, 18, true);
fixed!(Rate, 30, 12, false);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "String", into = "String")]
#[schema(value_type=String)]
pub struct Currency(String);
impl Currency {
    pub fn parse(value: &str) -> Result<Self, HubError> {
        if value.len() != 3 || !value.bytes().all(|b| b.is_ascii_uppercase()) {
            return Err(HubError::Invalid("currency"));
        }
        Ok(Self(value.into()))
    }
}
impl TryFrom<String> for Currency {
    type Error = HubError;
    fn try_from(v: String) -> Result<Self, HubError> {
        Self::parse(&v)
    }
}
impl From<Currency> for String {
    fn from(v: Currency) -> Self {
        v.0
    }
}
impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    InputUncached,
    InputCached,
    CacheWrite,
    OutputBillable,
    ExplicitOther,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Usage {
    pub categories: BTreeMap<Category, u64>,
    pub source: String,
    pub complete: bool,
}
impl Usage {
    pub fn normalized(
        input_total: u64,
        cached: u64,
        output: u64,
        source: String,
    ) -> Result<Self, HubError> {
        let uncached = input_total
            .checked_sub(cached)
            .ok_or(HubError::Invalid("usage_invalid: cached exceeds input"))?;
        if source.is_empty()
            || source.len() > 256
            || [input_total, cached, output]
                .iter()
                .any(|n| *n > i64::MAX as u64)
        {
            return Err(HubError::Invalid("usage_invalid"));
        }
        Ok(Self {
            categories: [
                (Category::InputUncached, uncached),
                (Category::InputCached, cached),
                (Category::OutputBillable, output),
            ]
            .into(),
            source,
            complete: true,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Price {
    pub currency: Currency,
    pub rates: BTreeMap<Category, Rate>,
    pub request_fee: Option<Amount>,
}
impl Price {
    /// Missing positive category or unknown fee means no monetary bound.
    pub fn cost(&self, usage: &Usage) -> Result<Option<Amount>, HubError> {
        if usage.source.is_empty() || usage.source.len() > 256 {
            return Err(HubError::Invalid("usage provenance"));
        }
        if !usage.complete
            || ![
                Category::InputUncached,
                Category::InputCached,
                Category::OutputBillable,
            ]
            .iter()
            .all(|c| usage.categories.contains_key(c))
        {
            return Ok(None);
        }
        let Some(mut total) = self.request_fee else {
            return Ok(None);
        };
        for (category, tokens) in &usage.categories {
            if *tokens > i64::MAX as u64 {
                return Err(HubError::Invalid("usage overflow"));
            }
            if *tokens == 0 {
                continue;
            }
            let Some(rate) = self.rates.get(category) else {
                return Ok(None);
            };
            // tokens * rate(1e-12 / million) directly yields amount(1e-18).
            let units = rate
                .units()
                .checked_mul(i128::from(*tokens))
                .ok_or(HubError::Invalid("cost overflow"))?;
            total = total.checked_add(Amount::from_units(units)?)?;
        }
        Ok(Some(total))
    }
}

/// Project NUMERIC(50,24) cannot be represented by a provider i128 codec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "String", into = "String")]
#[schema(value_type=String)]
pub struct ProjectAmount(BigInt);
impl ProjectAmount {
    pub fn parse(value: &str) -> Result<Self, HubError> {
        let (negative, whole, fraction) = parts(value, 50, 24, true)?;
        let digits = format!("{whole}{fraction}{}", "0".repeat(24 - fraction.len()));
        let mut units =
            BigInt::from_str(&digits).map_err(|_| HubError::Invalid("project decimal"))?;
        if negative {
            units = -units;
        }
        Self::from_units(units)
    }
    fn from_units(units: BigInt) -> Result<Self, HubError> {
        if units.to_string().trim_start_matches('-').len() > 50 {
            return Err(HubError::Invalid("project decimal overflow"));
        }
        Ok(Self(units))
    }
    pub fn markup(basis: Amount, markup_bps: u32) -> Result<Self, HubError> {
        if markup_bps > 1000000 {
            return Err(HubError::Invalid("markup bounds"));
        }
        Self::from_units(BigInt::from(basis.units()) * BigInt::from(10000 + markup_bps) * 100)
    }
    pub fn margin(&self, basis: Amount) -> Result<Self, HubError> {
        Self::from_units(&self.0 - BigInt::from(basis.units()) * 1000000)
    }
    pub fn custom_cost(price: &Price, usage: &Usage) -> Result<Option<Self>, HubError> {
        if usage.source.is_empty() || usage.source.len() > 256 {
            return Err(HubError::Invalid("usage provenance"));
        }
        if !usage.complete
            || ![
                Category::InputUncached,
                Category::InputCached,
                Category::OutputBillable,
            ]
            .iter()
            .all(|c| usage.categories.contains_key(c))
        {
            return Ok(None);
        }
        // Custom project rates are disjoint token rates, without provider fees.
        let mut total = BigInt::from(0);
        for (category, tokens) in &usage.categories {
            if *tokens > i64::MAX as u64 {
                return Err(HubError::Invalid("usage overflow"));
            }
            if *tokens == 0 {
                continue;
            }
            let Some(rate) = price.rates.get(category) else {
                return Ok(None);
            };
            total += BigInt::from(rate.units()) * BigInt::from(*tokens) * 1000000;
        }
        Ok(Some(Self::from_units(total)?))
    }
}
impl TryFrom<String> for ProjectAmount {
    type Error = HubError;
    fn try_from(v: String) -> Result<Self, HubError> {
        Self::parse(&v)
    }
}
impl From<ProjectAmount> for String {
    fn from(v: ProjectAmount) -> Self {
        let text = v.0.to_string();
        let negative = text.starts_with('-');
        let digits = text.trim_start_matches('-');
        let padded = format!(
            "{}{}",
            "0".repeat(25_usize.saturating_sub(digits.len())),
            digits
        );
        let split = padded.len() - 24;
        format!(
            "{}{}.{}",
            if negative { "-" } else { "" },
            &padded[..split],
            &padded[split..]
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Acceptance {
    NotAccepted,
    Accepted,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CostConfidence {
    Confirmed,
    Estimated,
    Unknown,
}

pub fn releasable(acceptance: Acceptance, confidence: CostConfidence, terminal: bool) -> bool {
    terminal && acceptance != Acceptance::Unknown && confidence != CostConfidence::Unknown
}

pub fn totals_by_currency(
    entries: impl IntoIterator<Item = (Currency, Amount)>,
) -> Result<BTreeMap<Currency, Amount>, HubError> {
    let mut totals = BTreeMap::new();
    for (currency, amount) in entries {
        let prior = totals
            .get(&currency)
            .copied()
            .unwrap_or(Amount::from_units(0)?);
        totals.insert(currency, prior.checked_add(amount)?);
    }
    Ok(totals)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn price() -> Price {
        Price {
            currency: Currency::parse("USD").unwrap(),
            rates: [
                (Category::InputUncached, Rate::parse("2").unwrap()),
                (Category::InputCached, Rate::parse("0.5").unwrap()),
                (Category::OutputBillable, Rate::parse("8").unwrap()),
            ]
            .into(),
            request_fee: Some(Amount::parse("0").unwrap()),
        }
    }
    #[test]
    fn accounting_example_and_paid_failed_attempt_are_exact() {
        let usage = Usage::normalized(2000, 800, 1000, "qualified-adapter".into()).unwrap();
        let cost = price().cost(&usage).unwrap().unwrap();
        assert_eq!(cost.to_string(), "0.010800000000000000");
        let failed = price()
            .cost(&Usage::normalized(100, 0, 0, "qualified-adapter".into()).unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(
            cost.checked_add(failed).unwrap().to_string(),
            "0.011000000000000000"
        );
        assert!(Usage::normalized(99, 100, 0, "qualified".into()).is_err());
    }
    #[test]
    fn smallest_unit_is_not_rounded_and_overflow_is_rejected() {
        let mut price = price();
        price.rates.insert(
            Category::InputUncached,
            Rate::parse("0.000000000001").unwrap(),
        );
        assert_eq!(
            price
                .cost(&Usage::normalized(1, 0, 0, "qualified".into()).unwrap())
                .unwrap()
                .unwrap()
                .units(),
            1
        );
        for value in [
            "0.0000000000000000001",
            "1e-18",
            "NaN",
            "-0.01",
            "100000000000000000000",
            "1.",
            "+1",
        ] {
            assert!(Amount::parse(value).is_err(), "{value}");
        }
        price.rates.insert(
            Category::InputUncached,
            Rate::parse("999999999999999999.999999999999").unwrap(),
        );
        assert!(
            price
                .cost(&Usage::normalized(i64::MAX as u64, 0, 0, "qualified".into()).unwrap())
                .is_err()
        );
    }
    #[test]
    fn absent_fee_or_rate_is_unknown_and_reserve_cannot_release() {
        let usage = Usage::normalized(1, 0, 1, "qualified".into()).unwrap();
        let mut price = price();
        price.request_fee = None;
        assert_eq!(price.cost(&usage).unwrap(), None);
        price.request_fee = Some(Amount::parse("0").unwrap());
        price.rates.remove(&Category::OutputBillable);
        assert_eq!(price.cost(&usage).unwrap(), None);
        for confidence in [
            CostConfidence::Confirmed,
            CostConfidence::Estimated,
            CostConfidence::Unknown,
        ] {
            assert!(!releasable(Acceptance::Unknown, confidence, true));
        }
        assert!(!releasable(
            Acceptance::Accepted,
            CostConfidence::Unknown,
            true
        ));
        assert!(releasable(
            Acceptance::Accepted,
            CostConfidence::Estimated,
            true
        ));
        let mut partial = usage.clone();
        partial.categories.remove(&Category::InputUncached);
        assert_eq!(super::tests::price().cost(&partial).unwrap(), None);
    }
    #[test]
    fn project_precision_margin_and_currency_boundaries() {
        let basis = Amount::parse("0.000000000000000001").unwrap();
        let charge = ProjectAmount::markup(basis, 2000).unwrap();
        assert_eq!(String::from(charge.clone()), "0.000000000000000001200000");
        assert_eq!(
            String::from(charge.margin(basis).unwrap()),
            "0.000000000000000000200000"
        );
        assert!(
            ProjectAmount::parse("99999999999999999999999999.999999999999999999999999").is_ok()
        );
        assert!(ProjectAmount::parse("100000000000000000000000000").is_err());
        let totals = totals_by_currency([
            (Currency::parse("USD").unwrap(), basis),
            (Currency::parse("EUR").unwrap(), basis),
        ])
        .unwrap();
        assert_eq!(totals.len(), 2);
        assert!(Currency::parse("usd").is_err());
    }
}
