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

    #[test]
    fn lanes_of_nothing_is_empty() {
        assert!(lanes(&[]).is_empty());
    }

    #[test]
    fn touching_spans_do_not_share_a_lane_count() {
        assert_eq!(lanes(&[(0, 4), (4, 8)]), [(0, 1), (0, 1)]);
    }

    #[test]
    fn lanes_are_reused_after_a_span_ends() {
        assert_eq!(lanes(&[(0, 10), (1, 3), (4, 6)]), [(0, 2), (1, 2), (1, 2)]);
    }

    #[test]
    fn three_way_overlap_uses_three_lanes() {
        assert_eq!(lanes(&[(0, 9), (1, 9), (2, 9)]), [(0, 3), (1, 3), (2, 3)]);
    }

    #[test]
    fn input_order_does_not_change_the_assignment_per_span() {
        assert_eq!(lanes(&[(2, 6), (0, 4), (6, 8)]), [(1, 2), (0, 2), (0, 1)]);
    }
}
