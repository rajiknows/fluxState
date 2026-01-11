// so we will use io_uring finally

// so we are going to write a custom data structer into the shared memory and read and write from
// the both python and rust

pub const MAX_TENSORS: usize = 512;

#[repr(C)]
pub struct SharedHeader {
    ready_flag: u32,
    num_tensors: u32,
    total_data_size: u64,
    pub entries: [TensorEntry; MAX_TENSORS],
}

#[repr(C)]
pub enum Dtype {
    F32 = 0,
    F16 = 1,
}

#[repr(C)]
pub struct TensorEntry {
    pub offset: u64,
    pub size: u64,
    pub dtype: Dtype,
    // pub shape: [u64; 4], // suppport dimensions
}

// ok so this is what we are trying to achieve
//
// so the rust module will fetch the trained data from the shared memorory and
// upload/copy it to disk/s3 storage

// also for the communication between python and rust we will use gRPC first and then flatbuffers ,
// and we will compare the both to finally chose the best
fn main() {}
