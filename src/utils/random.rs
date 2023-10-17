use ring::rand::{SecureRandom, SystemRandom};

use super::errors::RandomGenerationError;

// generates length bytes random
pub fn generate_random(length: usize) -> Result<Vec<u8>, RandomGenerationError> {
    let mut random_bytes = vec![0u8; length];
    let rng = SystemRandom::new();
    rng.fill(&mut random_bytes)?;

    Ok(random_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_random() {
        match generate_random(32) {
            Ok(random_bytes) => {
                assert_eq!(random_bytes.len(), 32);
            }
            Err(_) => {
                panic!("generate_random function returned an error");
            }
        }
    }

    #[tokio::test]
    async fn test_generated_numbers_not_equal() {
        let random_bytes1 = generate_random(8).expect("Failed to generate random bytes");
        let random_bytes2 = generate_random(8).expect("Failed to generate random bytes");

        assert_ne!(random_bytes1, random_bytes2);
    }
}
