use crypto_hash::{hex_digest, Algorithm};

pub fn hash(text: &str) -> String {
    hex_digest(
        Algorithm::MD5,
        hex_digest(Algorithm::SHA256, text.as_bytes()).as_bytes(),
    )
}

#[cfg(test)]
mod tests {
    use super::super::str::random_string;
    use super::*;

    #[test]
    fn test_random_string() {
        for i in 1..100 {
            assert_eq!(random_string(i).len(), i);
        }
    }
}
