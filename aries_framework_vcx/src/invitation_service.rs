use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc,
};

use aries_vcx::{
    aries_vcx_wallet::wallet::askar::AskarWallet,
    handlers::out_of_band::{receiver::OutOfBandReceiver, sender::OutOfBandSender},
    messages::{
        msg_fields::protocols::out_of_band::invitation::{Invitation, OobService},
        msg_types::{
            protocols::did_exchange::{DidExchangeType, DidExchangeTypeV1},
            Protocol,
        },
    },
    protocols::did_exchange::state_machine::helpers::create_peer_did_4,
};

use crate::{
    error::VCXFrameworkResult,
    framework::{EventEmitter, FrameworkConfig},
};

pub struct InvitationService {
    framework_config: FrameworkConfig,
    event_senders: Vec<Sender<InvitationEvent>>,
    wallet: Arc<AskarWallet>,
}

#[derive(Debug, Clone)]
pub struct InvitationEvent {
    pub state: String,
}

impl EventEmitter for InvitationService {
    type Event = InvitationEvent;
    fn emit_event(&mut self, event: InvitationEvent) {
        info!("Emitting InvitationEvent: {:?}", &event);
        self.event_senders
            .retain(|tx| match tx.send(event.clone()) {
                Ok(_) => true,
                Err(_) => {
                    debug!("Removing deallocated event listener from event listeners list");
                    false
                }
            })
    }

    fn register_event_receiver(&mut self) -> Receiver<Self::Event> {
        let (tx, rx): (Sender<InvitationEvent>, Receiver<InvitationEvent>) = mpsc::channel();

        self.event_senders.push(tx);
        rx
    }
}

impl InvitationService {
    pub fn new(framework_config: FrameworkConfig, wallet: Arc<AskarWallet>) -> Self {
        Self {
            framework_config,
            event_senders: vec![],
            wallet,
        }
    }

    /// Creates an Out of Band Invitation
    pub async fn create_invitation(&mut self) -> VCXFrameworkResult<OutOfBandSender> {
        debug!("Creating Out Of Band Invitation");
        // TODO - invitation should be able to be mediated (routing keys should be provided or generated)
        // TODO - create_peer_did_4() should move into peer did 4 implementation
        let (peer_did, _our_verkey) = create_peer_did_4(
            self.wallet.as_ref(),
            self.framework_config.agent_endpoint.clone(),
            vec![],
        )
        .await?;

        let service = OobService::Did(peer_did.to_string());

        let oob_sender = OutOfBandSender::create()
            .append_service(&service)
            .append_handshake_protocol(Protocol::DidExchangeType(DidExchangeType::V1(
                DidExchangeTypeV1::new_v1_1(),
            )))?;

        info!(
            "Created Out of Band Invitation {}",
            oob_sender.invitation_to_json_string()
        );

        // TODO - persist
        self.emit_event(InvitationEvent {
            state: "created".to_owned(),
        });
        Ok(oob_sender)
    }

    // pub async fn receive_invitation(
    //     &mut self,
    //     invitation: OutOfBandReceiver,
    // ) -> Result<OutOfBandReceiver, Box<dyn Error>> {
    //     debug!("Receiving Invitation");
    // }

    pub async fn get_invitation(&self) {}
}
