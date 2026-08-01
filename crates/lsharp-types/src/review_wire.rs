//! v0.3 review attestation/lifecycle の versioned JSON wire boundary。
//!
//! serde の default map deserializer は duplicate key を last-wins にするため、ここでは
//! root、attestation、lifecycle の各 object を custom visitor で読む。unknown field と
//! duplicate field を同じ schema boundary で拒否し、Rust/selfhost/native が implicit な
//! JSON parser 差分を持ち込まないようにする。

use super::review_attestation::{AttestationAlgorithm, AttestationError, ReviewAttestation};
use super::review_lifecycle::{
    LifecycleError, ReviewLifecycleEvent, ReviewLifecycleRegistry, ReviewLifecycleState,
};
use super::review_trust_store::{ReviewTrustKey, ReviewTrustStore, TrustStoreError};
use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fmt;

pub const REVIEW_WIRE_SCHEMA_VERSION: u64 = 1;

/// review wire の parse/projection error。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReviewWireError {
    #[error("review wire schema error: {message}")]
    Schema { message: String },
    #[error("review wire schema version が未対応です: {version}")]
    UnsupportedVersion { version: u64 },
    #[error("review attestation の wire 変換に失敗しました: {0}")]
    Attestation(#[from] AttestationError),
    #[error("review lifecycle の wire 変換に失敗しました: {0}")]
    Lifecycle(#[from] LifecycleError),
    #[error("review trust store の wire 変換に失敗しました: {0}")]
    TrustStore(#[from] TrustStoreError),
    #[error("review attestation の signature encoding が不正です: {value:?}")]
    InvalidSignatureEncoding { value: String },
    #[error("review trust store の public key encoding が不正です: {value:?}")]
    InvalidPublicKeyEncoding { value: String },
    #[error("review lifecycle の state が不正です: {value:?}")]
    InvalidLifecycleState { value: String },
    #[error("review wire の JSON projection に失敗しました: {message}")]
    Serialization { message: String },
}

/// attestation と lifecycle snapshot を束ねた versioned document。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewWireDocument {
    schema_version: u64,
    attestations: Vec<ReviewAttestation>,
    lifecycle: ReviewLifecycleRegistry,
    trust_store: Option<ReviewTrustStore>,
}

impl ReviewWireDocument {
    pub fn schema_version(&self) -> u64 {
        self.schema_version
    }

    pub fn attestations(&self) -> &[ReviewAttestation] {
        &self.attestations
    }

    pub fn lifecycle(&self) -> &ReviewLifecycleRegistry {
        &self.lifecycle
    }

    pub fn trust_store(&self) -> Option<&ReviewTrustStore> {
        self.trust_store.as_ref()
    }

    /// deterministic な object/array projection。
    pub fn to_json_value(&self) -> Value {
        let mut attestations = self.attestations.iter().collect::<Vec<_>>();
        attestations.sort_by(|left, right| {
            left.review_id()
                .as_str()
                .cmp(right.review_id().as_str())
                .then_with(|| left.sequence().cmp(&right.sequence()))
        });
        let attestation_values = attestations
            .into_iter()
            .map(|attestation| {
                json!({
                    "algorithm": attestation.algorithm().as_str(),
                    "expires_at": attestation.expires_at(),
                    "issued_at": attestation.issued_at(),
                    "key_id": attestation.key_id(),
                    "provenance_digest": attestation.provenance_digest(),
                    "provider": attestation.provider(),
                    "review_id": attestation.review_id().as_str(),
                    "sequence": attestation.sequence(),
                    "signature": encode_base64url(attestation.signature()),
                    "source_commit": attestation.source_commit(),
                    "subject_digest": attestation.subject_digest(),
                })
            })
            .collect::<Vec<_>>();

        let lifecycle_events = self.lifecycle.events();
        let lifecycle_values = lifecycle_events
            .into_iter()
            .map(|event| {
                json!({
                    "effective_at": event.effective_at(),
                    "reason_digest": event.reason_digest(),
                    "review_id": event.review_id().as_str(),
                    "sequence": event.sequence(),
                    "state": event.state().as_str(),
                })
            })
            .collect::<Vec<_>>();

        let mut root = serde_json::Map::new();
        root.insert("attestations".to_string(), json!(attestation_values));
        root.insert("lifecycle".to_string(), json!(lifecycle_values));
        root.insert("schema_version".to_string(), json!(self.schema_version));
        if let Some(trust_store) = &self.trust_store {
            let keys = trust_store
                .entries()
                .into_iter()
                .map(|key| {
                    json!({
                        "algorithm": key.algorithm().as_str(),
                        "active": key.is_active(),
                        "key_id": key.key_id(),
                        "provider": key.provider(),
                        "public_key": encode_base64url(key.public_key()),
                    })
                })
                .collect::<Vec<_>>();
            root.insert("trust_store".to_string(), json!(keys));
        }
        Value::Object(root)
    }

    pub fn to_json_string(&self) -> Result<String, ReviewWireError> {
        serde_json::to_string(&self.to_json_value()).map_err(|error| {
            ReviewWireError::Serialization {
                message: error.to_string(),
            }
        })
    }
}

/// version 1 review wire を parse する。
pub fn parse_review_wire(input: &str) -> Result<ReviewWireDocument, ReviewWireError> {
    let wire: WireDocument =
        serde_json::from_str(input).map_err(|error| ReviewWireError::Schema {
            message: error.to_string(),
        })?;
    if wire.schema_version != REVIEW_WIRE_SCHEMA_VERSION {
        return Err(ReviewWireError::UnsupportedVersion {
            version: wire.schema_version,
        });
    }

    let attestations = wire
        .attestations
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<ReviewAttestation>, ReviewWireError>>()?;
    let lifecycle_events = wire
        .lifecycle
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<ReviewLifecycleEvent>, ReviewWireError>>()?;
    let lifecycle = ReviewLifecycleRegistry::from_events(lifecycle_events)?;
    let trust_store = match wire.trust_store {
        Some(keys) => {
            let mut store = ReviewTrustStore::default();
            for key in keys {
                store.add_key(key.try_into()?)?;
            }
            Some(store)
        }
        None => None,
    };

    Ok(ReviewWireDocument {
        schema_version: wire.schema_version,
        attestations,
        lifecycle,
        trust_store,
    })
}

#[derive(Debug)]
struct WireDocument {
    schema_version: u64,
    attestations: Vec<AttestationWire>,
    lifecycle: Vec<LifecycleWire>,
    trust_store: Option<Vec<TrustKeyWire>>,
}

impl<'de> Deserialize<'de> for WireDocument {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(DocumentVisitor)
    }
}

