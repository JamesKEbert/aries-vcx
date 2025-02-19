use std::{str::FromStr, sync::Arc};

use aries_vcx::{
    aries_vcx_wallet::wallet::{
        askar::{
            askar_wallet_config::AskarWalletConfig,
            key_method::{ArgonLevel, AskarKdfMethod, KeyMethod},
        },
        base_wallet::ManageWallet,
    },
    did_peer::resolver::PeerDidResolver,
};
use did_resolver_registry::ResolverRegistry;
use url::Url;
use vcx_framework::{
    connection_service::{self, ConnectionService},
    messaging_service::MessagingService,
    repositories::{
        connection_repository::{
            ConnectionRecordData, ConnectionRecordTagKeys, ConnectionRepository,
        },
        did_repository::{DidRecordData, DidRecordTagKeys, DidRepository},
        invitation_repository::{
            self, InvitationRecordData, InvitationRecordTagKeys, InvitationRepository,
        },
    },
    storage::in_memory_storage::InMemoryStorage,
    transport::{HttpTransport, TransportRegistry},
};

mod common;

pub const IN_MEMORY_DB_URL: &str = "sqlite://:memory:";
pub const DEFAULT_WALLET_PROFILE: &str = "aries_framework_vcx_default";
pub const DEFAULT_ASKAR_KEY_METHOD: KeyMethod = KeyMethod::DeriveKey {
    inner: AskarKdfMethod::Argon2i {
        inner: (ArgonLevel::Interactive),
    },
};

#[tokio::test]
async fn connect() {
    common::test_init();

    let agent_endpoint = Url::from_str("localhost:3010").expect("Valid URL");
    let agent_label = String::from("VCX Holder Agent");

    let did_peer_resolver = PeerDidResolver::new();
    let did_resolver_registry =
        Arc::new(ResolverRegistry::new().register_resolver("peer".into(), did_peer_resolver));

    let in_memory_storage = InMemoryStorage::<ConnectionRecordData, ConnectionRecordTagKeys>::new();
    let connection_repository = Arc::new(ConnectionRepository::new(Box::new(in_memory_storage)));

    let in_memory_storage_invitations =
        InMemoryStorage::<InvitationRecordData, InvitationRecordTagKeys>::new();
    let invitation_repository = Arc::new(InvitationRepository::new(Box::new(
        in_memory_storage_invitations,
    )));

    let in_memory_storage_dids = InMemoryStorage::<DidRecordData, DidRecordTagKeys>::new();
    let did_repository = Arc::new(DidRepository::new(Box::new(in_memory_storage_dids)));

    let wallet_config = AskarWalletConfig {
        db_url: IN_MEMORY_DB_URL.to_string(),
        key_method: DEFAULT_ASKAR_KEY_METHOD,
        pass_key: "sample_pass_key".to_string(),
        profile: DEFAULT_WALLET_PROFILE.to_string(),
    };
    let wallet = Arc::new(
        wallet_config
            .create_wallet()
            .await
            .expect("valid wallet to be created"),
    );

    let transport_registry =
        Arc::new(TransportRegistry::new().register_transport(HttpTransport::new()));

    let messaging_service = Arc::new(MessagingService::new(
        did_resolver_registry.clone(),
        transport_registry,
        connection_repository.clone(),
        did_repository.clone(),
        wallet.clone(),
    ));

    let connection_service = ConnectionService::new(
        did_resolver_registry,
        connection_repository,
        invitation_repository,
        did_repository,
        messaging_service,
        wallet,
        agent_endpoint,
        agent_label,
    );

    // let invitation = connection_service.connect(invitation, mediated, specific_mediator_id);
}
