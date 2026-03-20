pub(crate) mod approve;
pub(crate) mod bridge;
pub(crate) mod clob;
pub(crate) mod comments;
pub(crate) mod ctf;
pub(crate) mod data;
pub(crate) mod events;
pub(crate) mod markets;
pub(crate) mod profiles;
pub(crate) mod series;
pub(crate) mod sports;
pub(crate) mod tags;

use chrono::{DateTime, Utc};
use polymarket_client_sdk::types::Decimal;
use rust_decimal::prelude::ToPrimitive;
use tabled::Table;
use tabled::settings::object::Columns;
use tabled::settings::{Modify, Style, Width};

pub(crate) const DASH: &str = "—";

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub(crate) enum OutputFormat {
    Table,
    Json,
}

pub(crate) fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut truncated: String = s.chars().take(max.saturating_sub(1)).collect();
    truncated.push('\u{2026}');
    truncated
}

pub(crate) fn format_decimal(n: Decimal) -> String {
    let f = n.to_f64().unwrap_or(0.0);
    let abs = f.abs();
    let sign = if f < 0.0 { "-" } else { "" };
    if abs >= 1_000_000.0 {
        format!("{sign}${:.1}M", abs / 1_000_000.0)
    } else if abs >= 1_000.0 {
        format!("{sign}${:.1}K", abs / 1_000.0)
    } else {
        format!("{sign}${abs:.2}")
    }
}

pub(crate) fn format_date(d: &DateTime<Utc>) -> String {
    d.format("%Y-%m-%d %H:%M UTC").to_string()
}

pub(crate) fn active_status(closed: Option<bool>, active: Option<bool>) -> &'static str {
    if closed == Some(true) {
        "Closed"
    } else if active == Some(true) {
        "Active"
    } else {
        "Inactive"
    }
}

pub(crate) fn print_json(data: &(impl serde::Serialize + ?Sized)) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(data)?);
    Ok(())
}

/// structured error kind so agents can branch on failure type without parsing prose
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ErrorKind {
    Auth,
    Config,
    Network,
    NotFound,
    InvalidInput,
    Internal,
}

impl ErrorKind {
    /// classify an anyhow error by walking its context chain
    pub fn from_error(error: &anyhow::Error) -> Self {
        let chain: String = error
            .chain()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let msg = chain.to_lowercase();

        if msg.contains("no wallet configured")
            || msg.contains("invalid private key")
            || msg.contains("failed to authenticate")
        {
            Self::Auth
        } else if msg.contains("failed to connect")
            || msg.contains("failed to fetch")
            || msg.contains("http error")
            || msg.contains("download failed")
        {
            Self::Network
        } else if msg.contains("not found") {
            Self::NotFound
        } else if msg.contains("invalid token id")
            || msg.contains("invalid date")
            || msg.contains("invalid price")
            || msg.contains("invalid size")
            || msg.contains("invalid amount")
        {
            Self::InvalidInput
        } else if msg.contains("config")
            || msg.contains("could not determine home")
            || msg.contains("invalid json in config")
        {
            Self::Config
        } else {
            Self::Internal
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auth => "auth",
            Self::Config => "config",
            Self::Network => "network",
            Self::NotFound => "not_found",
            Self::InvalidInput => "invalid_input",
            Self::Internal => "internal",
        }
    }

    pub fn exit_code(self) -> u8 {
        match self {
            Self::Auth => 2,
            Self::Config => 3,
            Self::Network => 4,
            Self::NotFound => 5,
            Self::InvalidInput => 6,
            Self::Internal => 1,
        }
    }

    pub fn hint(self) -> Option<&'static str> {
        match self {
            Self::Auth => Some("run `polymarket setup` or pass --private-key"),
            Self::Config => Some("check ~/.config/polymarket/config.json"),
            Self::Network => Some("check your internet connection and try again"),
            _ => None,
        }
    }
}

pub(crate) fn print_error(error: &anyhow::Error, format: OutputFormat) -> u8 {
    let kind = ErrorKind::from_error(error);
    match format {
        OutputFormat::Json => {
            let mut obj = serde_json::json!({
                "error": kind.as_str(),
                "message": error.to_string(),
            });
            if let Some(hint) = kind.hint() {
                obj["hint"] = serde_json::Value::String(hint.into());
            }
            println!("{}", serde_json::to_string_pretty(&obj).unwrap_or_default());
        }
        OutputFormat::Table => {
            eprintln!("Error: {error}");
            if let Some(hint) = kind.hint() {
                eprintln!("Hint: {hint}");
            }
        }
    }
    kind.exit_code()
}

pub(crate) fn print_detail_table(rows: Vec<[String; 2]>) {
    let table = Table::from_iter(rows)
        .with(Style::rounded())
        .with(Modify::new(Columns::first()).with(Width::wrap(20)))
        .with(Modify::new(Columns::last()).with(Width::wrap(80)))
        .to_string();
    println!("{table}");
}

