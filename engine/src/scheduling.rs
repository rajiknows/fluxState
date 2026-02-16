//! -----------------------------------------------------------------------------
//! Phase-1 Scheduling
//! -----------------------------------------------------------------------------
//!
//! To perform Phase-1 scheduling, we propose a dynamic programming algorithm
//! that implements the region-based and latency-dominant heuristic to obtain a
//! model allocation strategy that minimizes pipeline inference latencies while
//! maximizing overall system throughput.
//!
//! We define c_i ∈ N^+ to be the maximum layer capacity of GPU g_i,
//! k to be the number of pipeline replications,
//! and s*(k) to be the minimum total number of stages required to
//! accommodate k pipeline replications.
//!
//! Our objective is to maximize the number of replications k while
//! minimizing the average stages per replication s*(k)/k.
//!
//! The procedure follows three steps:
//!
//! (i) P1-Initialization:
//! The algorithm sorts GPU layer capacities in non-increasing order
//! to obtain:
//!
//!     c = (c1 ≥ · · · ≥ cN)
//!
//! and computes the maximum possible replication number:
//!
//!     k_max = min(N, floor((Σ_{i=1..N} c_i) / L))
//!
//! It initializes a dynamic programming state for Phase 1 scheduling
//! noted by dp1(0, ∅, 0) for each k ∈ {1, . . . , k_max}
//! with an empty multiset of residuals for partially assigned pipelines,
//! zero fully assigned pipelines, and a companion table of back-pointers.
//!
//! (ii) P1-DP exploration:
//! The dynamic programming state dp1(i, r, f) represents the assignment
//! status when processing GPU g_i (with capacity c_i) for target
//! replication count k.
//!
//! The state tracks:
//!
//!     r = (r1 ≤ r2 ≤ · · · ≤ rm)
//!
//! as the sorted residual layer counts for partially assigned pipelines,
//! where each r_j ∈ {1, 2, . . . , L − 1},
//! and f as the count of fully assigned pipelines (containing all L layers).
//!
//! At each GPU indexed by i, the algorithm considers three transitions:
//!
//! ❶ Skip GPU:
//!     Transition to dp1(i+1, r, f) without assigning the i-th GPU
//!     to any pipeline.
//!
//! ❷ Extend existing pipeline:
//!     Select a partially assigned pipeline j and assign the i-th GPU
//!     to this pipeline.
//!
//!     Update the residual count:
//!
//!         r_j ← r_j − c_i
//!
//!     If r_j ≤ 0, the pipeline becomes fully assigned
//!     (increment f and remove r_j from r).
//!
//! ❸ Start new pipeline:
//!     Create a new pipeline starting with the i-th GPU,
//!     subject to the constraint:
//!
//!         f + |r| < k
//!
//!     Initialize residual count:
//!
//!         r = L − c_i
//!
//!     If r ≤ 0, the pipeline is immediately fully assigned
//!     (increment f); otherwise, add r to r.
//!
//! The algorithm evaluates all valid transitions,
//! records the one yielding the minimum number of pipeline stages,
//! and stores the corresponding decision pointer for backtracking.
//!
//! (iii) P1-Objective evaluation and reconstruction:
//! The algorithm sets:
//!
//!     s*(k) = dp1(0, ∅, 0)
//!
//! and, for each k ∈ {1, . . . , k_max}, computes:
//!
//!     Z(k) = k^α / (T_comp + (s*(k)/k) r_RTT)
//!
//! Note that α > 0 controls how strongly the score favors additional
//! replications relative to the per-replication latency term,
//! T_comp is the average per-replication compute time (excluding communication),
//! and r_RTT is the average inter-stage hop latency obtained from profiling.
//!
//! The algorithm then selects:
//!
//!     k̂ = arg max_k Z(k)
//!
//! backtracks decisions to recover GPU-to-pipeline assignments,
//! and emits contiguous layer blocks per stage in pipeline order
//! using a write cursor to ensure gap-free layer placement.
//! -----------------------------------------------------------------------------

pub fn phase1_initialize(gpu_layer_cap: &mut Vec<usize>, model_layer: usize) -> usize {
    gpu_layer_cap.sort_unstable_by(|a, b| b.cmp(a));

    let n = gpu_layer_cap.len();
    let total_capacity: usize = gpu_layer_cap.iter().sum();

    let k_max = n.min(total_capacity / model_layer);

    k_max
}
