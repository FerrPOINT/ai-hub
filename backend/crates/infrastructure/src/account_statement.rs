//! Native unit is defined by the exact authenticated /key statement contract,
//! never by a connection display name or caller-supplied currency.
use crate::openrouter_metadata::decode_account;
use aihub_domain::{
    catalog::AccountObservation,
    error::HubError,
    financial::{Amount, Currency},
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
pub(crate) const OPENROUTER_STATEMENT_ORIGIN: &str = "openrouter.current-key.credit-statement-v1";
pub(crate) const OPENROUTER_BILLING_TIER: &str = "openrouter_catalog";
// No public fields, Debug or Deserialize: only the provider collector produces this witness.
pub struct OpenRouterStatement {
    pub(crate) account: AccountObservation,
    pub(crate) observed_at: DateTime<Utc>,
    pub(crate) currency: Currency,
    pub(crate) usage: Amount,
}
#[derive(Deserialize)]
struct CurrencyEcho {
    data: EchoData,
}
#[derive(Deserialize)]
struct EchoData {
    currency: Option<String>,
    billing_currency: Option<String>,
    is_provisioning_key: Option<bool>,
    disabled: Option<bool>,
}
pub fn decode_openrouter_statement(
    bytes: &[u8],
    observed_at: DateTime<Utc>,
) -> Result<OpenRouterStatement, HubError> {
    let account = decode_account(bytes)?;
    let echo: CurrencyEcho =
        serde_json::from_slice(bytes).map_err(|_| HubError::Invalid("statement shape"))?;
    if account.is_management_key != Some(false)
        || echo.data.is_provisioning_key == Some(true)
        || echo.data.disabled == Some(true)
        || account.expires_at.is_some_and(|t| t <= observed_at)
        || [echo.data.currency, echo.data.billing_currency]
            .iter()
            .flatten()
            .any(|unit| unit != "USD")
    {
        return Err(HubError::Invalid("unqualified account statement"));
    }
    let usage = account
        .usage
        .ok_or(HubError::Invalid("missing native statement usage"))?;
    Ok(OpenRouterStatement {
        account,
        observed_at,
        currency: Currency::parse("USD")?,
        usage,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authenticated_statement_requires_real_exact_usage_and_rejects_conflicting_units() {
        let at = Utc::now();
        let witness=decode_openrouter_statement(br#"{"data":{"usage":123.000000000000000001,"is_management_key":false,"label":"private-key-fragment"}}"#,at).unwrap();
        assert_eq!(witness.usage.to_string(), "123.000000000000000001");
        assert_eq!(witness.currency.to_string(), "USD");
        assert!(
            decode_openrouter_statement(br#"{"data":{"usage":0,"is_management_key":false}}"#, at)
                .is_ok(),
            "known zero is an actual statement, not fabricated expense"
        );
        for data in [br#"{"data":{"is_management_key":false}}"#.as_slice(),br#"{"data":{"usage":null,"is_management_key":false}}"#,br#"{"data":{"usage":"bad","is_management_key":false}}"#,br#"{"data":{"usage":0,"is_management_key":true}}"#,br#"{"data":{"usage":0,"is_management_key":false,"is_provisioning_key":true}}"#,br#"{"data":{"usage":0,"is_management_key":false,"currency":"EUR"}}"#,br#"{"data":{"usage":0,"is_management_key":false,"expires_at":"2000-01-01T00:00:00Z"}}"#] {assert!(decode_openrouter_statement(data,at).is_err());}
    }
}
