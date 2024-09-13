use std::collections::HashMap;

use aries_vcx::{
    aries_vcx_wallet::wallet::askar::packing_types::Jwe,
    utils::encryption_envelope::EncryptionEnvelope,
};
use async_trait::async_trait;
use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use url::Url;

use crate::VCXFrameworkResult;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum TransportProtocol {
    HTTP,
    WS,
}

pub const PREFERRED_PROTOCOL_ORDER: [TransportProtocol; 2] =
    [TransportProtocol::WS, TransportProtocol::HTTP];

pub type GenericTransport = dyn Transport;

#[async_trait]
pub trait Transport {
    /// Sends an encrypted DIDComm V1 message to the specified endpoint. Returns an option of `EncryptionEnvelope` for cases when a message is directly returned due to the outbound message using a transport decorator with return_route all.
    async fn send_message(
        &self,
        endpoint: Url,
        message: EncryptionEnvelope,
    ) -> VCXFrameworkResult<Option<Jwe>>;
}
#[derive(Default)]
pub struct TransportRegistry {
    transports: HashMap<TransportProtocol, Box<GenericTransport>>,
}

impl TransportRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_transport<T>(
        mut self,
        transport_protocol: TransportProtocol,
        transport: T,
    ) -> Self
    where
        T: Transport + 'static,
    {
        self.transports
            .insert(transport_protocol, Box::new(transport));
        self
    }

    pub fn get_transport(
        &self,
        transport_protocol: TransportProtocol,
    ) -> Option<&Box<dyn Transport>> {
        self.transports.get(&transport_protocol)
    }
}

#[derive(Debug, Default)]
pub struct HTTPTransport {}

#[async_trait]
impl Transport for HTTPTransport {
    async fn send_message(
        &self,
        endpoint: Url,
        message: EncryptionEnvelope,
    ) -> VCXFrameworkResult<Option<Jwe>> {
        debug!(
            "Sending DIDComm Message via HTTP to URL Endpoint '{}'",
            endpoint
        );

        let client = reqwest::Client::new();
        let res = client
            .post(endpoint)
            .body(message.0)
            .header(CONTENT_TYPE, "application/didcomm-envelope-enc")
            .header(USER_AGENT, "reqwest")
            .send()
            .await?;

        debug!("Received Response with Status '{}'", res.status());

        // Check if response contains an inbound message (possible with the transport decorator w/return_route: all)
        Ok(res.json::<Jwe>().await.ok())
    }
}
