use std::{collections::HashMap, str::FromStr};

use aries_vcx::{
    aries_vcx_wallet::wallet::askar::packing_types::Jwe,
    utils::encryption_envelope::EncryptionEnvelope,
};
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum TransportError {
    #[error("invalid transport scheme `{0}`")]
    InvalidTransportScheme(String),
    #[error("no transport registered for scheme `{}`", 0.to_string())]
    NoRegisteredTransportForScheme(TransportScheme),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum TransportScheme {
    HTTP,
    WS,
}

impl FromStr for TransportScheme {
    type Err = TransportError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "http" | "https" => Ok(TransportScheme::HTTP),
            "ws" | "wss" => Ok(TransportScheme::WS),
            _ => Err(TransportError::InvalidTransportScheme(String::from(s))),
        }
    }
}

pub const PREFERRED_TRANSPORT_SCHEME_ORDER: [TransportScheme; 2] =
    [TransportScheme::WS, TransportScheme::HTTP];

pub struct TransportRegistry {
    transports: HashMap<TransportScheme, Box<dyn Transport>>,
}

impl TransportRegistry {
    pub fn new() -> Self {
        Self {
            transports: HashMap::new(),
        }
    }
    pub fn register_transport(mut self, transport: impl Transport + 'static) -> Self {
        self.transports
            .insert(transport.get_scheme(), Box::new(transport));
        self
    }

    pub fn get_supported_schemes(&self) -> Vec<&TransportScheme> {
        self.transports.keys().collect()
    }
    pub fn send_message(
        &self,
        message: EncryptionEnvelope,
        endpoint: &Url,
    ) -> Result<Option<Jwe>, TransportError> {
        let scheme = TransportScheme::from_str(endpoint.scheme())?;
        let transport_option = self.transports.get(&scheme);

        match transport_option {
            Some(transport) => Ok(transport.send_message(message, endpoint)),
            None => Err(TransportError::NoRegisteredTransportForScheme(scheme)),
        }
    }
}

pub trait Transport {
    fn get_scheme(&self) -> TransportScheme;
    fn send_message(&self, message: EncryptionEnvelope, endpoint: &Url) -> Option<Jwe>;
}

pub trait InboundTransport {
    // fn new that takes inbound_message() method
}

struct HttpTransport {}

impl HttpTransport {
    pub fn new() -> Self {
        Self {}
    }
}

impl Transport for HttpTransport {
    fn get_scheme(&self) -> TransportScheme {
        TransportScheme::HTTP
    }

    fn send_message(&self, message: EncryptionEnvelope, endpoint: &Url) -> Option<Jwe> {
        debug!(
            "Sending message via HTTP Transport to endpoint `{}`",
            endpoint
        );

        debug!("Sent message via HTTP Transport to endpoint `{}`", endpoint);
        None
    }
}
