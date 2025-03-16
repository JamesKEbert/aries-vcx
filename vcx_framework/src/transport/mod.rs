use std::{
    collections::HashMap,
    error,
    str::FromStr,
    sync::{Arc, Weak},
};

use aries_vcx::{
    aries_vcx_wallet::wallet::{askar::packing_types::Jwe, base_wallet::BaseWallet},
    messages::decorators::thread::Thread,
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

/// A flag for a transport's return status -- whether to hold the connection open for all messages, for messages pertaining to a specific threadId, or to close the session (if appropriate).
pub enum ReturnStatus {
    Close,
    All,
    ThreadId(Thread),
}

#[async_trait(?Send)]
pub trait InboundMessageReceiver {
    async fn receive_message(&self, encrypted_message: Jwe) -> ReturnStatus;
}

pub struct TransportManager {
    transports: HashMap<TransportScheme, Box<dyn Transport>>,
    message_receiver: Arc<dyn InboundMessageReceiver>,
}

impl TransportManager {
    pub fn new(message_receiver: Arc<dyn InboundMessageReceiver>) -> Self {
        Self {
            transports: HashMap::new(),
            message_receiver,
        }
    }

    pub fn receive_message(&self, message: EncryptionEnvelope) -> ReturnStatus {
        //TODO
        ReturnStatus::Close
    }

    pub fn register_transport(&mut self, transport: Box<dyn Transport>) -> () {
        self.transports.insert(transport.get_scheme(), transport);
    }

    pub fn get_supported_schemes(&self) -> Vec<&TransportScheme> {
        self.transports.keys().collect()
    }
    pub async fn send_message(
        &self,
        message: EncryptionEnvelope,
        endpoint: Url,
        returned_messages_allowed: bool,
    ) -> Result<(), TransportError> {
        let scheme = TransportScheme::from_str(endpoint.scheme())?;
        let transport_option = self.transports.get(&scheme);

        match transport_option {
            Some(transport) => {
                transport
                    .send_message(message, endpoint, returned_messages_allowed)
                    .await?;
                Ok(())
            }
            None => Err(TransportError::NoRegisteredTransportForScheme(scheme)),
        }
    }
}

#[async_trait(?Send)]
pub trait Transport {
    fn get_scheme(&self) -> TransportScheme;
    async fn send_message(
        &self,
        message: EncryptionEnvelope,
        endpoint: Url,
        returned_messages_allowed: bool,
    ) -> Result<(), TransportError>;
}

pub trait InboundTransport {
    // fn new that takes inbound_message() method
}

pub struct HttpTransport {
    transport_manager: Weak<TransportManager>,
}

impl HttpTransport {
    pub fn new(transport_manager: Weak<TransportManager>) -> Self {
        Self { transport_manager }
    }
}

#[async_trait(?Send)]
impl Transport for HttpTransport {
    fn get_scheme(&self) -> TransportScheme {
        TransportScheme::HTTP
    }

    async fn send_message(
        &self,
        message: EncryptionEnvelope,
        endpoint: Url,
        returned_messages_allowed: bool,
    ) -> Result<(), TransportError> {
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
        Ok(())
        // Ok(res.json::<Jwe>().await.ok())
    }
}
