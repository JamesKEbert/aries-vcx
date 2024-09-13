use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc, Mutex,
};

use aries_vcx::{
    aries_vcx_wallet::wallet::askar::AskarWallet,
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
use did_resolver_registry::ResolverRegistry;
use uuid::Uuid;

use crate::{
    error::VCXFrameworkResult,
    framework::{EventEmitter, FrameworkConfig},
    invitation_service::InvitationService,
    messaging_service::MessagingService,
    transports::TransportProtocol,
};

#[derive(Clone)]
pub struct ConnectionServiceConfig {
    pub auto_complete_requests: bool,
    pub auto_respond_to_requests: bool,
    pub auto_handle_requests: bool,
}

impl Default for ConnectionServiceConfig {
    fn default() -> Self {
        Self {
            auto_complete_requests: true,
            auto_handle_requests: true,
            auto_respond_to_requests: true,
        }
    }
}

pub struct ConnectionService {
    framework_config: FrameworkConfig,
    event_senders: Vec<Sender<ConnectionEvent>>,
    wallet: Arc<AskarWallet>,
    did_resolver_registry: Arc<ResolverRegistry>,
    messaging_service: Arc<Mutex<MessagingService>>,
    invitation_service: Arc<Mutex<InvitationService>>,
}

impl ConnectionService {
    pub fn new(
        framework_config: FrameworkConfig,
        wallet: Arc<AskarWallet>,
        did_resolver_registry: Arc<ResolverRegistry>,
        messaging_service: Arc<Mutex<MessagingService>>,
        invitation_service: Arc<Mutex<InvitationService>>,
    ) -> Self {
        invitation_service
            .lock()
            .expect("unpoisoned mutex")
            .register_event_receiver();
        Self {
            framework_config,
            event_senders: vec![],
            wallet,
            messaging_service,
            did_resolver_registry,
            invitation_service,
        }
    }

    /// Helper function to request connection, automating everything until connection completed
    pub async fn connect(&mut self) {}

    /// Helper function to request connection and block until complete but with timeout, automating everything until connection completed
    pub async fn connect_and_await() {
        // TODO - add observer
    }

    /// Handles inbound connection requests in relation to a invitation the framework has created. It will automate the process until the connection is completed, barring any errors throughout the process.
    pub async fn handle_request() {}

    /// Handles inbound connection requests in relation to a invitation the framework has created. It will automate the process until the connection is completed, barring any errors throughout the process. This method will not return until completion, error, or the timeout has been reached. Use [`handle_request()`] instead for non blocking behavior.
    ///
    /// [`handle_request()`]: Self::handle_request()
    pub async fn handle_request_and_await(
        &mut self,
        invitation_id: &str,
    ) -> VCXFrameworkResult<()> {
        // testing I was doing here, ignore please
        // let invitation = self
        //     .invitation_service
        //     .lock()
        //     .expect("unpoisoned mutex")
        //     .create_invitation()
        //     .await?;
        // self.request_connection(invitation).await?;
        // TODO - add observer
        Ok(())
    }
}

// Provides internal framework functions for transitioning between protocol states
impl ConnectionService {
    // TODO - invitation should be accessing invitation service via an id for the OutOfBandReceiver, rather than requiring the consuming developer to generate the OutOfBandReceiver
    /// TODO - add description
    ///
    /// `specific_mediator_id` will override the usage of the configured default mediator if `mediated` is set to true.
    pub async fn request_connection(
        &mut self,
        invitation: OutOfBandReceiver,
        mediated: bool,
        specific_mediator_id: Option<Uuid>,
    ) -> VCXFrameworkResult<()> {
        debug!(
            "Requesting Connection via DID Exchange with invitation {}",
            invitation
        );

        // Create our peer DID using Peer DID Numalgo 4
        // TODO - peer did we create here should be able to be mediated (routing keys should be provided or generated)
        // TODO - create_peer_did_4() should move into peer did 4 implementation
        let (peer_did, _our_verkey) = create_peer_did_4(
            self.wallet.as_ref(),
            self.framework_config.agent_endpoint.clone(),
            vec![],
        )
        .await?;

        // Get Inviter/Responder DID from invitation
        let inviter_did = invitation_get_first_did_service(&invitation.oob)?;

        // Get DID Exchange version to use based off of invitation handshake protocols
        let version = invitation_get_acceptable_did_exchange_version(&invitation.oob)?;

        // If not mediated, we will specify the transport decorator with return route all to allow for the DID Exchange response message to be returned immediately. Most useful in mobile contexts for establishing connections with mediators
        let transport_decorator =
            (!mediated).then_some(Transport::builder().return_route(ReturnRoute::All).build());

        // TODO - Fix DID Exchange Goal Code definition - Should not be "To establish a connection" - rather should be a goal code or not specified (IIRC)
        // Create DID Exchange Request Message, generate did_exchange_requester for future
        let (state_machine, request) = GenericDidExchange::construct_request(
            &self.did_resolver_registry,
            Some(invitation.oob.id.clone()),
            &inviter_did,
            &peer_did,
            self.framework_config.agent_label.to_owned(),
            version,
            transport_decorator,
        )
        .await?;

        trace!("Created DID Exchange State Machine and request message, going to send message");

        // Send Request
        self.messaging_service
            .lock()
            .expect("unpoisoned mutex")
            .send_message(
                request.clone().into(),
                peer_did,
                inviter_did,
                Some(vec![TransportProtocol::HTTP, TransportProtocol::WS]),
            )
            .await?;

        // Store Updated State
        let record = ConnectionRecord {
            id: Uuid::parse_str(&request.inner().id)?,
            invitation_id: Uuid::parse_str(&invitation.oob.id)?,
            state_machine,
        };
        // TODO - Store Record

        // Emit new event indicating updated state
        self.emit_event(ConnectionEvent { record });

        Ok(())
    }

    fn process_response() {}

    fn send_complete() {}

    fn process_request() {}

    fn send_response() {}

    fn process_complete() {}
}

impl EventEmitter for ConnectionService {
    type Event = ConnectionEvent;

    fn emit_event(&mut self, event: ConnectionEvent) {
        info!("Emitting ConnectionEvent: {:?}", &event);
        self.event_senders
            .retain(|tx| match tx.send(event.clone()) {
                Ok(_) => true,
                Err(_) => {
                    debug!("Removing deallocated event listener from event listeners list");
                    false
                }
            })
    }

    fn register_event_receiver(&mut self) -> Receiver<ConnectionEvent> {
        let (tx, rx): (Sender<ConnectionEvent>, Receiver<ConnectionEvent>) = mpsc::channel();

        self.event_senders.push(tx);
        rx
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionRecord {
    id: Uuid,
    invitation_id: Uuid,
    state_machine: GenericDidExchange,
}

#[derive(Debug, Clone)]
pub struct ConnectionEvent {
    record: ConnectionRecord,
}
