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
    use std::{
        error::Error,
        fmt::{Display, Formatter},
        io::{self},
    };

    use thiserror::Error;

    use aries_vcx::{
        aries_vcx_wallet::wallet::base_wallet::{did_wallet::DidWallet, BaseWallet},
        did_doc::schema::{service::typed::ServiceType, utils::error::DidDocumentLookupError},
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
    use uuid::Uuid;
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
    }

    // #[derive(Debug)]
    // pub enum MessagingError {
    //     SendingMessage,
    //     ResolvingDid(GenericError),
    //     DidPeer(DidPeerError),
    // }

    // impl Display for MessagingError {
    //     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    //         match self {
    //             MessagingError::SendingMessage => write!(f, "Failed to send message"),
    //             MessagingError::ResolvingDid(_err) => write!(f, "Error Resolving DID"),
    //             MessagingError::DidPeer(_err) => write!(f, "Error with Peer DID"),
    //         }
    //     }
    // }

    // impl error::Error for MessagingError {
    //     fn source(&self) -> Option<&(dyn error::Error + 'static)> {
    //         match self {
    //             MessagingError::ResolvingDid(err) => Some(err.as_ref()),
    //             MessagingError::DidPeer(err) => Some(err),
    //             MessagingError::SendingMessage => None,
    //         }
    //     }
    // }

    #[derive(Debug, Clone, Eq, PartialEq, Hash)]
    pub enum TransportProtocol {
        HTTP,
        WS,
    }

    pub const PREFERRED_PROTOCOL_ORDER: [TransportProtocol; 2] =
        [TransportProtocol::WS, TransportProtocol::HTTP];

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
        preferred_transports: Option<Vec<TransportProtocol>>,
        wallet: &impl BaseWallet,
        did_resolver_registry: did_resolver_registry::ResolverRegistry,
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

        // Event emitting
        // self.emit_event(MessagingEvents::OutboundMessage(OutboundMessage {
        //     message: message.clone(),
        //     encrypted_message: encrypted_message.clone(),
        //     sender_did: sender_did.clone(),
        //     receiver_did: receiver_did.clone(),
        // }));

        // Allow override of default preferred transport protocol order (as protocols may dictate or prefer specific protocols)
        let protocols_to_try = preferred_transports.unwrap_or(PREFERRED_PROTOCOL_ORDER.to_vec());
        // Try all
        for protocol in protocols_to_try {
            let transport = self.transport_registry.get_transport(protocol.to_owned());
            match transport {
                Some(transport) => {
                    debug!(
                        "Sending message via transport with protocol '{:?}'",
                        protocol
                    );
                    let possible_returned_message = transport
                        .send_message(
                            receiver_service.service_endpoint().to_owned(),
                            encrypted_message,
                        )
                        .await?;
                    if possible_returned_message.is_some() {
                        debug!("Response contained returned DIDComm Message, sending for inbound processing");
                        self.receive_message(
                            possible_returned_message.expect("To be returned DIDComm message"),
                        )
                        .await?;
                    }
                    break;
                }
                None => {
                    trace!("Unable to get transport with protocol '{:?}'", protocol);
                    continue;
                }
            }
        }

        // TODO: Handle returned messages (via return-route decorator/extension)

        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use crate::test_init;

        use super::*;

        #[test]
        fn test_encrypt_message() {
            test_init();
            debug!("{}", test_error().err().unwrap());
        }
    }
}

pub mod repositories;
pub mod storage;

#[cfg(test)]
fn test_init() {
    env_logger::builder().is_test(true).try_init().ok();
}
