use alloc::vec::Vec;
use core::primitive::{u32, usize};

/// ```
/// # use app::calendar::layout::lanes;
/// assert_eq!(lanes(&[(0, 4), (2, 6), (6, 8)]), [(0, 2), (1, 2), (0, 1)]);
/// ```
pub fn lanes(spans: &[(u32, u32)]) -> Vec<(u32, u32)> {
    let mut order: Vec<usize> = (0..spans.len()).collect();
    order.sort_by(|a, b| spans[*a].0.cmp(&spans[*b].0).then(spans[*b].1.cmp(&spans[*a].1)));

    let mut result = alloc::vec![(0, 1); spans.len()];
    let mut cluster: Vec<(usize, u32, u32)> = Vec::new();
    let mut cluster_end = 0;
    for index in order {
        let (start, end) = spans[index];
        if !cluster.is_empty() && start >= cluster_end {
            flush(&mut cluster, &mut result);
        }
        let mut lane = 0;
        while cluster
            .iter()
            .any(|(_, other_end, other_lane)| *other_end > start && *other_lane == lane)
        {
            lane += 1;
        }
        cluster_end = if cluster.is_empty() { end } else { cluster_end.max(end) };
        cluster.push((index, end, lane));
    }
    flush(&mut cluster, &mut result);
    result
}

fn flush(cluster: &mut Vec<(usize, u32, u32)>, result: &mut [(u32, u32)]) {
    let count = cluster.iter().map(|(_, _, lane)| lane + 1).max().unwrap_or(1);
    for (index, _, lane) in cluster.drain(..) {
        result[index] = (lane, count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Rng;

    fn overlap(a: (u32, u32), b: (u32, u32)) -> bool {
        a.0 < b.1 && b.0 < a.1
    }

    fn deepest_overlap(spans: &[(u32, u32)], members: &[usize]) -> u32 {
        members
            .iter()
            .map(|&i| {
                let at = spans[i].0;
                members.iter().filter(|&&j| spans[j].0 <= at && at < spans[j].1).count() as u32
            })
            .max()
            .unwrap_or(1)
    }

    #[test]
    fn random_spans_get_conflict_free_minimal_lanes_independent_of_input_order() {
        for seed in 0..3000 {
            let mut rng = Rng::new(seed);
            let mut spans: Vec<(u32, u32)> = Vec::new();
            for _ in 0..rng.below(12) {
                let start = rng.below(30) as u32;
                let span = (start, start + 1 + rng.below(12) as u32);
                if !spans.contains(&span) {
                    spans.push(span);
                }
            }
            let assigned = lanes(&spans);
            assert_eq!(assigned.len(), spans.len(), "seed {seed}");

            let mut cluster = (0..spans.len()).collect::<Vec<usize>>();
            for i in 0..spans.len() {
                for j in 0..spans.len() {
                    if overlap(spans[i], spans[j]) {
                        let (low, high) = (cluster[i].min(cluster[j]), cluster[i].max(cluster[j]));
                        for entry in cluster.iter_mut() {
                            if *entry == high {
                                *entry = low;
                            }
                        }
                    }
                }
            }
            for i in 0..spans.len() {
                let (lane, count) = assigned[i];
                assert!(lane < count, "seed {seed} span {i}");
                for j in 0..spans.len() {
                    if i != j && overlap(spans[i], spans[j]) {
                        assert_ne!(lane, assigned[j].0, "seed {seed} spans {i} {j}");
                    }
                }
                let members: Vec<usize> =
                    (0..spans.len()).filter(|&j| cluster[j] == cluster[i]).collect();
                assert_eq!(count, deepest_overlap(&spans, &members), "seed {seed} span {i}");
            }

            let mut order: Vec<usize> = (0..spans.len()).collect();
            for index in (1..order.len()).rev() {
                order.swap(index, rng.below(index + 1));
            }
            let shuffled: Vec<(u32, u32)> = order.iter().map(|&i| spans[i]).collect();
            let reassigned = lanes(&shuffled);
            for (position, &original) in order.iter().enumerate() {
                assert_eq!(reassigned[position], assigned[original], "seed {seed} shuffled");
            }
        }
    }
}
