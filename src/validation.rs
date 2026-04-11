/// Pure business-logic validation. No Worker/WASM dependencies — fully testable
/// with `cargo test` on any native target.

/// Maximum single expense: $1,000,000.00 (in cents)
const MAX_AMOUNT_CENTS: i64 = 100_000_000;

/// Valid ISO 4217 currency codes accepted by the app
const SUPPORTED_CURRENCIES: &[&str] = &[
    "USD", "EUR", "GBP", "JPY", "AUD", "CAD", "CHF", "CNY", "INR",
    "KRW", "BRL", "MXN", "SGD", "HKD", "SEK", "NOK", "DKK", "NZD",
];

#[derive(Debug, PartialEq)]
pub enum ValidationError {
    AmountNotPositive,
    AmountExceedsMaximum,
    InvalidDateFormat,
    InvalidCurrencyCode,
    DescriptionTooLong,
    NameEmpty,
    NameTooLong,
    EmojiEmpty,
    InvalidEmail,
    PasswordTooShort,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AmountNotPositive => write!(f, "Amount must be greater than zero"),
            Self::AmountExceedsMaximum => write!(f, "Amount exceeds maximum of $1,000,000"),
            Self::InvalidDateFormat => write!(f, "Date must be in YYYY-MM-DD format"),
            Self::InvalidCurrencyCode => write!(f, "Unsupported currency code"),
            Self::DescriptionTooLong => write!(f, "Description must be 500 characters or fewer"),
            Self::NameEmpty => write!(f, "Name cannot be empty"),
            Self::NameTooLong => write!(f, "Name must be 100 characters or fewer"),
            Self::EmojiEmpty => write!(f, "Emoji cannot be empty"),
            Self::InvalidEmail => write!(f, "Invalid email address"),
            Self::PasswordTooShort => write!(f, "Password must be at least 8 characters"),
        }
    }
}

/// Validates that an expense amount (in cents) is within accepted bounds.
pub fn validate_amount_cents(amount: i64) -> Result<(), ValidationError> {
    if amount <= 0 {
        return Err(ValidationError::AmountNotPositive);
    }
    if amount > MAX_AMOUNT_CENTS {
        return Err(ValidationError::AmountExceedsMaximum);
    }
    Ok(())
}

/// Validates an ISO 8601 date string in `YYYY-MM-DD` format.
pub fn validate_date(date: &str) -> Result<(), ValidationError> {
    let bytes = date.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes[..4].iter().all(|b| b.is_ascii_digit())
        || !bytes[5..7].iter().all(|b| b.is_ascii_digit())
        || !bytes[8..].iter().all(|b| b.is_ascii_digit())
    {
        return Err(ValidationError::InvalidDateFormat);
    }

    let month: u8 = date[5..7].parse().unwrap_or(0);
    let day: u8 = date[8..].parse().unwrap_or(0);

    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(ValidationError::InvalidDateFormat);
    }
    Ok(())
}

/// Validates an ISO 4217 currency code against the supported list.
pub fn validate_currency(code: &str) -> Result<(), ValidationError> {
    if SUPPORTED_CURRENCIES.contains(&code) {
        Ok(())
    } else {
        Err(ValidationError::InvalidCurrencyCode)
    }
}

/// Validates a category or tag name.
pub fn validate_name(name: &str) -> Result<(), ValidationError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(ValidationError::NameEmpty);
    }
    if trimmed.len() > 100 {
        return Err(ValidationError::NameTooLong);
    }
    Ok(())
}

/// Validates a category emoji field.
pub fn validate_emoji(emoji: &str) -> Result<(), ValidationError> {
    if emoji.trim().is_empty() {
        return Err(ValidationError::EmojiEmpty);
    }
    Ok(())
}

/// Validates an expense description (optional, but length-bounded).
pub fn validate_description(description: &str) -> Result<(), ValidationError> {
    if description.len() > 500 {
        return Err(ValidationError::DescriptionTooLong);
    }
    Ok(())
}

