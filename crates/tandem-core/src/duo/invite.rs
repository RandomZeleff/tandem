//! Short invite codes (`7K2P-QX9M`) without a server of ours: the host signs "my iroh
//! endpoint is X" with a key derived from the code and publishes it on iroh's public
//! pkarr relay (backed by the Mainline DHT). The guest derives the same key from the code
//! and reads the record back. Only someone who knows the code can find the host.

use iroh::{EndpointId, SecretKey};
use iroh_dns::pkarr::SignedPacket;
use sha2::{Digest, Sha256};

use super::fail;
use crate::error::Result;

/// iroh's public pkarr relay, also used by iroh itself to publish endpoint addresses.
const PKARR_RELAY: &str = "https://dns.iroh.link/pkarr";
/// DNS name of the record inside the signed packet.
const RECORD: &str = "_tandem";
const TTL_SECS: u32 = 30;

/// Crockford base 32: no I, L, O or U, so a code read aloud cannot be mistyped.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const LENGTH: usize = 8;

/// A new random code, shown as `XXXX-XXXX` (40 bits).
pub fn new_code() -> String {
    let random = uuid::Uuid::new_v4();
    let mut bits = u64::from_le_bytes(random.as_bytes()[..8].try_into().unwrap_or([0; 8]));
    let mut code = String::with_capacity(LENGTH + 1);
    for i in 0..LENGTH {
        if i == 4 {
            code.push('-');
        }
        code.push(ALPHABET[(bits & 31) as usize] as char);
        bits >>= 5;
    }
    code
}

/// The canonical form of what the player typed (`7k2p qx9m`, `7K2P-QX9M`…), or `None`
/// if it cannot be a code. Letters that look like digits are read as those digits.
pub fn normalize(input: &str) -> Option<String> {
    let code: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            c => c,
        })
        .collect();
    (code.len() == LENGTH && code.bytes().all(|b| ALPHABET.contains(&b))).then_some(code)
}

/// `XXXX-XXXX` for display.
pub fn display(code: &str) -> String {
    match normalize(code) {
        Some(c) => format!("{}-{}", &c[..4], &c[4..]),
        None => code.to_owned(),
    }
}

fn record_key(code: &str) -> SecretKey {
    let digest: [u8; 32] = Sha256::digest(format!("tandem-invite-v1:{code}")).into();
    SecretKey::from_bytes(&digest)
}

/// What a guest sends to show it was given the code (the host compares).
pub fn proof(code: &str) -> String {
    let digest = Sha256::digest(format!("tandem-proof-v1:{code}"));
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Publishes (or refreshes) the record pointing the code at the host's endpoint.
pub async fn publish(http: &reqwest::Client, code: &str, host: EndpointId) -> Result<()> {
    let key = record_key(code);
    let packet = SignedPacket::from_txt_strings(&key, RECORD, [format!("host={host}")], TTL_SECS)
        .map_err(fail)?;
    let response = http
        .put(format!("{PKARR_RELAY}/{}", key.public().to_z32()))
        .body(packet.to_relay_payload())
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(fail(format!(
            "le relais a répondu {}",
            response.status().as_u16()
        )));
    }
    Ok(())
}

/// The host's endpoint for a code, or `None` if nobody is hosting with it.
pub async fn resolve(http: &reqwest::Client, code: &str) -> Result<Option<EndpointId>> {
    let public = record_key(code).public();
    let response = http
        .get(format!("{PKARR_RELAY}/{}", public.to_z32()))
        .send()
        .await?;
    if response.status().as_u16() == 404 {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(fail(format!(
            "le relais a répondu {}",
            response.status().as_u16()
        )));
    }
    let body = response.bytes().await?;
    // Signed by the code's key: nobody without the code can forge it.
    let packet = SignedPacket::from_relay_payload(&public, &body).map_err(fail)?;
    Ok(packet
        .txt_records(RECORD)
        .iter()
        .find_map(|value| value.strip_prefix("host=")?.parse().ok()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_readable_and_forgiving() {
        let code = new_code();
        assert_eq!(code.len(), 9);
        assert_eq!(&code[4..5], "-");
        assert!(normalize(&code).is_some());
        assert_ne!(new_code(), new_code());

        assert_eq!(normalize(" 7k2p-qx9m ").as_deref(), Some("7K2PQX9M"));
        // Look-alike letters read as digits.
        assert_eq!(normalize("OIL0-ABCD").as_deref(), Some("0110ABCD"));
        assert_eq!(normalize("7K2P"), None);
        assert_eq!(normalize("7K2P-QX9U"), None);
        assert_eq!(display("7k2pqx9m"), "7K2P-QX9M");
    }

    #[test]
    fn keys_and_proofs_depend_only_on_the_code() {
        assert_eq!(
            record_key("7K2PQX9M").public(),
            record_key("7K2PQX9M").public()
        );
        assert_ne!(
            record_key("7K2PQX9M").public(),
            record_key("7K2PQX9N").public()
        );
        assert_eq!(proof("7K2PQX9M").len(), 64);
        assert_ne!(proof("7K2PQX9M"), proof("7K2PQX9N"));
    }
}
