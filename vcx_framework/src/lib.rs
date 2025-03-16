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
        handlers::out_of_band::{receiver::OutOfBandReceiver, sender::OutOfBandSender},
        messages::{
            decorators::transport::{ReturnRoute, Transport},
            msg_fields::protocols::out_of_band::invitation::OobService,
            msg_types::{
                protocols::did_exchange::{DidExchangeType, DidExchangeTypeV1},
                Protocol,
            },
        },
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
        messaging::{MessageReceiver, MessageSender, MessagingError},
        repositories::{
            connection_repository::{
                ConnectionRecordData, ConnectionRecordTagKeys, ConnectionRepository,
                ConnectionRepositoryError, ConnectionRole,
            },
            did_repository::{DidRecordData, DidRecordTagKeys, DidRepository},
            invitation_repository::{
                InvitationRecordData, InvitationRecordTagKeys, InvitationRepository,
                InvitationRepositoryError,
            },
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
        #[error("Connection Record Storage Error")]
        ConnectionStorageError(#[source] ConnectionRepositoryError),
        #[error("Invitation Record Storage Error")]
        InvitationStorageError(#[source] InvitationRepositoryError),
        #[error("Error with creating Out Of Band Invitation")]
        OutOfBandCreation(#[source] AriesVcxError),
    }

    pub struct ConnectionService<W: BaseWallet> {
        did_resolver_registry: Arc<did_resolver_registry::ResolverRegistry>,
        connection_repository: Arc<ConnectionRepository>,
        invitation_repository: Arc<InvitationRepository>,
        did_repository: Arc<DidRepository>,
        message_sender: Arc<MessageSender<W>>,
        message_receiver: Arc<MessageReceiver<W>>,
        wallet: Arc<W>,
        agent_endpoint: Url,
        agent_label: String,
    }

    impl<W: BaseWallet> ConnectionService<W> {
        pub fn new(
            did_resolver_registry: Arc<did_resolver_registry::ResolverRegistry>,
            connection_repository: Arc<ConnectionRepository>,
            invitation_repository: Arc<InvitationRepository>,
            did_repository: Arc<DidRepository>,
            message_sender: Arc<MessageSender<W>>,
            message_receiver: Arc<MessageReceiver<W>>,
            wallet: Arc<W>,
            agent_endpoint: Url,
            agent_label: String,
        ) -> Self {
            Self {
                did_resolver_registry,
                connection_repository,
                invitation_repository,
                did_repository,
                message_sender,
                message_receiver,
                wallet,
                agent_endpoint,
                agent_label,
            }
        }

        // Out of current scope: single-use invitations and connectionless-style invitations (requests included in invitation)
        pub async fn create_invitation(&self) -> Result<OutOfBandSender, ConnectionServiceError> {
            info!("Creating Out Of Band Invitation");
            // TODO - invitation should be able to be mediated (routing keys should be provided or generated)
            // TODO - create_peer_did_4() should []'pmove into peer did 4 implementation
            let (peer_did, _our_verkey) =
                create_peer_did_4(self.wallet.as_ref(), self.agent_endpoint.clone(), vec![])
                    .await
                    .map_err(ConnectionServiceError::PeerDIDError)?;

            let service = OobService::Did(peer_did.to_string());

            let oob_sender = OutOfBandSender::create()
                .append_service(&service)
                .append_handshake_protocol(Protocol::DidExchangeType(DidExchangeType::V1(
                    DidExchangeTypeV1::new_v1_1(),
                )))
                .map_err(ConnectionServiceError::OutOfBandCreation)?;

            info!(
                "Created Out of Band Invitation {}",
                oob_sender.invitation_to_json_string()
            );

            let id = oob_sender.get_id();
            let mut record_keys = HashMap::new();
            record_keys.insert(InvitationRecordTagKeys::SelfCreated, true.to_string());
            let record = Record::new(
                id.clone(),
                InvitationRecordData {
                    invite: oob_sender.clone(),
                    self_created: true,
                },
                Some(record_keys),
            );

            self.invitation_repository
                .add_or_update_record(record)
                .map_err(ConnectionServiceError::InvitationStorageError)?;
            // TODO -- Emit event

            Ok(oob_sender)
        }

        pub async fn connect(
            &self,
            invitation: OutOfBandReceiver,
            mediated: bool,
            specific_mediator_id: Option<Uuid>,
        ) -> Result<Uuid, ConnectionServiceError> {
            info!(
                "Requesting Connection via DID Exchange with invitation {}",
                invitation
            );

            // TODO - peer DID we create here should be able to be mediated (routing keys should be provided or generated)
            // TODO - create_peer_did_4() function should move into peer did 4 implementation
            let (peer_did, _our_verkey) =
                create_peer_did_4(self.wallet.as_ref(), self.agent_endpoint.clone(), vec![])
                    .await
                    .map_err(ConnectionServiceError::PeerDIDError)?;

            // Get Inviter DID from invitation
            let inviter_did = invitation_get_first_did_service(&invitation.oob)
                .map_err(ConnectionServiceError::NoDIDServiceFound)?;

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

            self.message_sender
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
            record_keys.insert(
                ConnectionRecordTagKeys::InvitationDid,
                inviter_did.to_string(),
            );
            let record = Record::new(
                connection_id.to_string(),
                ConnectionRecordData {
                    role: ConnectionRole::Requester,
                    our_did: peer_did,
                    their_did: inviter_did.clone(),
                    invitation_did: inviter_did,
                },
                Some(record_keys),
            );

            self.connection_repository
                .add_or_update_record(record)
                .map_err(ConnectionServiceError::ConnectionStorageError)?;

            Ok(connection_id)
        }

        pub async fn handle_connection_response(&self) -> Result<(), ConnectionServiceError> {
            // TODO - Process message
            Ok(())
        }
    }
}

pub mod messaging;

pub mod repositories;
pub mod storage;
pub mod transport;

#[cfg(test)]
fn test_init() {
    env_logger::builder().is_test(true).try_init().ok();
}
