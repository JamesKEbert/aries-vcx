use std::{collections::HashMap, error, str::FromStr};

use aries_vcx::{
    aries_vcx_wallet::wallet::askar::packing_types::Jwe,
    utils::encryption_envelope::EncryptionEnvelope,
};
use async_trait::async_trait;
use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum TransportError {
    #[error("invalid transport scheme `{0}`")]
    InvalidTransportScheme(String),
    #[error("no transport registered for scheme `{}`", 0.to_string())]
    NoRegisteredTransportForScheme(TransportScheme),
    #[error("error sending message")]
    ErrorSendingMessage(Box<dyn error::Error>),
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
    pub async fn send_message(
        &self,
        message: EncryptionEnvelope,
        endpoint: Url,
    ) -> Result<Option<Jwe>, TransportError> {
        let scheme = TransportScheme::from_str(endpoint.scheme())?;
        let transport_option = self.transports.get(&scheme);

        match transport_option {
            Some(transport) => Ok(transport.send_message(message, endpoint).await?),
            None => Err(TransportError::NoRegisteredTransportForScheme(scheme)),
        }
    }
}

#[async_trait]
pub trait Transport {
    fn get_scheme(&self) -> TransportScheme;
    async fn send_message(
        &self,
        message: EncryptionEnvelope,
        endpoint: Url,
    ) -> Result<Option<Jwe>, TransportError>;
}

pub trait InboundTransport {
    // fn new that takes inbound_message() method
}

pub struct HttpTransport {}

impl HttpTransport {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Transport for HttpTransport {
    fn get_scheme(&self) -> TransportScheme {
        TransportScheme::HTTP
    }

    async fn send_message(
        &self,
        message: EncryptionEnvelope,
        endpoint: Url,
    ) -> Result<Option<Jwe>, TransportError> {
        debug!(
            "Sending DIDComm message via HTTP Transport to endpoint `{}`",
            endpoint
        );

        let client = reqwest::Client::new();
        let res = client
            .post(endpoint.clone())
            .body(message.0)
            .header(CONTENT_TYPE, "application/didcomm-envelope-enc")
            .header(USER_AGENT, "reqwest")
            .send()
            .await
            .map_err(|err| TransportError::ErrorSendingMessage(Box::new(err)))?;

        debug!("Received Response with Status `{}`", res.status());

        debug!("Sent message via HTTP Transport to endpoint `{}`", endpoint);
        Ok(res.json::<Jwe>().await.ok())
    }
}