macro_rules! detail_field {
    ($rows:expr, $label:expr, $val:expr) => {
        $rows.push([$label.into(), $val]);
    };
}

pub(crate) use detail_field;

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn truncate_shorter_than_max_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_exact_length_unchanged() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn truncate_over_max_appends_ellipsis() {
        assert_eq!(truncate("hello world", 6), "hello\u{2026}");
    }

    #[test]
    fn truncate_max_one_is_just_ellipsis() {
        assert_eq!(truncate("hello", 1), "\u{2026}");
    }

    #[test]
    fn truncate_max_zero_is_just_ellipsis() {
        assert_eq!(truncate("hello", 0), "\u{2026}");
    }

    #[test]
    fn truncate_empty_string_unchanged() {
        assert_eq!(truncate("", 5), "");
    }

    #[test]
    fn truncate_counts_chars_not_bytes() {
        // "café!" is 5 chars but 6 bytes (é is 2 bytes)
        assert_eq!(truncate("café!", 3), "ca\u{2026}");
    }

    #[test]
    fn format_decimal_millions() {
        assert_eq!(format_decimal(dec!(1_500_000)), "$1.5M");
    }

    #[test]
    fn format_decimal_at_million_boundary() {
        assert_eq!(format_decimal(dec!(1_000_000)), "$1.0M");
    }

    #[test]
    fn format_decimal_thousands() {
        assert_eq!(format_decimal(dec!(1_500)), "$1.5K");
    }

    #[test]
    fn format_decimal_at_thousand_boundary() {
        assert_eq!(format_decimal(dec!(1_000)), "$1.0K");
    }

    #[test]
    fn format_decimal_just_below_thousand() {
        assert_eq!(format_decimal(dec!(999)), "$999.00");
    }

    #[test]
    fn format_decimal_sub_dollar() {
        assert_eq!(format_decimal(dec!(0.5)), "$0.50");
    }

    #[test]
    fn format_decimal_zero() {
        assert_eq!(format_decimal(dec!(0)), "$0.00");
    }

    #[test]
    fn format_decimal_negative() {
        assert_eq!(format_decimal(dec!(-500)), "-$500.00");
    }

    #[test]
    fn format_decimal_negative_thousands() {
        assert_eq!(format_decimal(dec!(-1_500)), "-$1.5K");
    }

    #[test]
    fn format_decimal_just_below_million_uses_k() {
        assert_eq!(format_decimal(dec!(999_999)), "$1000.0K");
    }

    #[test]
    fn error_kind_classifies_missing_wallet_as_auth() {
        let err = anyhow::anyhow!("No wallet configured. Run `polymarket wallet create`");
        assert_eq!(ErrorKind::from_error(&err), ErrorKind::Auth);
    }

    #[test]
    fn error_kind_classifies_invalid_key_as_auth() {
        let err = anyhow::anyhow!("something went wrong").context("Invalid private key");
        assert_eq!(ErrorKind::from_error(&err), ErrorKind::Auth);
    }

    #[test]
    fn error_kind_classifies_connection_failure_as_network() {
        let err = anyhow::anyhow!("timeout").context("Failed to connect to Polygon RPC");
        assert_eq!(ErrorKind::from_error(&err), ErrorKind::Network);
    }

    #[test]
    fn error_kind_classifies_not_found() {
        let err = anyhow::anyhow!("Comment not found");
        assert_eq!(ErrorKind::from_error(&err), ErrorKind::NotFound);
    }

    #[test]
    fn error_kind_classifies_bad_input() {
        let err = anyhow::anyhow!("Invalid token ID: abc");
        assert_eq!(ErrorKind::from_error(&err), ErrorKind::InvalidInput);
    }

    #[test]
    fn error_kind_classifies_config_errors() {
        let err = anyhow::anyhow!("parse error").context("Invalid JSON in config file");
        assert_eq!(ErrorKind::from_error(&err), ErrorKind::Config);
    }

    #[test]
    fn error_kind_defaults_to_internal() {
        let err = anyhow::anyhow!("something unexpected");
        assert_eq!(ErrorKind::from_error(&err), ErrorKind::Internal);
    }

    #[test]
    fn error_kind_exit_codes_are_distinct() {
        let codes: Vec<u8> = [
            ErrorKind::Auth,
            ErrorKind::Config,
            ErrorKind::Network,
            ErrorKind::NotFound,
            ErrorKind::InvalidInput,
            ErrorKind::Internal,
        ]
        .iter()
        .map(|k| k.exit_code())
        .collect();
        // all codes should be unique
        let mut deduped = codes.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(codes.len(), deduped.len());
    }

    #[test]
    fn auth_error_has_hint() {
        assert!(ErrorKind::Auth.hint().is_some());
    }

    #[test]
    fn internal_error_has_no_hint() {
        assert!(ErrorKind::Internal.hint().is_none());
    }
}
