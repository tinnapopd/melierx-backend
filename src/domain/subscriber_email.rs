use validator::Validate;

// Public Types
#[derive(Debug)]
pub struct SubscriberEmail(String);

// Private Types
#[derive(Validate)]
struct EmailWrapper {
    #[validate(email)]
    email: String,
}

// Implementations
impl SubscriberEmail {
    pub fn parse(s: String) -> Result<Self, String> {
        let validator = EmailWrapper { email: s.clone() };
        if validator.validate().is_ok() {
            Ok(Self(s))
        } else {
            Err(format!("'{}' is not a valid subscriber email.", s))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::SubscriberEmail;
    use claim::assert_err;
    use fake::Fake;
    use fake::faker::internet::en::SafeEmail;

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "tinnapopduangthagmail.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn valid_emails_are_parsed_successfully() {
        let email = SafeEmail().fake();
        claim::assert_ok!(SubscriberEmail::parse(email));
    }

    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary(_: &mut quickcheck::Gen) -> Self {
            let email: String = SafeEmail().fake();
            Self(email)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_emails_are_parsed_successfully_quickcheck(valid_email: ValidEmailFixture) -> bool {
        SubscriberEmail::parse(valid_email.0).is_ok()
    }
}
