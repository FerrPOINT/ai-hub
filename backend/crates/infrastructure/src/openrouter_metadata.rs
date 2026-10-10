//! Bounded vendor decoding. No raw provider object crosses a public DTO boundary.
use aihub_domain::{
    catalog::{AccountObservation, CatalogModel, CatalogObservation, ModelMetadata},
    error::HubError,
    financial::{Amount, Rate},
};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, str::FromStr};
pub const MAX_METADATA_BYTES: usize = 2 * 1024 * 1024;
#[derive(Deserialize)]
struct Models {
    data: Vec<Model>,
}
#[derive(Deserialize)]
struct Model {
    id: String,
    context_length: Option<i64>,
    top_provider: Option<TopProvider>,
    #[serde(default)]
    supported_parameters: Vec<String>,
    architecture: Option<Architecture>,
    pricing: Option<Pricing>,
}
#[derive(Deserialize)]
struct TopProvider {
    context_length: Option<i64>,
    max_completion_tokens: Option<i64>,
}
#[derive(Deserialize)]
struct Architecture {
    #[serde(default)]
    input_modalities: Vec<String>,
    #[serde(default)]
    output_modalities: Vec<String>,
}
#[derive(Deserialize)]
struct Pricing {
    prompt: Option<String>,
    completion: Option<String>,
    request: Option<String>,
    input_cache_read: Option<String>,
    input_cache_write: Option<String>,
}
#[derive(Deserialize)]
struct KeyResponse {
    data: KeyData,
}
#[derive(Deserialize)]
struct KeyData {
    limit: Option<Box<RawValue>>,
    limit_remaining: Option<Box<RawValue>>,
    usage: Option<Box<RawValue>>,
    byok_usage: Option<Box<RawValue>>,
    is_free_tier: Option<bool>,
    is_management_key: Option<bool>,
    include_byok_in_limit: Option<bool>,
    expires_at: Option<DateTime<Utc>>,
}
fn decimal(value: &str) -> Result<BigDecimal, HubError> {
    // Bound digits/exponents before BigDecimal can allocate its expanded representation.
    if value.is_empty()
        || value.len() > 96
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'e' | b'E' | b'+' | b'-'))
    {
        return Err(HubError::Invalid("bounded provider decimal"));
    }
    if let Some((_, exponent)) = value.split_once(['e', 'E']) {
        let exponent = exponent
            .parse::<i32>()
            .map_err(|_| HubError::Invalid("provider decimal exponent"))?;
        if !(-64..=64).contains(&exponent) {
            return Err(HubError::Invalid("provider decimal exponent"));
        }
    }
    let number = BigDecimal::from_str(value).map_err(|_| HubError::Invalid("provider decimal"))?;
    if number < BigDecimal::from(0) {
        return Err(HubError::Invalid("negative provider price"));
    }
    Ok(number)
}
pub fn exact_amount_lexeme(value: &str) -> Result<Amount, HubError> {
    Amount::parse(&decimal(value)?.normalized().to_plain_string())
}
pub fn per_token_rate(value: &str) -> Result<Rate, HubError> {
    Rate::parse(
        &(decimal(value)? * BigDecimal::from(1000000))
            .normalized()
            .to_plain_string(),
    )
}
fn amount(value: Option<&RawValue>) -> Option<Amount> {
    value.and_then(|v| exact_amount_lexeme(v.get()).ok())
}
fn rate(value: Option<&str>) -> Option<Rate> {
    value.and_then(|v| per_token_rate(v).ok())
}
fn positive(value: Option<i64>) -> Result<Option<i64>, HubError> {
    if value.is_some_and(|n| n < 1 || n > 100000000) {
        return Err(HubError::Invalid("provider token metadata"));
    }
    Ok(value)
}
pub fn decode_catalog(
    bytes: &[u8],
    observed_at: DateTime<Utc>,
) -> Result<CatalogObservation, HubError> {
    if bytes.is_empty() || bytes.len() > MAX_METADATA_BYTES {
        return Err(HubError::Invalid("catalog payload bound"));
    }
    let input: Models =
        serde_json::from_slice(bytes).map_err(|_| HubError::Invalid("catalog shape"))?;
    if input.data.len() > 1000 {
        return Err(HubError::Invalid("catalog model count"));
    }
    let mut ids = BTreeSet::new();
    let mut models = Vec::with_capacity(input.data.len());
    for model in input.data {
        if model.id.is_empty()
            || model.id.len() > 256
            || !ids.insert(model.id.clone())
            || model.supported_parameters.len() > 100
        {
            return Err(HubError::Invalid("catalog model identity"));
        }
        let declared = positive(model.context_length)?;
        let (input_limit, output_limit) = if let Some(top) = model.top_provider {
            let top_context = positive(top.context_length)?;
            (
                match (declared, top_context) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (a, b) => a.or(b),
                },
                positive(top.max_completion_tokens)?,
            )
        } else {
            (declared, None)
        };
        let mut capabilities = vec![];
        if model.architecture.is_some_and(|a| {
            a.input_modalities.iter().any(|s| s == "text")
                && a.output_modalities.iter().any(|s| s == "text")
        }) {
            capabilities.push("text".into())
        }
        if model.supported_parameters.iter().any(|s| s == "tools") {
            capabilities.push("function_tools".into())
        }
        if model
            .supported_parameters
            .iter()
            .any(|s| s == "response_format")
        {
            capabilities.push("json_schema".into())
        }
        let pricing = model.pricing;
        models.push(CatalogModel {
            metadata: ModelMetadata {
                provider_model_id: model.id,
                input_limit,
                output_limit,
                capabilities,
                evidence_status: "unverified".into(),
                observed_at,
            },
            input_uncached: rate(pricing.as_ref().and_then(|p| p.prompt.as_deref())),
            input_cached: rate(pricing.as_ref().and_then(|p| p.input_cache_read.as_deref())),
            cache_write: rate(
                pricing
                    .as_ref()
                    .and_then(|p| p.input_cache_write.as_deref()),
            ),
            output_billable: rate(pricing.as_ref().and_then(|p| p.completion.as_deref())),
            request_fee: pricing
                .as_ref()
                .and_then(|p| p.request.as_deref())
                .and_then(|v| exact_amount_lexeme(v).ok()),
        });
    }
    Ok(CatalogObservation {
        models,
        observed_at,
        digest: hex::encode(Sha256::digest(bytes)),
    })
}
pub fn decode_account(bytes: &[u8]) -> Result<AccountObservation, HubError> {
    if bytes.is_empty() || bytes.len() > 65536 {
        return Err(HubError::Invalid("account metadata bound"));
    }
    let data: KeyResponse =
        serde_json::from_slice(bytes).map_err(|_| HubError::Invalid("account metadata shape"))?;
    Ok(AccountObservation {
        limit: amount(data.data.limit.as_deref()),
        remaining: amount(data.data.limit_remaining.as_deref()),
        usage: amount(data.data.usage.as_deref()),
        byok_usage: amount(data.data.byok_usage.as_deref()),
        is_free_tier: data.data.is_free_tier,
        is_management_key: data.data.is_management_key,
        include_byok_in_limit: data.data.include_byok_in_limit,
        expires_at: data.data.expires_at,
        digest: hex::encode(Sha256::digest(bytes)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_numeric_lexemes_and_per_token_conversion_never_pass_through_float() {
        assert_eq!(
            exact_amount_lexeme("99999999999999999999.000000000000000001")
                .unwrap()
                .to_string(),
            "99999999999999999999.000000000000000001"
        );
        assert_eq!(
            exact_amount_lexeme("1e-18").unwrap().to_string(),
            "0.000000000000000001"
        );
        assert_eq!(
            per_token_rate("0.000000000000000001").unwrap().to_string(),
            "0.000000000001"
        );
        for invalid in [
            "-1",
            "NaN",
            "1e999999999",
            "1e-10000000",
            "0.0000000000000000001",
        ] {
            assert!(exact_amount_lexeme(invalid).is_err(), "{invalid}")
        }
        assert!(
            per_token_rate("1e-19").is_err(),
            "a sub-scale rate must stay unknown, not rounded zero"
        );
    }
    #[test]
    fn catalog_keeps_missing_prices_unknown_and_advertisements_unverified() {
        let observed = Utc::now();
        let catalog=decode_catalog(br#"{"data":[{"id":"vendor/model-a","context_length":128000,"top_provider":{"context_length":32000,"max_completion_tokens":4096},"supported_parameters":["tools","response_format"],"architecture":{"input_modalities":["text"],"output_modalities":["text"]},"pricing":{"prompt":"0.0000002","completion":"0.0000008","request":"0"}},{"id":"vendor/model-b","context_length":null,"top_provider":null,"pricing":{"prompt":"-1","completion":null}}]}"#,observed).unwrap();
        assert_eq!(catalog.models.len(), 2);
        assert_eq!(catalog.models[0].metadata.input_limit, Some(32000));
        assert_eq!(catalog.models[0].metadata.output_limit, Some(4096));
        assert_eq!(catalog.models[0].metadata.evidence_status, "unverified");
        assert_eq!(
            catalog.models[0].input_uncached.unwrap().to_string(),
            "0.200000000000"
        );
        assert_eq!(
            catalog.models[0].output_billable.unwrap().to_string(),
            "0.800000000000"
        );
        assert!(catalog.models[0].input_cached.is_none());
        assert!(catalog.models[0].cache_write.is_none());
        assert!(catalog.models[1].request_fee.is_none());
        assert!(catalog.models[1].input_uncached.is_none());
        assert!(catalog.models[1].metadata.input_limit.is_none());
        assert!(decode_catalog(br#"{"data":[{"id":"same"},{"id":"same"}]}"#, observed).is_err());
        assert!(
            decode_catalog(br#"{"data":[{"id":"model","context_length":0}]}"#, observed).is_err()
        );
    }
    #[test]
    fn account_numbers_are_exact_and_label_or_unknown_missing_money_is_not_zero() {
        let account=decode_account(br#"{"data":{"label":"sensitive-key-fragment","creator_user_id":"private-account","limit":null,"limit_remaining":0.100000000000000001,"usage":1e-18,"byok_usage":null,"is_free_tier":false,"is_management_key":false,"include_byok_in_limit":false,"expires_at":null}}"#).unwrap();
        assert_eq!(
            account.remaining.unwrap().to_string(),
            "0.100000000000000001"
        );
        assert_eq!(account.usage.unwrap().to_string(), "0.000000000000000001");
        assert!(account.limit.is_none());
        assert!(account.byok_usage.is_none());
        let missing = decode_account(br#"{"data":{}}"#).unwrap();
        assert!(missing.usage.is_none());
        assert!(missing.remaining.is_none());
        let unsupported =
            decode_account(br#"{"data":{"usage":-1,"limit_remaining":1e10000000}}"#).unwrap();
        assert!(unsupported.usage.is_none());
        assert!(unsupported.remaining.is_none());
    }
}