struct DocumentVisitor;

impl<'de> Visitor<'de> for DocumentVisitor {
    type Value = WireDocument;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("review wire document object")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut seen = BTreeSet::new();
        let mut schema_version = None;
        let mut attestations = None;
        let mut lifecycle = None;
        let mut trust_store = None;
        while let Some(key) = map.next_key::<String>()? {
            if !seen.insert(key.clone()) {
                return Err(de::Error::custom(format!("duplicate field document.{key}")));
            }
            match key.as_str() {
                "schema_version" => schema_version = Some(map.next_value::<u64>()?),
                "attestations" => attestations = Some(map.next_value::<Vec<AttestationWire>>()?),
                "lifecycle" => lifecycle = Some(map.next_value::<Vec<LifecycleWire>>()?),
                "trust_store" => trust_store = Some(map.next_value::<Vec<TrustKeyWire>>()?),
                _ => return Err(de::Error::custom(format!("unknown field document.{key}"))),
            }
        }
        Ok(WireDocument {
            schema_version: required(schema_version, "document.schema_version")?,
            attestations: required(attestations, "document.attestations")?,
            lifecycle: required(lifecycle, "document.lifecycle")?,
            trust_store,
        })
    }
}

#[derive(Debug)]
struct AttestationWire {
    review_id: String,
    subject_digest: String,
    source_commit: String,
    provenance_digest: String,
    provider: String,
    key_id: String,
    algorithm: String,
    signature: String,
    issued_at: String,
    expires_at: Option<String>,
    sequence: u64,
}

impl TryFrom<AttestationWire> for ReviewAttestation {
    type Error = ReviewWireError;

    fn try_from(value: AttestationWire) -> Result<Self, Self::Error> {
        let signature = decode_base64url(&value.signature)?;
        let algorithm = AttestationAlgorithm::parse(value.algorithm)?;
        Ok(ReviewAttestation::new(
            value.review_id,
            value.subject_digest,
            value.source_commit,
            value.provenance_digest,
            value.provider,
            value.key_id,
            algorithm,
            value.issued_at,
            value.expires_at,
            value.sequence,
            signature,
        )?)
    }
}

impl<'de> Deserialize<'de> for AttestationWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(AttestationVisitor)
    }
}

struct AttestationVisitor;

