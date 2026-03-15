use ahin_nervous_system::CogHash;
use sha2::{Digest, Sha256};

/// Lightweight append-only Merkle tree for simulation and batching.
#[derive(Debug, Default)]
pub struct ConcurrentMerkleTree {
    leaves: Vec<CogHash>,
}

impl ConcurrentMerkleTree {
    pub fn new() -> Self {
        Self { leaves: Vec::new() }
    }

    pub fn insert(&mut self, leaf: CogHash) {
        self.leaves.push(leaf);
    }

    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    pub fn root(&self) -> CogHash {
        if self.leaves.is_empty() {
            return [0u8; 32];
        }

        let mut level = self.leaves.clone();
        while level.len() > 1 {
            let mut next = Vec::with_capacity(level.len().div_ceil(2));
            for pair in level.chunks(2) {
                let left = pair[0];
                let right = if pair.len() == 2 { pair[1] } else { pair[0] };
                next.push(hash_pair(&left, &right));
            }
            level = next;
        }

        level[0]
    }
}

fn hash_pair(left: &CogHash, right: &CogHash) -> CogHash {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tree_has_zero_root() {
        let tree = ConcurrentMerkleTree::new();
        assert_eq!(tree.root(), [0u8; 32]);
    }

    #[test]
    fn root_changes_after_inserts() {
        let mut tree = ConcurrentMerkleTree::new();
        tree.insert([1u8; 32]);
        let single = tree.root();
        tree.insert([2u8; 32]);
        assert_ne!(single, tree.root());
    }
}
