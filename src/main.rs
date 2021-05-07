mod transport;
use crate::transport::{StegosBehaviour, build_transport, configure_gossip, Response};
mod resources;

use log::info;
use once_cell::sync::Lazy;
use env_logger::{Builder, Env};
use tokio::{io::{self, AsyncBufReadExt}, sync::mpsc};


use libp2p::{Multiaddr, identity, PeerId, Swarm};
use libp2p::gossipsub::{IdentTopic as Topic, Gossipsub, MessageAuthenticity};
use libp2p::mdns::Mdns;
use libp2p_swarm::SwarmBuilder;

static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("usage"));

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Builder::from_env(Env::default().default_filter_or("info")).init();
    info!("starting PeerId: {:?}", PEER_ID.to_string());

    //channel used to respond to the received pub/sub requests
    let (response_tx, mut response_rx) = mpsc::unbounded_channel::<Response>();

    let mut swarm = {
        let transport = build_transport(&KEYS);
        let mdns = Mdns::new(Default::default()).await?;
        let gossipsub_config = configure_gossip();
        let mut behaviour = StegosBehaviour {
            gossipsub: Gossipsub::new(MessageAuthenticity::Signed(KEYS.to_owned()), gossipsub_config).unwrap(),
            mdns,
            response_tx,
        };

        behaviour.gossipsub.subscribe(&TOPIC).unwrap();

        SwarmBuilder::new(transport, behaviour, PEER_ID.to_owned())
            // We want the connection background tasks to be spawned
            // onto the tokio runtime.
            .executor(Box::new(|fut| { tokio::spawn(fut); }))
            .build()
    };

    // Reach out to another node if specified
    if let Some(to_dial) = std::env::args().nth(1) {
        let addr: Multiaddr = to_dial.parse()?;
        swarm.dial_addr(addr)?;
        println!("Dialed {:?}", to_dial)
    }

    // Read full lines from stdin
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    // Listen on all interfaces and whatever port the OS assigns
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    // Kick it off
    let mut listening = false;
    loop {
        let to_publish = {
            tokio::select! {
                line = stdin.next_line() => {
                    let line = line?.expect("stdin closed");
                    Some((TOPIC.to_owned(), line))
                }
                event = swarm.next() => {
                    // All events are handled by the `NetworkBehaviourEventProcess`es.
                    info!("Unhandled Swarm Event: {:?}", event);
                    None
                }
            }
        };
        if let Some((topic, line)) = to_publish {
            swarm.behaviour_mut().gossipsub.publish(topic, line.as_bytes()).unwrap();
        }
        if !listening {
            for addr in Swarm::listeners(&swarm) {
                println!("Listening on {:?}", addr);
                listening = true;
            }
        }
    }
}