impl<'de> Visitor<'de> for AttestationVisitor {
    type Value = AttestationWire;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("review attestation object")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut seen = BTreeSet::new();
        let mut review_id = None;
        let mut subject_digest = None;
        let mut source_commit = None;
        let mut provenance_digest = None;
        let mut provider = None;
        let mut key_id = None;
        let mut algorithm = None;
        let mut signature = None;
        let mut issued_at = None;
        let mut expires_at = None;
        let mut sequence = None;
        while let Some(key) = map.next_key::<String>()? {
            if !seen.insert(key.clone()) {
                return Err(de::Error::custom(format!(
                    "duplicate field attestation.{key}"
                )));
            }
            match key.as_str() {
                "review_id" => review_id = Some(map.next_value::<String>()?),
                "subject_digest" => subject_digest = Some(map.next_value::<String>()?),
                "source_commit" => source_commit = Some(map.next_value::<String>()?),
                "provenance_digest" => provenance_digest = Some(map.next_value::<String>()?),
                "provider" => provider = Some(map.next_value::<String>()?),
                "key_id" => key_id = Some(map.next_value::<String>()?),
                "algorithm" => algorithm = Some(map.next_value::<String>()?),
                "signature" => signature = Some(map.next_value::<String>()?),
                "issued_at" => issued_at = Some(map.next_value::<String>()?),
                "expires_at" => expires_at = Some(map.next_value::<Option<String>>()?),
                "sequence" => sequence = Some(map.next_value::<u64>()?),
                _ => {
                    return Err(de::Error::custom(format!(
                        "unknown field attestation.{key}"
                    )))
                }
            }
        }
        Ok(AttestationWire {
            review_id: required(review_id, "attestation.review_id")?,
            subject_digest: required(subject_digest, "attestation.subject_digest")?,
            source_commit: required(source_commit, "attestation.source_commit")?,
            provenance_digest: required(provenance_digest, "attestation.provenance_digest")?,
            provider: required(provider, "attestation.provider")?,
            key_id: required(key_id, "attestation.key_id")?,
            algorithm: required(algorithm, "attestation.algorithm")?,
            signature: required(signature, "attestation.signature")?,
            issued_at: required(issued_at, "attestation.issued_at")?,
            expires_at: expires_at.unwrap_or(None),
            sequence: required(sequence, "attestation.sequence")?,
        })
    }
}

#[derive(Debug)]
struct LifecycleWire {
    review_id: String,
    sequence: u64,
    state: String,
    effective_at: String,
    reason_digest: Option<String>,
}

impl TryFrom<LifecycleWire> for ReviewLifecycleEvent {
    type Error = ReviewWireError;

    fn try_from(value: LifecycleWire) -> Result<Self, Self::Error> {
        let state = match value.state.as_str() {
            "proposed" => ReviewLifecycleState::Proposed,
            "active" => ReviewLifecycleState::Active,
            "superseded" => ReviewLifecycleState::Superseded,
            "revoked" => ReviewLifecycleState::Revoked,
            _ => return Err(ReviewWireError::InvalidLifecycleState { value: value.state }),
        };
        Ok(ReviewLifecycleEvent::new(
            value.review_id,
            value.sequence,
            state,
            value.effective_at,
            value.reason_digest,
        )?)
    }
}

impl<'de> Deserialize<'de> for LifecycleWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(LifecycleVisitor)
    }
}

struct LifecycleVisitor;

impl<'de> Visitor<'de> for LifecycleVisitor {
    type Value = LifecycleWire;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("review lifecycle object")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut seen = BTreeSet::new();
        let mut review_id = None;
        let mut sequence = None;
        let mut state = None;
        let mut effective_at = None;
        let mut reason_digest = None;
        while let Some(key) = map.next_key::<String>()? {
            if !seen.insert(key.clone()) {
                return Err(de::Error::custom(format!(
                    "duplicate field lifecycle.{key}"
                )));
            }
            match key.as_str() {
                "review_id" => review_id = Some(map.next_value::<String>()?),
                "sequence" => sequence = Some(map.next_value::<u64>()?),
                "state" => state = Some(map.next_value::<String>()?),
                "effective_at" => effective_at = Some(map.next_value::<String>()?),
                "reason_digest" => reason_digest = Some(map.next_value::<Option<String>>()?),
                _ => return Err(de::Error::custom(format!("unknown field lifecycle.{key}"))),
            }
        }
        Ok(LifecycleWire {
            review_id: required(review_id, "lifecycle.review_id")?,
            sequence: required(sequence, "lifecycle.sequence")?,
            state: required(state, "lifecycle.state")?,
            effective_at: required(effective_at, "lifecycle.effective_at")?,
            reason_digest: reason_digest.unwrap_or(None),
        })
    }
}

#[derive(Debug)]
struct TrustKeyWire {
    provider: String,
    key_id: String,
    algorithm: String,
    public_key: String,
    active: Option<bool>,
}

impl TryFrom<TrustKeyWire> for ReviewTrustKey {
    type Error = ReviewWireError;

