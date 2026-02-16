import mmap
import os
import ctypes
import torch
import platform
import grpc
import sys

# --- CONFIGURATION ---
MAX_NAME_LEN = 64
MAX_TENSORS = 512

# --- PATH HACK FOR PROTOBUF ---
# Get the absolute path of the 'python' folder
current_dir = os.path.dirname(os.path.abspath(__file__))
# Define the path to the generated code folder (python/flux_state)
generated_dir = os.path.join(current_dir, "flux_state")

# Add BOTH to sys.path so Python can find the modules
sys.path.append(current_dir)
sys.path.append(generated_dir)

# Now import directly (Do not use 'from flux_state ...')
import flux_pb2
import flux_pb2_grpc


# --- SHARED MEMORY STRUCTS ---
class TensorEntry(ctypes.Structure):
    _fields_ = [
        ("name", ctypes.c_char * MAX_NAME_LEN),
        ("offset", ctypes.c_uint64),
        ("size", ctypes.c_uint64),
        ("dtype", ctypes.c_uint8),
        ("_pad", ctypes.c_uint8 * 7),
    ]


class SharedHeader(ctypes.Structure):
    _fields_ = [
        ("ready_flag", ctypes.c_uint32),
        ("num_tensors", ctypes.c_uint32),
        ("total_data_size", ctypes.c_uint64),
        ("entries", TensorEntry * MAX_TENSORS),
    ]


# --- OS DETECTION ---
SHM_SIZE = 1024 * 1024 * 100  # 100 MB
if platform.system() == "Linux":
    SHM_PATH = "/dev/shm/flux_state_1"
else:
    SHM_PATH = "/tmp/flux_state_1"


# --- HELPER FUNCTIONS ---
def create_shared_memory():
    with open(SHM_PATH, "wb") as f:
        f.truncate(SHM_SIZE)
    fd = os.open(SHM_PATH, os.O_RDWR)
    return mmap.mmap(fd, SHM_SIZE)


def write_tensors(buf, state_dict):
    header = SharedHeader.from_buffer(buf)
    header.ready_flag = 0

    heap_start = ctypes.sizeof(SharedHeader)
    current_offset = heap_start
    idx = 0

    print(f"[Python] Writing {len(state_dict)} tensors to Shared Memory...")

    for name, tensor in state_dict.items():
        entry = header.entries[idx]

        # Metadata
        entry.name = name.encode("utf-8")[:MAX_NAME_LEN]
        data_bytes = tensor.numpy().tobytes()
        d_len = len(data_bytes)

        # Write Data to Heap
        buf[current_offset : current_offset + d_len] = data_bytes

        # Update Entry
        entry.offset = current_offset
        entry.size = d_len
        entry.dtype = 0  # F32

        current_offset += d_len
        idx += 1

    header.num_tensors = idx
    header.total_data_size = current_offset - heap_start
    header.ready_flag = 1
    print(f"[Python] Data written. Total size: {header.total_data_size} bytes.")


def trigger_save(stub):
    print("[Python] Sending gRPC SaveRequest to Rust...")
    request = flux_pb2.SaveRequest(
        req_id="ckpt_alpha", shm_path=SHM_PATH, expected_tensors=2
    )
    try:
        response = stub.saveCheckPoint(request)
        print(
            f"[Python] RPC Response: Success={response.success}, Written={response.bytes_written}"
        )
    except grpc.RpcError as e:
        print(f"[Python] RPC Failed: {e}")


# --- MAIN EXECUTION ---
if __name__ == "__main__":
    # 1. Define Dummy Model Data
    tensors = {
        "layer1.weight": torch.randn(1024, 1024),  # 4MB
        "layer2.bias": torch.randn(1024),  # 4KB
    }

    # 2. Setup Memory
    buf = create_shared_memory()

    # 3. Write Data (Data Plane)
    write_tensors(buf, tensors)

    # 4. Trigger Rust (Control Plane)
    with grpc.insecure_channel("localhost:50051") as channel:
        stub = flux_pb2_grpc.FluxControlStub(channel)
        trigger_save(stub)

