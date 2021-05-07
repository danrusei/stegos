use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tokio::time::Duration;
//use tokio::io::{self, AsyncBufReadExt};

use libp2p::core::{muxing::StreamMuxerBox, transport, upgrade};
use libp2p::gossipsub::{
    Gossipsub, GossipsubConfig, GossipsubConfigBuilder, GossipsubEvent, GossipsubMessage,
    MessageId, ValidationMode,
};
use libp2p::mdns::{Mdns, MdnsEvent};
use libp2p::{identity, mplex, noise, tcp::TokioTcpConfig, PeerId, Transport};
use libp2p_swarm::NetworkBehaviourEventProcess;
use libp2p_swarm_derive::*;

// We create a custom network behaviour that combines floodsub and mDNS.
// The derive generates a delegating `NetworkBehaviour` impl which in turn
// requires the implementations of `NetworkBehaviourEventProcess` for
// the events of each behaviour.
#[derive(NetworkBehaviour)]
pub(crate) struct StegosBehaviour {
    pub(crate) gossipsub: Gossipsub,
    pub(crate) mdns: Mdns,
}

impl NetworkBehaviourEventProcess<GossipsubEvent> for StegosBehaviour {
    // Called when `gossipsub` produces an event.
    fn inject_event(&mut self, event: GossipsubEvent) {
        match event {
            GossipsubEvent::Message {
                propagation_source: peer_id,
                message_id: id,
                message,
            } => println!(
                "Got message: {} with id: {} from peer: {:?}",
                String::from_utf8_lossy(&message.data),
                id,
                peer_id
            ),
            _ => {}
        }
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for StegosBehaviour {
    // Called when `mdns` produces an event.
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(list) => {
                for (peer, _) in list {
                    self.gossipsub.add_explicit_peer(&peer);
                }
            }
            MdnsEvent::Expired(list) => {
                for (peer, _) in list {
                    if !self.mdns.has_node(&peer) {
                        self.gossipsub.blacklist_peer(&peer);
                    }
                }
            }
        }
    }
}

pub(crate) fn build_transport(
    keys: &identity::Keypair,
) -> transport::Boxed<(PeerId, StreamMuxerBox)> {
    // Create a keypair for authenticated encryption of the transport.
    let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(keys)
        .expect("Signing libp2p-noise static DH keypair failed.");

    // Create a tokio-based TCP transport use noise for authenticated
    // encryption and Mplex for multiplexing of substreams on a TCP stream.
    let transport = TokioTcpConfig::new()
        .nodelay(true)
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();
    transport
}

pub(crate) fn configure_gossip() -> GossipsubConfig {
    // To content-address message, we can take the hash of message and use it as an ID.
    let message_id_fn = |message: &GossipsubMessage| {
        let mut s = DefaultHasher::new();
        message.data.hash(&mut s);
        MessageId::from(s.finish().to_string())
    };

    // Set a custom gossipsub
    let gossipsub_config = GossipsubConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
        .validation_mode(ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
        .message_id_fn(message_id_fn) // content-address messages. No two messages of the
        // same content will be propagated.
        .build()
        .expect("Valid config");
    gossipsub_config
}
