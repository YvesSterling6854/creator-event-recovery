use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum DeliveryDecision {
    Rejected,
    DeliveryQueued { asset_id: String },
    SubscriberUpdateQueued { creator_id: String },
    ContentProcessingQueued { asset_id: String },
}

#[derive(Debug, PartialEq, Eq)]
pub enum CommerceError {
    InvalidSignature,
    UnknownEvent,
}

impl fmt::Display for CommerceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Decides the durable handoff before any downstream side effect occurs.
pub fn decide_delivery(payload: &str, signature: &str, secret: &str) -> Result<DeliveryDecision, CommerceError> {
    if !constant_time_eq(&hmac_sha256_hex(secret.as_bytes(), payload.as_bytes()), signature) {
        return Err(CommerceError::InvalidSignature);
    }

    let event = json_string(payload, "event").ok_or(CommerceError::UnknownEvent)?;
    let asset_id = json_string(payload, "asset_id");
    let creator_id = json_string(payload, "creator_id");
    match event.as_str() {
        "digital_asset.purchased" => Ok(DeliveryDecision::DeliveryQueued {
            asset_id: asset_id.ok_or(CommerceError::UnknownEvent)?,
        }),
        "subscriber.updated" => Ok(DeliveryDecision::SubscriberUpdateQueued {
            creator_id: creator_id.ok_or(CommerceError::UnknownEvent)?,
        }),
        "content.uploaded" => Ok(DeliveryDecision::ContentProcessingQueued {
            asset_id: asset_id.ok_or(CommerceError::UnknownEvent)?,
        }),
        _ => Err(CommerceError::UnknownEvent),
    }
}

pub fn queue_payload(decision: &DeliveryDecision) -> String {
    match decision {
        DeliveryDecision::DeliveryQueued { asset_id } => format!("{{\"kind\":\"signed_download\",\"asset_id\":\"{}\"}}", asset_id),
        DeliveryDecision::SubscriberUpdateQueued { creator_id } => format!("{{\"kind\":\"subscriber_update\",\"creator_id\":\"{}\"}}", creator_id),
        DeliveryDecision::ContentProcessingQueued { asset_id } => format!("{{\"kind\":\"content_processing\",\"asset_id\":\"{}\"}}", asset_id),
        DeliveryDecision::Rejected => "{\"kind\":\"rejected\"}".to_owned(),
    }
}

fn json_string(input: &str, key: &str) -> Option<String> {
    let marker = format!("\"{}\":\"", key);
    let start = input.find(&marker)? + marker.len();
    let end = input[start..].find('"')? + start;
    Some(input[start..end].to_owned())
}

fn constant_time_eq(left: &str, right: &str) -> bool {
    if left.len() != right.len() { return false; }
    left.bytes().zip(right.bytes()).fold(0u8, |diff, (a, b)| diff | (a ^ b)) == 0
}

// Small SHA-256/HMAC implementation keeps signature verification dependency-free.
fn hmac_sha256_hex(secret: &[u8], message: &[u8]) -> String {
    let mut key = [0u8; 64];
    if secret.len() > 64 { key[..32].copy_from_slice(&sha256(secret)); } else { key[..secret.len()].copy_from_slice(secret); }
    let mut inner = [0x36u8; 64];
    let mut outer = [0x5cu8; 64];
    for i in 0..64 { inner[i] ^= key[i]; outer[i] ^= key[i]; }
    let mut inside = inner.to_vec(); inside.extend_from_slice(message);
    let mut outside = outer.to_vec(); outside.extend_from_slice(&sha256(&inside));
    sha256(&outside).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn sha256(input: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2];
    let mut bytes = input.to_vec(); let bits = (bytes.len() as u64) * 8; bytes.push(0x80);
    while bytes.len() % 64 != 56 { bytes.push(0); } bytes.extend_from_slice(&bits.to_be_bytes());
    let mut state = [0x6a09e667u32,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19];
    for chunk in bytes.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 { w[i] = u32::from_be_bytes(chunk[i*4..i*4+4].try_into().unwrap()); }
        for i in 16..64 { let s0 = w[i-15].rotate_right(7)^w[i-15].rotate_right(18)^(w[i-15]>>3); let s1 = w[i-2].rotate_right(17)^w[i-2].rotate_right(19)^(w[i-2]>>10); w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1); }
        let (mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut h)=(state[0],state[1],state[2],state[3],state[4],state[5],state[6],state[7]);
        for i in 0..64 { let s1=e.rotate_right(6)^e.rotate_right(11)^e.rotate_right(25); let ch=(e&f)^(!e&g); let t1=h.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]); let s0=a.rotate_right(2)^a.rotate_right(13)^a.rotate_right(22); let maj=(a&b)^(a&c)^(b&c); let t2=s0.wrapping_add(maj); h=g;g=f;f=e;e=d.wrapping_add(t1);d=c;c=b;b=a;a=t1.wrapping_add(t2); }
        state[0]=state[0].wrapping_add(a);state[1]=state[1].wrapping_add(b);state[2]=state[2].wrapping_add(c);state[3]=state[3].wrapping_add(d);state[4]=state[4].wrapping_add(e);state[5]=state[5].wrapping_add(f);state[6]=state[6].wrapping_add(g);state[7]=state[7].wrapping_add(h);
    }
    let mut out=[0u8;32]; for (i, word) in state.iter().enumerate(){out[i*4..i*4+4].copy_from_slice(&word.to_be_bytes());} out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paid_asset_becomes_a_download_job_only_with_a_valid_signature() {
        let body = "{\"event\":\"digital_asset.purchased\",\"asset_id\":\"lesson-7\"}";
        let signature = hmac_sha256_hex(b"local-signing-secret", body.as_bytes());
        assert_eq!(decide_delivery(body, &signature, "local-signing-secret"), Ok(DeliveryDecision::DeliveryQueued { asset_id: "lesson-7".into() }));
        assert_eq!(decide_delivery(body, "bad", "local-signing-secret"), Err(CommerceError::InvalidSignature));
    }
}
