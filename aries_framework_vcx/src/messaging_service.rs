use core::str;
use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc,
};

use aries_vcx::{
    aries_vcx_wallet::wallet::askar::{packing_types::Jwe, AskarWallet},
    did_doc::schema::service::typed::ServiceType,
    did_parser_nom::Did,
    messages::AriesMessage,
    utils::encryption_envelope::EncryptionEnvelope,
};
use did_peer::peer_did::{numalgos::numalgo4::Numalgo4, PeerDid};
use did_resolver_registry::ResolverRegistry;

use crate::{
    error::VCXFrameworkResult,
    framework::{EventEmitter, FrameworkConfig},
    transports::{TransportProtocol, TransportRegistry, PREFERRED_PROTOCOL_ORDER},
};

pub struct MessagingService {
    framework_config: FrameworkConfig,
    wallet: Arc<AskarWallet>,
    did_resolver_registry: Arc<ResolverRegistry>,
    event_senders: Vec<Sender<MessagingEvents>>,
    transport_registry: TransportRegistry,
}

#[derive(Debug, Clone)]
pub enum MessagingEvents {
    InboundMessage(InboundMessage),
    OutboundMessage(OutboundMessage),
}

#[derive(Debug, Clone)]
pub struct InboundMessage {
    pub receiver_did: PeerDid<Numalgo4>,
    pub sender_did: Did,
    pub message: AriesMessage,
}

#[derive(Debug, Clone)]
pub struct OutboundMessage {
    pub sender_did: PeerDid<Numalgo4>,
    pub receiver_did: Did,
    pub message: AriesMessage,
    pub encrypted_message: EncryptionEnvelope,
}

impl EventEmitter for MessagingService {
    type Event = MessagingEvents;
    fn emit_event(&mut self, event: MessagingEvents) {
        self.event_senders
            .retain(|tx| match tx.send(event.clone()) {
                Ok(_) => true,
                Err(_) => {
                    debug!("Removing deallocated event listener from event listeners list");
                    false
                }
            })
    }

    /// Register event receivers to monitor inbound and outbound messages. Not intended to be used to handle inbound messages, use TODO for that purpose
    fn register_event_receiver(&mut self) -> Receiver<Self::Event> {
        let (tx, rx): (Sender<MessagingEvents>, Receiver<MessagingEvents>) = mpsc::channel();

        self.event_senders.push(tx);
        rx
    }
}

impl MessagingService {
    pub fn new(
        framework_config: FrameworkConfig,
        wallet: Arc<AskarWallet>,
        did_resolver_registry: Arc<ResolverRegistry>,
        transport_registry: TransportRegistry,
    ) -> Self {
        Self {
            framework_config,
            wallet,
            did_resolver_registry,
            event_senders: vec![],
            transport_registry,
        }
    }

    pub async fn send_message(
        &mut self,
        message: AriesMessage,
        sender_did: PeerDid<Numalgo4>,
        receiver_did: Did,
        preferred_transports: Option<Vec<TransportProtocol>>,
    ) -> VCXFrameworkResult<()> {
        info!(
            "Sending Aries Message {} 
              to Receiver DID {}
              from Sender DID {}",
            &message, &receiver_did, &sender_did
        );

        let receiver_did_document = self
            .did_resolver_registry
            .resolve(&receiver_did, &Default::default())
            .await?
            .did_document;
        let sender_did_document = sender_did.resolve_did_doc()?;

        let receiver_service =
            receiver_did_document.get_service_of_type(&ServiceType::DIDCommV1)?;

        let encrypted_message = EncryptionEnvelope::create(
            self.wallet.as_ref(),
            message.to_string().as_bytes(),
            &sender_did_document,
            &receiver_did_document,
            receiver_service.id(),
        )
        .await?;

        trace!(
            "EncryptedMessage to send: {}",
            str::from_utf8(&encrypted_message.0)?
        );

        self.emit_event(MessagingEvents::OutboundMessage(OutboundMessage {
            message: message.clone(),
            encrypted_message: encrypted_message.clone(),
            sender_did: sender_did.clone(),
            receiver_did: receiver_did.clone(),
        }));

        // Allow override of default preferred transport protocol order (as protocols may dictate or prefer specific protocols)
        let protocols_to_try = preferred_transports.unwrap_or(PREFERRED_PROTOCOL_ORDER.to_vec());
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
        Ok(())
    }

    async fn receive_message(&mut self, encrypted_message: Jwe) -> VCXFrameworkResult<()> {
        trace!("Received encrypted message: {:?}", encrypted_message);
        // Note that the function name here references anon_unpack,
        // however the implementation itself will perform either anon or auth unpacking based off of the indicated "alg" in the message.
        // May be worthwhile considering adjusting the underlining function API in the future to be more clear.
        let (message, sender_vk, recipient_vk) = EncryptionEnvelope::unpack(
            self.wallet.as_ref(),
            serde_json::json!(encrypted_message).to_string().as_bytes(),
            &None,
        )
        .await?;
        debug!(
            "Received inbound message from sender key: {:?}
              for recipient key: {:?}
              message: {}",
            sender_vk, recipient_vk, message
        );
        Ok(())
    }

    pub fn receive_inbound_message(&mut self, message: Jwe) {
        // TODO -- very big todo -- allow for a message to be delivered back as a response to an inbound message if the original message had a transport decorator with return route all. This likely will be done with a session management strategy

        // TODO - close inbound transport session if appropriate for the transport (WS is not) and if no transport decorator with return route all for inbound message
    }
}
