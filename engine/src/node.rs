// here we will represent a node
// a node will have
// node_address ,
// location( will be needed in scheduling for nearer nodes)
// gpu_score
// gpu cores
// network_bandwidth
//
//
#[derive(Debug, Clone)]
struct Node {
    addr: String,
    region: String,
    gpu_score: usize,
    gpu_cores: usize,
    network_bandwidth: usize,
    layer_capacity: usize,
}

impl Node {
    pub fn new(addr: String) -> Node {
        // identify location if location permission is off request permission or terminate

        // identify gpu on system and derive cores and information about the gpu

        // calculate the network_bandwidth

        // build the Node
        todo!()
    }
}