/// Validates an email address (RFC 5321 simplified: local@domain.tld).
pub fn validate_email(email: &str) -> Result<(), ValidationError> {
    let trimmed = email.trim();
    // Must have exactly one '@'
    let at_pos = match trimmed.find('@') {
        Some(p) if trimmed.rfind('@') == Some(p) => p,
        _ => return Err(ValidationError::InvalidEmail),
    };
    let local = &trimmed[..at_pos];
    let domain = &trimmed[at_pos + 1..];

    // local part: 1–64 chars, no leading/trailing dot
    if local.is_empty() || local.len() > 64 {
        return Err(ValidationError::InvalidEmail);
    }
    if local.starts_with('.') || local.ends_with('.') {
        return Err(ValidationError::InvalidEmail);
    }

    // domain: must have at least one dot, no leading/trailing dot or hyphen
    if !domain.contains('.') || domain.starts_with('.') || domain.ends_with('.') {
        return Err(ValidationError::InvalidEmail);
    }
    // All chars must be alphanumeric, dot, or hyphen
    if !domain
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return Err(ValidationError::InvalidEmail);
    }
    Ok(())
}

/// Validates that a password meets minimum strength requirements.
pub fn validate_password(password: &str) -> Result<(), ValidationError> {
    if password.len() < 8 {
        return Err(ValidationError::PasswordTooShort);
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Amount validation ────────────────────────────────────────────────────

    #[test]
    fn amount_positive_is_valid() {
        assert!(validate_amount_cents(1).is_ok());
        assert!(validate_amount_cents(100).is_ok());
        assert!(validate_amount_cents(9_99).is_ok()); // $9.99
        assert!(validate_amount_cents(MAX_AMOUNT_CENTS).is_ok());
    }

    #[test]
    fn amount_boundary_at_max_is_valid() {
        assert!(validate_amount_cents(MAX_AMOUNT_CENTS).is_ok());
        assert_eq!(
            validate_amount_cents(MAX_AMOUNT_CENTS + 1),
            Err(ValidationError::AmountExceedsMaximum)
        );
    }

    #[test]
    fn amount_zero_is_invalid() {
        assert_eq!(
            validate_amount_cents(0),
            Err(ValidationError::AmountNotPositive)
        );
    }

    #[test]
    fn amount_negative_is_invalid() {
        assert_eq!(
            validate_amount_cents(-1),
            Err(ValidationError::AmountNotPositive)
        );
        assert_eq!(
            validate_amount_cents(-9999),
            Err(ValidationError::AmountNotPositive)
        );
    }

    #[test]
    fn amount_over_maximum_is_invalid() {
        assert_eq!(
            validate_amount_cents(MAX_AMOUNT_CENTS + 1),
            Err(ValidationError::AmountExceedsMaximum)
        );
    }

    #[test]
    fn amount_extreme_values() {
        assert_eq!(
            validate_amount_cents(i64::MIN),
            Err(ValidationError::AmountNotPositive)
        );
        assert_eq!(
            validate_amount_cents(i64::MAX),
            Err(ValidationError::AmountExceedsMaximum)
        );
    }

    // ── Date validation ──────────────────────────────────────────────────────

    #[test]
    fn valid_dates_are_accepted() {
        assert!(validate_date("2024-01-15").is_ok());
        assert!(validate_date("2026-04-11").is_ok());
        assert!(validate_date("2000-12-31").is_ok());
    }

    #[test]
    fn date_wrong_separator_is_rejected() {
        assert_eq!(
            validate_date("2024/01/15"),
            Err(ValidationError::InvalidDateFormat)
        );
        assert_eq!(
            validate_date("2024.01.15"),
            Err(ValidationError::InvalidDateFormat)
        );
    }

    #[test]
    fn date_wrong_length_is_rejected() {
        assert_eq!(
            validate_date("20240115"),
            Err(ValidationError::InvalidDateFormat)
        );
        assert_eq!(
            validate_date("24-1-1"),
            Err(ValidationError::InvalidDateFormat)
        );
    }

    #[test]
    fn date_invalid_month_is_rejected() {
        assert_eq!(
            validate_date("2024-00-01"),
            Err(ValidationError::InvalidDateFormat)
        );
        assert_eq!(
            validate_date("2024-13-01"),
            Err(ValidationError::InvalidDateFormat)
        );
    }

    #[test]
    fn date_non_numeric_is_rejected() {
        assert_eq!(
            validate_date("YYYY-MM-DD"),
            Err(ValidationError::InvalidDateFormat)
        );
    }

    #[test]
    fn date_empty_string_is_rejected() {
        assert_eq!(validate_date(""), Err(ValidationError::InvalidDateFormat));
    }

    #[test]
    fn date_day_zero_is_rejected() {
        assert_eq!(
            validate_date("2024-01-00"),
            Err(ValidationError::InvalidDateFormat)
        );
    }

    #[test]
    fn date_day_32_is_rejected() {
        assert_eq!(
            validate_date("2024-01-32"),
            Err(ValidationError::InvalidDateFormat)
        );
    }

    #[test]
    fn date_leading_trailing_spaces_rejected() {
        assert_eq!(
            validate_date(" 2024-01-15"),
            Err(ValidationError::InvalidDateFormat)
        );
        assert_eq!(
            validate_date("2024-01-15 "),
            Err(ValidationError::InvalidDateFormat)
        );
    }

    // ── Currency validation ──────────────────────────────────────────────────

    #[test]
    fn supported_currencies_are_valid() {
        assert!(validate_currency("USD").is_ok());
        assert!(validate_currency("EUR").is_ok());
        assert!(validate_currency("GBP").is_ok());
        assert!(validate_currency("JPY").is_ok());
        assert!(validate_currency("INR").is_ok());
    }

    #[test]
    fn all_supported_currencies_are_valid() {
        for code in super::SUPPORTED_CURRENCIES {
            assert!(validate_currency(code).is_ok(), "Expected {} to be valid", code);
        }
    }

    #[test]
    fn unsupported_currency_is_rejected() {
        assert_eq!(
            validate_currency("XYZ"),
            Err(ValidationError::InvalidCurrencyCode)
        );
        assert_eq!(
            validate_currency("usd"), // case-sensitive
            Err(ValidationError::InvalidCurrencyCode)
        );
        assert_eq!(
            validate_currency(""),
            Err(ValidationError::InvalidCurrencyCode)
        );
    }

    // ── Name validation ──────────────────────────────────────────────────────

    #[test]
    fn valid_names_are_accepted() {
        assert!(validate_name("Food").is_ok());
        assert!(validate_name("  Transport  ").is_ok()); // leading/trailing spaces ok
        assert!(validate_name(&"a".repeat(100)).is_ok());
    }

    #[test]
    fn unicode_names_are_accepted() {
        assert!(validate_name("食品").is_ok());
        assert!(validate_name("Essen & Trinken").is_ok());
        assert!(validate_name("カテゴリー").is_ok());
    }

    #[test]
    fn empty_name_is_rejected() {
        assert_eq!(validate_name(""), Err(ValidationError::NameEmpty));
        assert_eq!(validate_name("   "), Err(ValidationError::NameEmpty));
    }

    #[test]
    fn name_over_100_chars_is_rejected() {
        assert_eq!(
            validate_name(&"a".repeat(101)),
            Err(ValidationError::NameTooLong)
        );
    }

    // ── Description validation ───────────────────────────────────────────────

    #[test]
    fn description_within_limit_is_valid() {
        assert!(validate_description("").is_ok());
        assert!(validate_description("Coffee at Starbucks").is_ok());
        assert!(validate_description(&"x".repeat(500)).is_ok());
    }

    #[test]
    fn description_at_boundary_is_valid() {
        assert!(validate_description(&"x".repeat(500)).is_ok());
    }

    #[test]
    fn description_unicode_counted_by_bytes() {
        // Multi-byte chars: "é" is 2 bytes in UTF-8
        let desc = "é".repeat(251); // 502 bytes
        assert_eq!(
            validate_description(&desc),
            Err(ValidationError::DescriptionTooLong)
        );
    }

    #[test]
    fn description_over_500_chars_is_rejected() {
        assert_eq!(
            validate_description(&"x".repeat(501)),
            Err(ValidationError::DescriptionTooLong)
        );
    }

    // ── Emoji validation ─────────────────────────────────────────────────────

    #[test]
    fn non_empty_emoji_is_valid() {
        assert!(validate_emoji("☕").is_ok());
        assert!(validate_emoji("🍔").is_ok());
    }

    #[test]
    fn multi_codepoint_emoji_is_valid() {
        assert!(validate_emoji("👨‍👩‍👧‍👦").is_ok()); // family emoji (ZWJ sequence)
        assert!(validate_emoji("🏳️‍🌈").is_ok());
    }

    #[test]
    fn empty_emoji_is_rejected() {
        assert_eq!(validate_emoji(""), Err(ValidationError::EmojiEmpty));
        assert_eq!(validate_emoji("  "), Err(ValidationError::EmojiEmpty));
    }

    // ── Email validation ──────────────────────────────────────────────────────

    #[test]
    fn valid_emails_are_accepted() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("first.last@sub.domain.org").is_ok());
        assert!(validate_email("user+tag@example.co.uk").is_ok());
        assert!(validate_email("  user@example.com  ").is_ok()); // trimmed
    }

    #[test]
    fn email_without_at_is_rejected() {
        assert_eq!(
            validate_email("userexample.com"),
            Err(ValidationError::InvalidEmail)
        );
    }

    #[test]
    fn email_with_multiple_at_is_rejected() {
        assert_eq!(
            validate_email("a@b@c.com"),
            Err(ValidationError::InvalidEmail)
        );
    }

    #[test]
    fn email_without_domain_dot_is_rejected() {
        assert_eq!(
            validate_email("user@localhost"),
            Err(ValidationError::InvalidEmail)
        );
    }

    #[test]
    fn empty_local_part_is_rejected() {
        assert_eq!(
            validate_email("@example.com"),
            Err(ValidationError::InvalidEmail)
        );
    }

    #[test]
    fn email_leading_dot_in_local_is_rejected() {
        assert_eq!(
            validate_email(".user@example.com"),
            Err(ValidationError::InvalidEmail)
        );
    }

    #[test]
    fn email_trailing_dot_in_local_is_rejected() {
        assert_eq!(
            validate_email("user.@example.com"),
            Err(ValidationError::InvalidEmail)
        );
    }

    #[test]
    fn email_empty_string_is_rejected() {
        assert_eq!(validate_email(""), Err(ValidationError::InvalidEmail));
    }

    #[test]
    fn email_domain_leading_dot_is_rejected() {
        assert_eq!(
            validate_email("user@.example.com"),
            Err(ValidationError::InvalidEmail)
        );
    }

    #[test]
    fn email_domain_trailing_dot_is_rejected() {
        assert_eq!(
            validate_email("user@example.com."),
            Err(ValidationError::InvalidEmail)
        );
    }

    #[test]
    fn email_local_part_too_long_is_rejected() {
        let long_local = "a".repeat(65);
        assert_eq!(
            validate_email(&format!("{}@example.com", long_local)),
            Err(ValidationError::InvalidEmail)
        );
    }

    #[test]
    fn email_local_part_at_max_length_is_valid() {
        let local = "a".repeat(64);
        assert!(validate_email(&format!("{}@example.com", local)).is_ok());
    }

    // ── Password validation ───────────────────────────────────────────────────

    #[test]
    fn password_of_8_chars_is_valid() {
        assert!(validate_password("12345678").is_ok());
        assert!(validate_password("correct-horse").is_ok());
    }

    #[test]
    fn short_password_is_rejected() {
        assert_eq!(
            validate_password("1234567"),
            Err(ValidationError::PasswordTooShort)
        );
        assert_eq!(
            validate_password(""),
            Err(ValidationError::PasswordTooShort)
        );
    }

    #[test]
    fn password_exactly_8_chars_boundary() {
        assert!(validate_password("12345678").is_ok());
        assert_eq!(
            validate_password("1234567"),
            Err(ValidationError::PasswordTooShort)
        );
    }

    #[test]
    fn password_long_is_valid() {
        assert!(validate_password(&"a".repeat(256)).is_ok());
    }

    // ── Display trait ─────────────────────────────────────────────────────────

    #[test]
    fn error_display_messages() {
        assert_eq!(
            ValidationError::AmountNotPositive.to_string(),
            "Amount must be greater than zero"
        );
        assert_eq!(
            ValidationError::AmountExceedsMaximum.to_string(),
            "Amount exceeds maximum of $1,000,000"
        );
        assert_eq!(
            ValidationError::InvalidDateFormat.to_string(),
            "Date must be in YYYY-MM-DD format"
        );
        assert_eq!(
            ValidationError::InvalidCurrencyCode.to_string(),
            "Unsupported currency code"
        );
        assert_eq!(
            ValidationError::DescriptionTooLong.to_string(),
            "Description must be 500 characters or fewer"
        );
        assert_eq!(
            ValidationError::NameEmpty.to_string(),
            "Name cannot be empty"
        );
        assert_eq!(
            ValidationError::NameTooLong.to_string(),
            "Name must be 100 characters or fewer"
        );
        assert_eq!(
            ValidationError::EmojiEmpty.to_string(),
            "Emoji cannot be empty"
        );
        assert_eq!(
            ValidationError::InvalidEmail.to_string(),
            "Invalid email address"
        );
        assert_eq!(
            ValidationError::PasswordTooShort.to_string(),
            "Password must be at least 8 characters"
        );
    }
}
