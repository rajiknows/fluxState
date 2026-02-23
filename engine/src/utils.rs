use std::hash::{DefaultHasher, Hash, Hasher};

use libp2p::identity::Keypair;
pub fn generate_node_id() -> u64 {
    let mut hasher = DefaultHasher::new();
    let keypair = Keypair::generate_ed25519();
    let public = keypair.public();

    public.hash(&mut hasher);
    hasher.finish()
}
