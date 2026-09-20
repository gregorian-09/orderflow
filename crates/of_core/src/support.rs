use std::collections::HashMap;
use std::hash::Hash;

pub(crate) const DEFAULT_PATTERN_HISTORY_CAP: usize = 4096;
pub(crate) const DEFAULT_PATTERN_PRICE_LEVEL_CAP: usize = 8192;
pub(crate) const MAX_SESSION_TRADES: usize = 100_000;

pub(crate) fn bounded_push<T>(items: &mut Vec<T>, max_len: usize, item: T) {
    if max_len == 0 {
        return;
    }
    if items.len() >= max_len {
        items.remove(0);
    }
    items.push(item);
}

pub(crate) fn bounded_push_pair<T, U>(
    left: &mut Vec<T>,
    right: &mut Vec<U>,
    max_len: usize,
    left_item: T,
    right_item: U,
) {
    if max_len == 0 {
        return;
    }
    if left.len() >= max_len {
        left.remove(0);
        if !right.is_empty() {
            right.remove(0);
        }
    }
    left.push(left_item);
    right.push(right_item);
}

pub(crate) fn prune_hash_map<K, V>(items: &mut HashMap<K, V>, max_len: usize)
where
    K: Eq + Hash + Clone,
{
    if max_len == 0 {
        items.clear();
        return;
    }
    while items.len() > max_len {
        let Some(key) = items.keys().next().cloned() else {
            break;
        };
        items.remove(&key);
    }
}
