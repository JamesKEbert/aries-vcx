#[macro_use]
extern crate log;

pub mod error {
    use std::fmt::{Display, Formatter};

    use crate::storage::error::StorageError;

    #[derive(Debug)]
    pub enum VCXFrameworkError {
        Storage(StorageError),
    }

    impl Display for VCXFrameworkError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                VCXFrameworkError::Storage(storage_error) => StorageError::fmt(storage_error, f),
            }
        }
    }

    impl std::error::Error for VCXFrameworkError {}
}

pub mod messaging_module {
    use std::{clone, collections::HashMap, str::FromStr};

    use thiserror::Error;

    use aries_vcx::{
        aries_vcx_wallet::wallet::{askar::packing_types::Jwe, base_wallet::BaseWallet},
        did_doc::schema::{
            self, service::typed::ServiceType, utils::error::DidDocumentLookupError,
        },
        did_parser_nom::Did,
        did_peer::{
            error::DidPeerError,
            peer_did::{numalgos::numalgo4::Numalgo4, PeerDid},
        },
        errors::error::AriesVcxError,
        messages::AriesMessage,
        utils::encryption_envelope::EncryptionEnvelope,
    };
    use did_resolver_registry::GenericError;
    use url::Url;

    #[derive(Error, Debug)]
    pub enum MessagingError {
        #[error("error resolving DID `{1}`")]
        DidResolution(#[source] GenericError, String),
        #[error("error resolving peer DID `{1}`")]
        DidResolutionPeerDid(#[source] DidPeerError, String),
        #[error("unable to get service from DIDDoc for DID `{1}`")]
        InvalidDidDocService(#[source] DidDocumentLookupError, String),
        #[error("error encrypting message")]
        EncryptMessage(#[source] AriesVcxError),
        #[error("error decrypting message")]
        DecryptMessage(#[source] AriesVcxError),
        #[error("invalid transport scheme `{0}`")]
        InvalidTransportScheme(#[source] TransportError, String),
        #[error("no registered transports for diddoc service endpoint scheme `{}`", 1.to_string())]
        NoRegisteredTransportsForScheme(#[source] TransportError, TransportScheme),
    }

    // pub async fn send_message(
    //     message: AriesMessage,
    //     connectionId: Uuid,
    //     preferred_transports: Option<Vec<TransportProtocol>>,
    // ) -> Result<(), MessagingError> {
    // }

    // Should this be restricted to sender_did being a peer did? (probably not)
    async fn send_message_by_did(
        message: &AriesMessage,
        sender_did: PeerDid<Numalgo4>,
        receiver_did: Did,
        _preferred_transports: Option<&[TransportScheme]>,
        wallet: &impl BaseWallet,
        did_resolver_registry: did_resolver_registry::ResolverRegistry,
        transport_registry: TransportRegistry,
    ) -> Result<(), MessagingError> {
        debug!(
            "Sending Aries Message {}
              to Receiver DID {}
              from Sender DID {}",
            &message, &receiver_did, &sender_did
        );

        let receiver_did_document = did_resolver_registry
            .resolve(&receiver_did, &Default::default())
            .await
            .map_err(|err| MessagingError::DidResolution(err, receiver_did.to_string()))?
            .did_document;
        let sender_did_document = sender_did
            .resolve_did_doc()
            .map_err(|err| MessagingError::DidResolutionPeerDid(err, sender_did.to_string()))?;

        // TODO: need to provide a way of iterating through all available services, in order of transport preference, instead of just taking the first available service. This would also allow us additional services if one fails.
        // Allow override of default preferred transport scheme order (as protocols may dictate or prefer specific protocols)
        // let protocols_to_try = preferred_transports.unwrap_or(PREFERRED_PROTOCOL_ORDER.to_vec());

        let receiver_service = receiver_did_document
            .get_service_of_type(&ServiceType::DIDCommV1)
            .map_err(|err| MessagingError::InvalidDidDocService(err, receiver_did.to_string()))?;

        let encrypted_message = EncryptionEnvelope::create(
            wallet,
            message.to_string().as_bytes(),
            &sender_did_document,
            &receiver_did_document,
            receiver_service.id(),
        )
        .await
        .map_err(MessagingError::EncryptMessage)?;

        trace!(
            "EncryptedMessage to send: {}",
            String::from_utf8_lossy(&encrypted_message.0)
        );

        let returned_message = transport_registry
            .send_message(encrypted_message, receiver_service.service_endpoint())
            .map_err(|err| match err {
                TransportError::InvalidTransportScheme(ref scheme) => {
                    let new_scheme = String::from(scheme);
                    MessagingError::InvalidTransportScheme(err, new_scheme)
                }
                TransportError::NoRegisteredTransportForScheme(scheme) => {
                    MessagingError::NoRegisteredTransportsForScheme(err, scheme.to_owned())
                }
            })?;

        debug!("Sent message");

        // Handle inbound message if one was returned due to a return route transport decorator (DIDComm v1) or return route extension (DIDComm v2)
        if returned_message.is_some() {
            debug!("Handling received message returned via return route mechanism");
            // TODO: Check whether outbound message contained return route field, if not, we should log error upon receiving message and send problem report if possible
            // let return_route_enabled = false;

            // TODO
        }

        // Event emitting
        // TODO
        // self.emit_event(MessagingEvents::OutboundMessage(OutboundMessage {
        //     message: message.clone(),
        //     encrypted_message: encrypted_message.clone(),
        //     sender_did: sender_did.clone(),
        //     receiver_did: receiver_did.clone(),
        // }));

        Ok(())
    }

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

    struct TransportRegistry {
        transports: HashMap<TransportScheme, Box<dyn Transport>>,
    }

    impl TransportRegistry {
        fn new() -> Self {
            Self {
                transports: HashMap::new(),
            }
        }
        fn register_transport(mut self, transport: impl Transport + 'static) -> Self {
            self.transports
                .insert(transport.get_scheme(), Box::new(transport));
            self
        }

        fn get_supported_schemes(&self) -> Vec<&TransportScheme> {
            self.transports.keys().collect()
        }
        fn send_message(
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
        fn new() -> Self {
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

    #[cfg(test)]
    mod tests {
        use crate::test_init;

        use super::*;

        #[test]
        fn test_encrypt_message() {
            test_init();
        }
    }
}

pub mod repositories;
pub mod storage;

#[cfg(test)]
fn test_init() {
    env_logger::builder().is_test(true).try_init().ok();
}
