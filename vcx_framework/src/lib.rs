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
        // connection_repository: Arc<
        //     ConnectionRepository<
        //         VCXFrameworkStorage<ConnectionRecordData, ConnectionRecordTagKeys>,
        //     >,
        // >,
        // did_repository:
        //     Arc<DidRepository<impl VCXFrameworkStorage<DidRecordData, DidRecordTagKeys>>>,
        transport_registry: Arc<TransportRegistry>,
        wallet: Arc<W>,
    }

    impl<W: BaseWallet> MessagingService<W> {
        pub fn new(
            did_resolver_registry: Arc<did_resolver_registry::ResolverRegistry>,
            transport_registry: Arc<TransportRegistry>,
            wallet: Arc<W>,
        ) -> Self {
            Self {
                did_resolver_registry,
                transport_registry,
                wallet,
            }
        }
        pub async fn send_message(
            &self,
            message: &AriesMessage,
            connection_id: &Uuid,
            _preferred_transports: Option<&[TransportScheme]>,
            connection_repository: ConnectionRepository<
                impl VCXFrameworkStorage<ConnectionRecordData, ConnectionRecordTagKeys>,
            >,
            did_repository: DidRepository<
                impl VCXFrameworkStorage<DidRecordData, DidRecordTagKeys>,
            >,
        ) -> Result<(), MessagingError> {
            info!(
                "Sending Aries Message to connection `{}`:
            {:?}",
                connection_id, message
            );

            let connection_record: Record<ConnectionRecordData, ConnectionRecordTagKeys> =
                connection_repository
                    .get_record(connection_id)
                    .map_err(|_| MessagingError::ConnectionRecordNotFound(connection_id.clone()))?
                    .ok_or(MessagingError::ConnectionRecordNotFound(
                        connection_id.clone(),
                    ))?;

            // TODO Save DIDs in DID Repository (important for finding relevant connection on inbound message)

            self.send_message_by_did(
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
        async fn send_message_by_did(
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
            let mut connection_repository = ConnectionRepository::new(in_memory_storage);

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
            let mut did_repository = DidRepository::new(in_memory_storage_dids);

            let messaging_service = MessagingService::new(
                Arc::new(did_resolver_registry),
                Arc::new(transport_registry),
                Arc::new(wallet),
            );
            messaging_service
                .send_message(
                    &message,
                    &connection_id,
                    Some(&[TransportScheme::HTTP, TransportScheme::WS]),
                    connection_repository,
                    did_repository,
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