    fn try_from(value: TrustKeyWire) -> Result<Self, Self::Error> {
        let encoded = value.public_key;
        let public_key =
            decode_base64url(&encoded).map_err(|_| ReviewWireError::InvalidPublicKeyEncoding {
                value: encoded.clone(),
            })?;
        let algorithm = AttestationAlgorithm::parse(value.algorithm)?;
        Ok(
            ReviewTrustKey::new(value.provider, value.key_id, algorithm, public_key)?
                .with_active(value.active.unwrap_or(true)),
        )
    }
}

impl<'de> Deserialize<'de> for TrustKeyWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(TrustKeyVisitor)
    }
}

struct TrustKeyVisitor;

impl<'de> Visitor<'de> for TrustKeyVisitor {
    type Value = TrustKeyWire;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("review trust key object")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut seen = BTreeSet::new();
        let mut provider = None;
        let mut key_id = None;
        let mut algorithm = None;
        let mut public_key = None;
        let mut active = None;
        while let Some(key) = map.next_key::<String>()? {
            if !seen.insert(key.clone()) {
                return Err(de::Error::custom(format!(
                    "duplicate field trust_key.{key}"
                )));
            }
            match key.as_str() {
                "provider" => provider = Some(map.next_value::<String>()?),
                "key_id" => key_id = Some(map.next_value::<String>()?),
                "algorithm" => algorithm = Some(map.next_value::<String>()?),
                "public_key" => public_key = Some(map.next_value::<String>()?),
                "active" => active = Some(map.next_value::<bool>()?),
                _ => return Err(de::Error::custom(format!("unknown field trust_key.{key}"))),
            }
        }
        Ok(TrustKeyWire {
            provider: required(provider, "trust_key.provider")?,
            key_id: required(key_id, "trust_key.key_id")?,
            algorithm: required(algorithm, "trust_key.algorithm")?,
            public_key: required(public_key, "trust_key.public_key")?,
            active,
        })
    }
}

fn required<T, E>(value: Option<T>, field: &'static str) -> Result<T, E>
where
    E: de::Error,
{
    value.ok_or_else(|| E::custom(format!("missing field {field}")))
}

fn decode_base64url(value: &str) -> Result<Vec<u8>, ReviewWireError> {
    if value.is_empty() || value.contains('=') || value.len() % 4 == 1 {
        return Err(ReviewWireError::InvalidSignatureEncoding {
            value: value.to_string(),
        });
    }
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity((bytes.len() * 3) / 4);
    let mut index = 0;
    while index < bytes.len() {
        let remaining = bytes.len() - index;
        let a = decode_base64url_char(bytes[index]).ok_or_else(|| {
            ReviewWireError::InvalidSignatureEncoding {
                value: value.to_string(),
            }
        })?;
        let b = decode_base64url_char(bytes[index + 1]).ok_or_else(|| {
            ReviewWireError::InvalidSignatureEncoding {
                value: value.to_string(),
            }
        })?;
        let c = if remaining >= 3 {
            Some(decode_base64url_char(bytes[index + 2]).ok_or_else(|| {
                ReviewWireError::InvalidSignatureEncoding {
                    value: value.to_string(),
                }
            })?)
        } else {
            None
        };
        let d = if remaining >= 4 {
            Some(decode_base64url_char(bytes[index + 3]).ok_or_else(|| {
                ReviewWireError::InvalidSignatureEncoding {
                    value: value.to_string(),
                }
            })?)
        } else {
            None
        };
        output.push((a << 2) | (b >> 4));
        if let Some(c) = c {
            if d.is_none() && c & 0x03 != 0 {
                return Err(ReviewWireError::InvalidSignatureEncoding {
                    value: value.to_string(),
                });
            }
            output.push((b << 4) | (c >> 2));
            if let Some(d) = d {
                output.push((c << 6) | d);
            }
        } else if b & 0x0f != 0 {
            return Err(ReviewWireError::InvalidSignatureEncoding {
                value: value.to_string(),
            });
        }
        index += remaining.min(4);
    }
    Ok(output)
}

fn decode_base64url_char(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'-' => Some(62),
        b'_' => Some(63),
        _ => None,
    }
}

fn encode_base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = String::with_capacity((bytes.len() * 4).div_ceil(3));
    for chunk in bytes.chunks(3) {
        let first = chunk[0] as u32;
        let second = chunk.get(1).copied().unwrap_or(0) as u32;
        let third = chunk.get(2).copied().unwrap_or(0) as u32;
        output.push(ALPHABET[((first >> 2) & 0x3f) as usize] as char);
        output.push(ALPHABET[(((first << 4) | (second >> 4)) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            output.push(ALPHABET[(((second << 2) | (third >> 6)) & 0x3f) as usize] as char);
        }
        if chunk.len() > 2 {
            output.push(ALPHABET[(third & 0x3f) as usize] as char);
        }
    }
    output
}
