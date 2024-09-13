use std::sync::{mpsc::Receiver, Arc, Mutex};

use aries_vcx::aries_vcx_wallet::wallet::{
    askar::{
        askar_wallet_config::AskarWalletConfig,
        key_method::{ArgonLevel, AskarKdfMethod, KeyMethod},
        AskarWallet,
    },
    base_wallet::ManageWallet,
};
use did_peer::resolver::PeerDidResolver;
use did_resolver_registry::ResolverRegistry;
use url::Url;

use crate::{
    connection_service::{ConnectionService, ConnectionServiceConfig},
    error::VCXFrameworkResult,
    invitation_service::InvitationService,
    messaging_service::MessagingService,
    transports::{HTTPTransport, TransportProtocol, TransportRegistry},
};

pub const IN_MEMORY_DB_URL: &str = "sqlite://:memory:";
pub const DEFAULT_WALLET_PROFILE: &str = "aries_framework_vcx_default";
pub const DEFAULT_ASKAR_KEY_METHOD: KeyMethod = KeyMethod::DeriveKey {
    inner: AskarKdfMethod::Argon2i {
        inner: (ArgonLevel::Interactive),
    },
};

#[derive(Clone)]
pub struct FrameworkConfig {
    pub wallet_config: AskarWalletConfig,
    pub connection_service_config: ConnectionServiceConfig,
    pub agent_endpoint: Url,
    pub agent_label: String,
}

pub struct AriesFrameworkVCX {
    pub framework_config: FrameworkConfig,
    pub wallet: Arc<AskarWallet>,
    pub did_resolver_registry: Arc<ResolverRegistry>,
    pub messaging_service: Arc<Mutex<MessagingService>>,
    pub invitation_service: Arc<Mutex<InvitationService>>,

    /// A service for the management of any and all things related to connections, including the usage of invitations (Out Of Band Invitations), the DID Exchange protocol, and mediation protocols.
    ///
    /// Note: This is service is about generic DIDComm connections and is **NOT** to be confused with the specific Aries handshake connection protocol RFC 0160 - https://github.com/hyperledger/aries-rfcs/tree/main/features/0160-connection-protocol
    pub connection_service: Arc<Mutex<ConnectionService>>,
}

impl AriesFrameworkVCX {
    pub async fn initialize(framework_config: FrameworkConfig) -> VCXFrameworkResult<Self> {
        info!("Initializing Aries Framework VCX");

        // Warn if the wallet pass key being used is the sample key from the documentation
        if framework_config.wallet_config.pass_key() == "sample_pass_key" {
            warn!("The Default Wallet Pass Key SHOULD NOT be used in production");
        }

        // Wallet Initialization
        let wallet = Arc::new(framework_config.wallet_config.create_wallet().await?);

        // DID Resolver Registry
        // TODO - DID Sov Resolver
        let did_peer_resolver = PeerDidResolver::new();
        let did_resolver_registry =
            Arc::new(ResolverRegistry::new().register_resolver("peer".into(), did_peer_resolver));

        // Transport Resolver Registry
        let transport_resolver =
            TransportRegistry::new().register_transport(TransportProtocol::HTTP, HTTPTransport {});

        // Service Initializations
        let messaging_service = Arc::new(Mutex::new(MessagingService::new(
            framework_config.clone(),
            wallet.clone(),
            did_resolver_registry.clone(),
            transport_resolver,
        )));
        let invitation_service = Arc::new(Mutex::new(InvitationService::new(
            framework_config.clone(),
            wallet.clone(),
        )));
        let connection_service = Arc::new(Mutex::new(ConnectionService::new(
            framework_config.clone(),
            wallet.clone(),
            did_resolver_registry.clone(),
            messaging_service.clone(),
            invitation_service.clone(),
        )));

        Ok(Self {
            framework_config,
            wallet,
            did_resolver_registry,
            messaging_service,
            invitation_service,
            connection_service,
        })
    }
}

// TODO - Consider adding a way to register event emitters with restrictions on the type of events to listen to for a given emitter -- such as, only receive events for did-exchange response messages (rather than having to filter all events)
pub trait EventEmitter {
    type Event;
    fn emit_event(&mut self, event: Self::Event);
    fn register_event_receiver(&mut self) -> Receiver<Self::Event>;
}
