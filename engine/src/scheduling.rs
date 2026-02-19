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

use core::f64;

#[derive(Debug, Clone, Default)]
struct DpState {
    // The state tracks r = (r1 ≤ r2 ≤ · · · ≤ rm)
    // as the sorted residual layer counts for partially assigned pipelines,
    // where each rj ∈ {1, 2, . . . , L − 1}
    r: Vec<usize>,
    // f as the count of fully assigned pipelines (containing all L layers).
    f: usize,
}

impl DpState {
    fn new() -> Self {
        Self {
            r: Vec::new(),
            f: 0,
        }
    }
    fn normalize(&mut self) {
        self.r.sort_unstable();
    }
}
#[derive(Debug, Clone)]
enum Decision {
    Skip,
    Extend(usize),
    StartNew,
}

struct ResultState {
    stages: usize,
    decision: Option<Decision>,
}

#[derive(Debug, Clone, Copy)]
struct Gpu {
    layer_cap: usize,
    compute_cap: usize,
}

pub fn phase1_naive(gpu_caps: &Vec<Gpu>, model_layer: usize, alpha: f64, r_rtt: f64, t_comp: f64) {
    let mut sorted = gpu_caps.clone();
    // non increasing order
    sorted.sort_unstable_by(|b, a| b.layer_cap.cmp(&a.layer_cap));

    let n = sorted.len();
    let total_cap: usize = sorted.iter().map(|g| g.layer_cap).sum();
    let k_max = n.min(total_cap / model_layer);

    // k is number of pipeline replication , we need to maximize k
    let mut best_k = 0;
    let mut best_score = f64::MIN;
    let mut best_trace = vec![];

    for k in 1..=k_max {
        let (s_star, trace) = solve_for_k(&sorted, model_layer, k);

        let z = (k as f64).powf(alpha) / (t_comp + (s_star as f64 / k as f64) * r_rtt);

        if z > best_score {
            best_score = z;
            best_k = k;
            best_trace = trace;
        }
    }
    println!("Selected k̂ = {best_k}");
    let pipelines = reconstruct(best_trace, &sorted);

    for (i, p) in pipelines.iter().enumerate() {
        println!("Pipeline {i}: {:?}", p);
    }
    for pipeline in &pipelines {
        let capacities: Vec<usize> = pipeline.iter().map(|p| p.layer_cap).collect();

        let compute: Vec<usize> = pipeline.iter().map(|p| p.compute_cap).collect();

        let layers = water_fill(model_layer, &capacities, &compute);

        println!("Layer allocation: {:?}", layers);
    }
}

fn water_fill(model_layer: usize, layer_cap: &[usize], compute_cap: &[usize]) -> Vec<usize> {
    let total_f: usize = compute_cap.iter().sum();

    let lambda = model_layer as f64 / total_f as f64;

    let mut frac: Vec<f64> = layer_cap
        .iter()
        .zip(compute_cap.iter())
        .map(|(&c, &f)| {
            let ideal = lambda * f as f64;
            ideal.min(c as f64)
        })
        .collect();

    let mut alloc: Vec<usize> = frac.iter().map(|x| x.floor() as usize).collect();

    let current_sum: usize = alloc.iter().sum();
    let mut remaining = model_layer.saturating_sub(current_sum);

    let mut remainders: Vec<(usize, f64)> = frac
        .iter()
        .enumerate()
        .map(|(i, &x)| (i, x - x.floor()))
        .collect();

    remainders.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Hamilton distribution
    for (idx, _) in remainders {
        if remaining == 0 {
            break;
        }
        if alloc[idx] < layer_cap[idx] {
            alloc[idx] += 1;
            remaining -= 1;
        }
    }

    alloc
}

fn solve_for_k(gpus: &Vec<Gpu>, model_layer: usize, k: usize) -> (usize, Vec<Decision>) {
    let mut trace = vec![];
    let res = dfs(
        0,
        gpus,
        model_layer,
        k,
        DpState::new(),
        &mut vec![],
        &mut trace,
    );
    (res, trace)
}

const INF: usize = usize::MAX / 4;
fn dfs(
    i: usize,
    gpus: &Vec<Gpu>,
    model_layer: usize,
    k: usize,
    state: DpState,
    path: &mut Vec<Decision>,
    best_path: &mut Vec<Decision>,
) -> usize {
    if i == gpus.len() {
        if state.f == k {
            *best_path = path.clone();
            return 0;
        }
        return INF;
    }

    let mut best = INF;
    let ci = gpus[i].layer_cap;

    // 1. skip
    path.push(Decision::Skip);
    let v = dfs(i + 1, gpus, model_layer, k, state.clone(), path, best_path);
    if v < best {
        best = v;
    }
    path.pop();

    // 2. extend
    for idx in 0..state.r.len() {
        let mut next = state.clone();
        next.r[idx] = next.r[idx].saturating_sub(ci);

        if next.r[idx] == 0 {
            next.r.remove(idx);
            next.f += 1;
        }

        next.normalize();

        path.push(Decision::Extend(idx));
        let v = 1 + dfs(i + 1, gpus, model_layer, k, next, path, best_path);
        if v < best {
            best = v;
        }
        path.pop();
    }

    // 3. start new

    if state.f + state.r.len() < k {
        let mut next = state.clone();
        let residual = model_layer.saturating_sub(ci);

        if residual == 0 {
            next.f += 1;
        } else {
            next.r.push(residual);
            next.normalize();
        }

        path.push(Decision::StartNew);
        let v = 1 + dfs(i + 1, gpus, model_layer, k, next, path, best_path);
        if v < best {
            best = v;
        }
        path.pop();
    }
    best
}
fn reconstruct(trace: Vec<Decision>, gpus: &Vec<Gpu>) -> Vec<Vec<Gpu>> {
    let mut pipelines: Vec<Vec<usize>> = vec![];
    let mut active: Vec<usize> = vec![];

    for (gpu_idx, decision) in trace.iter().enumerate() {
        match decision {
            Decision::Skip => {}
            Decision::StartNew => {
                pipelines.push(vec![gpu_idx]);
                active.push(pipelines.len() - 1);
            }
            Decision::Extend(p_idx) => {
                if let Some(&pipe_id) = active.get(*p_idx) {
                    pipelines[pipe_id].push(gpu_idx);
                }
            }
        }
    }

    let mut result: Vec<Vec<Gpu>> = vec![];

    for (pid, pipe) in pipelines.iter().enumerate() {
        println!("Pipeline {pid}:");
        let mut current = vec![];
        for (stage, gpu_idx) in pipe.iter().enumerate() {
            let gpu = gpus[*gpu_idx];
            println!(
                "  Stage {stage} -> GPU {gpu_idx} (cap={}, compute={})",
                gpu.layer_cap, gpu.compute_cap
            );
            current.push(gpu);
        }
        println!();
        result.push(current);
    }

    result
}

pub fn main() {
    let gpus = vec![
        Gpu {
            layer_cap: 6,
            compute_cap: 1,
        },
        Gpu {
            layer_cap: 6,
            compute_cap: 2,
        },
        Gpu {
            layer_cap: 6,
            compute_cap: 3,
        },
        Gpu {
            layer_cap: 6,
            compute_cap: 2,
        },
        Gpu {
            layer_cap: 6,
            compute_cap: 1,
        },
    ];

    let model_layer = 10;

    let alpha = 1.0;
    let t_comp = 10.0;
    let r_rtt = 1.0;

    phase1_naive(&gpus, model_layer, alpha, r_rtt, t_comp);
}
