use sha2::{Digest, Sha256};

#[derive(Debug)]
pub enum Error {
    MissingNonce,
    InvalidNonce,
}

pub trait Validatable {
    fn payload(&self) -> &str;
    fn nonce(&self) -> Option<&str>;
}

pub trait ProofOfWorkService: Clone + Send + Sync + 'static {
    fn is_valid(&self, validatable: &impl Validatable) -> Result<(), Error>;
}

#[derive(Clone)]
pub struct Sha256ProofOfWorkService {
    cost: usize,
}

impl Sha256ProofOfWorkService {
    pub fn new(cost: usize) -> Sha256ProofOfWorkService {
        Sha256ProofOfWorkService { cost }
    }
}

impl ProofOfWorkService for Sha256ProofOfWorkService {
    fn is_valid(&self, validatable: &impl Validatable) -> Result<(), Error> {
        if self.cost == 0 {
            return Ok(());
        }

        let nonce = validatable.nonce().ok_or(Error::MissingNonce)?;
        let hash = format!(
            "{:x}",
            Sha256::digest(format!("{}{}", validatable.payload(), nonce).as_bytes())
        );

        let pattern = "0".repeat(self.cost);
        if hash.starts_with(&pattern) {
            Ok(())
        } else {
            Err(Error::InvalidNonce)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct TestingValidatable {
        payload: String,
        nonce: Option<String>,
    }

    impl Validatable for TestingValidatable {
        fn payload(&self) -> &str {
            &self.payload
        }

        fn nonce(&self) -> Option<&str> {
            self.nonce.as_deref()
        }
    }

    #[test]
    fn can_bypass_with_a_cost_of_zero() {
        let pow = Sha256ProofOfWorkService::new(0);
        let validatable = TestingValidatable {
            payload: "test".to_string(),
            nonce: None,
        };
        assert!(pow.is_valid(&validatable).is_ok());
    }

    #[test]
    fn requires_a_nonce_with_a_cost_above_zero() {
        let pow = Sha256ProofOfWorkService::new(1);
        let validatable = TestingValidatable {
            payload: "test".to_string(),
            nonce: None,
        };
        assert!(matches!(
            pow.is_valid(&validatable),
            Err(Error::MissingNonce)
        ));
    }

    #[test]
    fn rejects_an_invalid_nonce() {
        let pow = Sha256ProofOfWorkService::new(1);
        let validatable = TestingValidatable {
            payload: "test".to_string(),
            nonce: Some("invalid".to_string()),
        };
        assert!(pow.is_valid(&validatable).is_err());
    }

    #[test]
    fn accepts_a_valid_nonce() {
        let pow = Sha256ProofOfWorkService::new(4);
        let validatable = TestingValidatable {
            payload: "test".to_string(),
            nonce: Some("93721".to_string()),
        };
        assert!(pow.is_valid(&validatable).is_ok());
    }

    #[test]
    fn rejects_a_nonce_that_is_too_weak() {
        let pow = Sha256ProofOfWorkService::new(5);
        let validatable = TestingValidatable {
            payload: "test".to_string(),
            nonce: Some("93721".to_string()),
        };
        assert!(pow.is_valid(&validatable).is_err());
    }
}
