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

pub mod connection_service {
    use std::{collections::HashMap, sync::Arc};

    use aries_vcx::{
        aries_vcx_wallet::wallet::base_wallet::BaseWallet,
        errors::error::AriesVcxError,
        handlers::out_of_band::receiver::OutOfBandReceiver,
        messages::decorators::transport::{ReturnRoute, Transport},
        protocols::did_exchange::state_machine::{
            generic::GenericDidExchange,
            helpers::create_peer_did_4,
            requester::helpers::{
                invitation_get_acceptable_did_exchange_version, invitation_get_first_did_service,
            },
        },
    };
    use thiserror::Error;
    use url::Url;
    use uuid::Uuid;

    use crate::{
        messaging_service::{MessagingError, MessagingService},
        repositories::{
            connection_repository::{
                ConnectionRecordData, ConnectionRecordTagKeys, ConnectionRepository,
            },
            did_repository::{DidRecordData, DidRecordTagKeys, DidRepository},
        },
        storage::{base::VCXFrameworkStorage, record::Record},
        transport::TransportScheme,
    };

    #[derive(Error, Debug)]
    pub enum ConnectionServiceError {
        #[error("Unsupporrted Did Exchange Version in Invitation, unable to create connection")]
        InvalidDidExchangeVersion(#[source] AriesVcxError),
        #[error("Error with peer DID")]
        PeerDIDError(#[source] AriesVcxError),
        #[error("Error finding DID service in Invitation")]
        NoDIDServiceFound(#[source] AriesVcxError),
        #[error("Unable to create connection request")]
        ErrorCreateConnectionRequest(#[source] AriesVcxError),
        #[error("Error Sending Message")]
        ErrorSendingMessage(#[source] MessagingError),
    }

    pub struct ConnectionService<W: BaseWallet> {
        did_resolver_registry: Arc<did_resolver_registry::ResolverRegistry>,
        connection_repository: Arc<ConnectionRepository>,
        did_repository: Arc<DidRepository>,
        messaging_service: Arc<MessagingService<W>>,
        wallet: Arc<W>,
        agent_endpoint: Url,
        agent_label: String,
    }

    impl<W: BaseWallet> ConnectionService<W> {
        pub fn new(
            did_resolver_registry: Arc<did_resolver_registry::ResolverRegistry>,
            connection_repository: Arc<ConnectionRepository>,
            did_repository: Arc<DidRepository>,
            messaging_service: Arc<MessagingService<W>>,
            wallet: Arc<W>,
            agent_endpoint: Url,
            agent_label: String,
        ) -> Self {
            Self {
                did_resolver_registry,
                connection_repository,
                did_repository,
                messaging_service,
                wallet,
                agent_endpoint,
                agent_label,
            }
        }

        pub async fn connect(
            &self,
            invitation: OutOfBandReceiver,
            mediated: bool,
            specific_mediator_id: Option<Uuid>,
        ) -> Result<(), ConnectionServiceError> {
            debug!(
                "Requesting Connection via DID Exchange with invitation {}",
                invitation
            );

            // TODO - peer did we create here should be able to be mediated (routing keys should be provided or generated)
            // TODO - create_peer_did_4() function should move into peer did 4 implementation
            let (peer_did, _our_verkey) =
                create_peer_did_4(self.wallet.as_ref(), self.agent_endpoint.clone(), vec![])
                    .await
                    .map_err(ConnectionServiceError::PeerDIDError)?;

            // Get Inviter DID from invitation
            let inviter_did = invitation_get_first_did_service(&invitation.oob)
                .map_err(ConnectionServiceError::NoDIDServiceFound)?;

            // Get DID Exchange version to use based off of invitation handshake protocols
            let version = invitation_get_acceptable_did_exchange_version(&invitation.oob)
                .map_err(ConnectionServiceError::InvalidDidExchangeVersion)?;

            // If not mediated, we will specify the transport decorator with return route all to allow for the DID Exchange response message to be returned immediately. Most useful in mobile contexts for establishing connections with mediators
            let transport_decorator =
                (!mediated).then_some(Transport::builder().return_route(ReturnRoute::All).build());

            // TODO - Fix DID Exchange Goal Code definition - Should not be "To establish a connection" - rather should be a goal code or not specified (IIRC)
            let (state_machine, request) = GenericDidExchange::construct_request(
                &self.did_resolver_registry,
                Some(invitation.oob.id.clone()),
                &inviter_did,
                &peer_did,
                self.agent_label.to_owned(),
                version,
                transport_decorator,
            )
            .await
            .map_err(ConnectionServiceError::ErrorCreateConnectionRequest)?;

            trace!("Created DID Exchange State Machine and request message, going to send message");

            let connection_id = Uuid::new_v4();

            self.messaging_service
                .send_message(
                    &request.into(),
                    connection_id,
                    Some(&[TransportScheme::HTTP, TransportScheme::WS]),
                )
                .await
                .map_err(ConnectionServiceError::ErrorSendingMessage)?;

            let mut record_keys = HashMap::new();
            record_keys.insert(ConnectionRecordTagKeys::OurDid, peer_did.to_string());
            record_keys.insert(ConnectionRecordTagKeys::TheirDid, inviter_did.to_string());
            let record = Record::new(
                connection_id.to_string(),
                ConnectionRecordData {
                    our_did: peer_did,
                    their_did: inviter_did,
                },
                Some(record_keys),
            );

            self.connection_repository.add_or_update_record(record);

            //TODO - Emit Event

            Ok(())
        }
    }
}

pub mod messaging_service {
    use std::sync::Arc;

    use thiserror::Error;

    use aries_vcx::{
        aries_vcx_wallet::wallet::base_wallet::BaseWallet,
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

    use crate::{
        repositories::{
            connection_repository::{
                ConnectionRecordData, ConnectionRecordTagKeys, ConnectionRepository,
            },
            did_repository::{DidRecordData, DidRecordTagKeys, DidRepository},
        },
        storage::{base::VCXFrameworkStorage, record::Record},
        transport::{TransportError, TransportRegistry, TransportScheme},
    };

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
        #[error("transport error while sending message")]
        OutboundTransportError(#[source] TransportError),
        // #[error("invalid transport scheme `{0}`")]
        // InvalidTransportScheme(#[source] TransportError, String),
        // #[error("no registered transports for diddoc service endpoint scheme `{}`", 1.to_string())]
        // NoRegisteredTransportsForScheme(#[source] TransportError, TransportScheme),
        #[error("connection record not found for id `{0}`")]
        ConnectionRecordNotFound(Uuid),
    }

    pub struct MessagingService<W: BaseWallet> {
        did_resolver_registry: Arc<did_resolver_registry::ResolverRegistry>,
        connection_repository: Arc<ConnectionRepository>,
        did_repository: Arc<DidRepository>,
        transport_registry: Arc<TransportRegistry>,
        wallet: Arc<W>,
    }

    impl<W: BaseWallet> MessagingService<W> {
        pub fn new(
            did_resolver_registry: Arc<did_resolver_registry::ResolverRegistry>,
            transport_registry: Arc<TransportRegistry>,
            connection_repository: Arc<ConnectionRepository>,
            did_repository: Arc<DidRepository>,
            wallet: Arc<W>,
        ) -> Self {
            Self {
                did_resolver_registry,
                transport_registry,
                connection_repository,
                did_repository,
                wallet,
            }
        }
        pub async fn send_message(
            &self,
            message: &AriesMessage,
            connection_id: Uuid,
            _preferred_transports: Option<&[TransportScheme]>,
        ) -> Result<(), MessagingError> {
            info!(
                "Sending Aries Message to connection `{}`:
            {:?}",
                connection_id, message
            );

            let connection_record: Record<ConnectionRecordData, ConnectionRecordTagKeys> = self
                .connection_repository
                .get_record(&connection_id)
                .map_err(|_| MessagingError::ConnectionRecordNotFound(connection_id))?
                .ok_or(MessagingError::ConnectionRecordNotFound(connection_id))?;

            // TODO Save DIDs in DID Repository (important for finding relevant connection on inbound message)
            // Actually -- we should check for DID on connection_record -- it should be set at connection record creation

            self.send_message_to_did(
                message,
                connection_record.data.our_did,
                connection_record.data.their_did,
                _preferred_transports,
            )
            .await?;

            info!("Sent Aries Message to connection `{}`", connection_id);
            Ok(())
        }

        // Should this be restricted to sender_did being a peer did? (probably not)
        async fn send_message_to_did(
            &self,
            message: &AriesMessage,
            sender_did: PeerDid<Numalgo4>,
            receiver_did: Did,
            _preferred_transports: Option<&[TransportScheme]>,
        ) -> Result<(), MessagingError> {
            debug!(
                "Sending Aries Message {}
                  to Receiver DID {}
                  from Sender DID {}",
                &message, &receiver_did, &sender_did
            );

            let receiver_did_document = self
                .did_resolver_registry
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
                .map_err(|err| {
                    MessagingError::InvalidDidDocService(err, receiver_did.to_string())
                })?;

            let encrypted_message = EncryptionEnvelope::create(
                self.wallet.as_ref(),
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

            let returned_message = self
                .transport_registry
                .send_message(
                    encrypted_message,
                    receiver_service.service_endpoint().to_owned(),
                )
                .await
                .map_err(MessagingError::OutboundTransportError)?;

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
    }

    #[cfg(test)]
    mod tests {
        use std::str::FromStr;

        use aries_vcx::{
            aries_vcx_wallet::wallet::{
                askar::{
                    askar_wallet_config::AskarWalletConfig,
                    key_method::{ArgonLevel, AskarKdfMethod, KeyMethod},
                },
                base_wallet::ManageWallet,
            },
            did_peer::resolver::PeerDidResolver,
            messages::msg_fields::protocols::trust_ping::ping::{
                Ping, PingContent, PingDecorators,
            },
            protocols::did_exchange::state_machine::helpers::create_peer_did_4,
        };
        use did_resolver_registry::ResolverRegistry;
        use url::Url;

        use crate::{
            storage::in_memory_storage::InMemoryStorage, test_init, transport::HttpTransport,
        };

        use super::*;

        pub const IN_MEMORY_DB_URL: &str = "sqlite://:memory:";
        pub const DEFAULT_WALLET_PROFILE: &str = "aries_framework_vcx_default";
        pub const DEFAULT_ASKAR_KEY_METHOD: KeyMethod = KeyMethod::DeriveKey {
            inner: AskarKdfMethod::Argon2i {
                inner: (ArgonLevel::Interactive),
            },
        };

        #[tokio::test]
        async fn test_send_message() {
            test_init();

            let connection_id = Uuid::new_v4();
            let message_content = PingContent::builder().response_requested(true).build();
            let message_decorators = PingDecorators::builder().build();
            let message = AriesMessage::TrustPing(
                Ping::builder()
                    .id(connection_id.to_string())
                    .decorators(message_decorators)
                    .content(message_content)
                    .build(),
            );

            let wallet_config = AskarWalletConfig {
                db_url: IN_MEMORY_DB_URL.to_string(),
                key_method: DEFAULT_ASKAR_KEY_METHOD,
                pass_key: "sample_pass_key".to_string(),
                profile: DEFAULT_WALLET_PROFILE.to_string(),
            };
            let wallet = wallet_config.create_wallet().await.unwrap();

            let did_peer_resolver = PeerDidResolver::new();
            let did_resolver_registry =
                ResolverRegistry::new().register_resolver("peer".into(), did_peer_resolver);

            let transport_registry =
                TransportRegistry::new().register_transport(HttpTransport::new());

            let in_memory_storage =
                InMemoryStorage::<ConnectionRecordData, ConnectionRecordTagKeys>::new();
            let mut connection_repository = ConnectionRepository::new(Box::new(in_memory_storage));

            let (our_did, _our_verkey) = create_peer_did_4(
                &wallet,
                Url::from_str("http://example.com").unwrap(),
                vec![],
            )
            .await
            .unwrap();
            let (their_did, _their_verkey) = create_peer_did_4(
                &wallet,
                Url::from_str("http://example.com").unwrap(),
                vec![],
            )
            .await
            .unwrap();

            connection_repository
                .add_or_update_record(Record::new(
                    connection_id.to_string(),
                    ConnectionRecordData {
                        our_did,
                        their_did: their_did.did().clone(),
                    },
                    None,
                ))
                .unwrap();

            let in_memory_storage_dids = InMemoryStorage::<DidRecordData, DidRecordTagKeys>::new();
            let mut did_repository = DidRepository::new(Box::new(in_memory_storage_dids));

            let messaging_service = MessagingService::new(
                Arc::new(did_resolver_registry),
                Arc::new(transport_registry),
                Arc::new(connection_repository),
                Arc::new(did_repository),
                Arc::new(wallet),
            );
            messaging_service
                .send_message(
                    &message,
                    connection_id,
                    Some(&[TransportScheme::HTTP, TransportScheme::WS]),
                )
                .await
                .unwrap()
        }
    }
}

pub mod repositories;
pub mod storage;
pub mod transport;

#[cfg(test)]
fn test_init() {
    env_logger::builder().is_test(true).try_init().ok();
}
