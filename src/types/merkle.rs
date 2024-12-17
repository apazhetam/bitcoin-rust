use super::hash::{Hashable, H256};
use ring::digest::{Context, SHA256};
use hex_literal::hex;

/// A Merkle tree.
#[derive(Debug, Default)]
pub struct MerkleTree {
    root: Option<H256>,
    nodes: Vec<Option<H256>>,
    leaf_count: usize
}

impl MerkleTree {
    /// Creates a new Merkle tree, given a slice of Hashable data as input. 
    pub fn new<T>(data: &[T]) -> Self where T: Hashable, {
        if data.is_empty() {
            // handle empty input case
            let item: H256 = (hex!("0000000000000000000000000000000000000000000000000000000000000000")).into();
            return MerkleTree {
                root: Some(item),
                nodes: Vec::new(),
                leaf_count: 0,
            };
        }
        
        let base: i32 = 2; // base for exponentials

        let mut leaf_count = data.len();
        let mut nodes = vec![None; 2 * leaf_count.next_power_of_two() - 1];

        let max_level = ((leaf_count.next_power_of_two()) as f32).log2() as i32;
        let first_leaf_index = base.pow(max_level as u32) as usize - 1;

        // Fill in the leaf nodes with hashed data
        for (i, item) in data.iter().enumerate() {
            nodes[first_leaf_index + i] = Some(item.hash());
        }

        // Add duplicate node to leaf row if it has odd number of elements
        if leaf_count % 2 == 1 && max_level > 0 {
            nodes[first_leaf_index + leaf_count] = nodes[first_leaf_index + leaf_count - 1];
            leaf_count = leaf_count + 1;
        }
    
        let mut level_count = leaf_count / 2;

        for level in (0..max_level).rev() {
            let level_first_index = base.pow(level as u32) as usize - 1;

            for i in 0..level_count {
                let current_index = level_first_index + i;
                let left = nodes[2 * current_index + 1].clone().unwrap_or_default();
                let right = nodes[2 * current_index + 2].clone().unwrap_or_default();

                // Use left and right hashes to create a combined hash
                let mut context = Context::new(&SHA256);
                context.update(&left.as_ref());
                context.update(&right.as_ref());
                let combined_hash = context.finish();
                
                nodes[current_index] = Some(combined_hash.into());
            }
            
            // add duplicate to end of row if necessary
            if level_count % 2 == 1 && level > 0 {
                nodes[level_first_index + level_count] = nodes[level_first_index + level_count - 1];
                level_count += 1;
            }

            // update max_level count
            level_count = level_count / 2;
        }

        MerkleTree {
            root: nodes[0].clone(),
            nodes: nodes,
            leaf_count: leaf_count,
        }
    }

    /// Returns the root of the given Merkle tree.
    pub fn root(&self) -> H256 {
        self.root.unwrap()
    }

    /// Returns the Merkle Proof of data at index i, as a vector of hashes.
    pub fn proof(&self, index: usize) -> Vec<H256> {
        if index >= self.leaf_count {
            // Return an empty vector if the index is out of bounds
            return Vec::new();
        }

        let mut proof = Vec::new();
        let mut current_index = (self.nodes.len().next_power_of_two() as f32 / 2.0) as usize - 1 + index;
        let max_level = ((self.nodes.len().next_power_of_two()) as f32).log2() as i32 - 1;

        // Start from the leaf level and go upwards through tree (excluding root)
        for _level in (1..(max_level + 1)).rev() {
            if current_index % 2 == 0 {
                // If the current node is a right child, add the sibling on the left
                let sibling_index = current_index - 1;
                let sibling_hash = &self.nodes[sibling_index];
                proof.push(sibling_hash.unwrap());
                current_index = (current_index - 2) / 2;
            } 
            else {
                // If the current node is a left child, add the sibling on the right
                let sibling_index = current_index + 1;
                let sibling_hash = &self.nodes[sibling_index];
                proof.push(sibling_hash.unwrap());
                current_index = (current_index - 1) / 2;
            }
        }

        proof
    }
}

/// Verify that the datum hash with a vector of proofs will produce the Merkle root. Also need the
/// index of datum and `leaf_size`, the total number of leaves.
pub fn verify(root: &H256, datum: &H256, proof: &[H256], index: usize, leaf_size: usize) -> bool {
    // Check if the provided index is valid
    if index >= leaf_size {
        return false;
    }

    let mut current_index = leaf_size.next_power_of_two() - 1 + index;
    let mut current_hash = datum.clone();

    for sibling_hash in proof.iter() {
        let mut context = Context::new(&SHA256);

        // Check if current node is left or right child, 
        // in order to preserve the original order in the combined hashing
        if current_index % 2 == 0 {   
            // current node is a right child, so hash sibling & current
            context.update(&sibling_hash.as_ref());
            context.update(&current_hash.as_ref());

            // move current_index up to parent
            current_index = (current_index - 2) / 2;
        }
        else {                          
            // current node is a left child, so hash current & sibling
            context.update(&current_hash.as_ref());
            context.update(&sibling_hash.as_ref());
            
            // move current_index up to parent
            current_index = (current_index - 1) / 2;
        }

        // Create a combined hash using current and sibling hashes
        let combined_hash = context.finish();        
        current_hash = combined_hash.into();
    }

    // At the end of the loop, current_hash should be the calculated Merkle root
    current_hash == *root
}
// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. BEFORE TEST

#[cfg(test)]
mod tests {
    use crate::types::hash::H256;
    use super::*;

    macro_rules! gen_merkle_tree_data {
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
            ]
        }};
    }

    #[test]
    fn merkle_root() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let root = merkle_tree.root();
        assert_eq!(
            root,
            (hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into()
        );
        // "b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0" is the hash of
        // "0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d"
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
        // "6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920" is the hash of
        // the concatenation of these two hashes "b69..." and "965..."
        // notice that the order of these two matters
    }

    #[test]
    fn merkle_proof() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert_eq!(proof,
            vec![hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f").into()]
        );
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
    }

    #[test]
    fn merkle_verifying() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 0, input_data.len()));
    }

    // define a slice of Hashable data of length 6
    macro_rules! gen_merkle_tree_data2 {
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
            ]
        }};
    }

    #[test]
    fn merkle_nodes_v1() {
        // generate a merkle tree starting with 6 leaf nodes
        let input_data: Vec<H256> = gen_merkle_tree_data2!();
        let merkle_tree = MerkleTree::new(&input_data);
        let root = merkle_tree.root();
        let nodes = merkle_tree.nodes;
        
        assert_eq!(
            nodes[7].unwrap(),
            (hex!("b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0")).into()
        );
        assert_eq!(
            nodes[8].unwrap(),
            (hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f")).into()
        );
        assert_eq!(
            nodes[3].unwrap(),
            (hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into()
        );
        assert_eq!(
            nodes[4].unwrap(),
            (hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into()
        );
        assert_eq!(
            nodes[5].unwrap(),
            (hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into()
        );
        assert_eq!(
            nodes[6].unwrap(),
            (hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into()
        );
        assert_eq!(
            nodes[1].unwrap(),
            nodes[2].unwrap()
        );
        assert_eq!(
            root,
            nodes[0].unwrap(),
        );
        
        // "b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0" is the hash of
        // "0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d"
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
        // "6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920" is the hash of
        // the concatenation of these two hashes "b69..." and "965..."
        // notice that the order of these two matters
    }
    
    #[test]
    fn merkle_proof_v2() {
        let input_data: Vec<H256> = gen_merkle_tree_data2!();
        let merkle_tree = MerkleTree::new(&input_data);
        let nodes = merkle_tree.nodes.clone();
        
        let proof_1 = merkle_tree.proof(0);
        // data point at index 0 refers to nodes[7]
        // thus proof_1 should be hashes of nodes [8, 4, 2]
        assert_eq!(proof_1.len(), 3);
        assert_eq!(proof_1[0], nodes[8].unwrap());
        assert_eq!(proof_1[1], nodes[4].unwrap());
        assert_eq!(proof_1[2], nodes[2].unwrap());

        let proof_2 = merkle_tree.proof(6);  // invalid index
        assert_eq!(proof_2.len(), 0);
        
        let proof_3 = merkle_tree.proof(5);
        // data point at index 5 refers to nodes[12]
        // thus proof_3 should be hashes of nodes [11, 6, 1]
        assert_eq!(proof_3.len(), 3);
        assert_eq!(proof_3[0], nodes[11].unwrap());
        assert_eq!(proof_3[1], nodes[6].unwrap());
        assert_eq!(proof_3[2], nodes[1].unwrap());

        let proof_4 = merkle_tree.proof(3);
        // data point at index 3 refers to nodes[10]
        // thus proof_4 should be hashes of nodes [9, 3, 2]
        assert_eq!(proof_4.len(), 3);
        assert_eq!(proof_4[0], nodes[9].unwrap());
        assert_eq!(proof_4[1], nodes[3].unwrap());
        assert_eq!(proof_4[2], nodes[2].unwrap());
    }

    #[test]
    fn merkle_verifying_v2() {
        let input_data: Vec<H256> = gen_merkle_tree_data2!();
        let merkle_tree = MerkleTree::new(&input_data);
        let mut proof; 
        
        proof = merkle_tree.proof(0);
        assert!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 0, input_data.len()));

        proof = merkle_tree.proof(5);
        assert!(verify(&merkle_tree.root(), &input_data[5].hash(), &proof, 5, input_data.len()));
    }

    // define a slice of Hashable data of length 6
    macro_rules! gen_merkle_tree_data3 {
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
                (hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into(),
                (hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
            ]
        }};
    }

    #[test]
    fn merkle_proof_v3() {
        let input_data: Vec<H256> = gen_merkle_tree_data3!();
        let merkle_tree = MerkleTree::new(&input_data);
        let nodes = merkle_tree.nodes.clone();
        
        let proof_1 = merkle_tree.proof(0);
        // data point at index 0 refers to nodes[7]
        // thus proof_1 should be hashes of nodes [8, 4, 2]
        assert_eq!(proof_1.len(), 3);
        assert_eq!(proof_1[0], nodes[8].unwrap());
        assert_eq!(proof_1[1], nodes[4].unwrap());
        assert_eq!(proof_1[2], nodes[2].unwrap());

        let proof_2 = merkle_tree.proof(6);  // invalid index
        assert_eq!(proof_2.len(), 0);
        
        let proof_3 = merkle_tree.proof(5);
        // data point at index 5 refers to nodes[12]
        // thus proof_3 should be hashes of nodes [11, 6, 1]
        assert_eq!(proof_3.len(), 3);
        assert_eq!(proof_3[0], nodes[11].unwrap());
        assert_eq!(proof_3[1], nodes[6].unwrap());
        assert_eq!(proof_3[2], nodes[1].unwrap());

        let proof_4 = merkle_tree.proof(3);
        // data point at index 3 refers to nodes[10]
        // thus proof_4 should be hashes of nodes [9, 3, 2]
        assert_eq!(proof_4.len(), 3);
        assert_eq!(proof_4[0], nodes[9].unwrap());
        assert_eq!(proof_4[1], nodes[3].unwrap());
        assert_eq!(proof_4[2], nodes[2].unwrap());
    }

    #[test]
    fn merkle_verifying_v3() {
        let input_data: Vec<H256> = gen_merkle_tree_data3!();
        let merkle_tree = MerkleTree::new(&input_data);
        let mut proof;
        
        proof = merkle_tree.proof(0);
        assert!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 0, input_data.len()));

        proof = merkle_tree.proof(5);
        assert!(verify(&merkle_tree.root(), &input_data[5].hash(), &proof, 5, input_data.len()));
    }

    // define a slice of Hashable data of length 5
    macro_rules! gen_merkle_tree_data4 {
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
            ]
        }};
    }

    #[test]
    fn merkle_nodes_v2() {
        // generate a merkle tree starting with 5 leaf nodes
        let input_data: Vec<H256> = gen_merkle_tree_data4!();
        let merkle_tree = MerkleTree::new(&input_data);
        let root = merkle_tree.root();
        let nodes = merkle_tree.nodes;
        
        assert_eq!(
            nodes[7].unwrap(),
            (hex!("b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0")).into()
        );
        assert_eq!(
            nodes[8].unwrap(),
            (hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f")).into()
        );
        assert_eq!(
            nodes[11].unwrap(),
            (hex!("b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0")).into()
        );
        assert_eq!(
            nodes[11].unwrap(),
            nodes[12].unwrap()
        );
        assert_eq!(
            nodes[4].unwrap(),
            (hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into()
        );
        assert_eq!(
            nodes[5].unwrap(),
            nodes[6].unwrap()
        );
        assert_eq!(
            nodes[13], None
        );
        assert_eq!(
            root,
            nodes[0].unwrap(),
        );
        
        // "b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0" is the hash of
        // "0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d"
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
        // "6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920" is the hash of
        // the concatenation of these two hashes "b69..." and "965..."
        // notice that the order of these two matters
    }

    #[test]
    fn merkle_proof_v4() {
        let input_data: Vec<H256> = gen_merkle_tree_data4!();
        let merkle_tree = MerkleTree::new(&input_data);
        let nodes = merkle_tree.nodes.clone();
        
        let proof_1 = merkle_tree.proof(0);
        // data point at index 0 refers to nodes[7]
        // thus proof_1 should be hashes of nodes [8, 4, 2]
        assert_eq!(proof_1.len(), 3);
        assert_eq!(proof_1[0], nodes[8].unwrap());
        assert_eq!(proof_1[1], nodes[4].unwrap());
        assert_eq!(proof_1[2], nodes[2].unwrap());

        let proof_2 = merkle_tree.proof(6);  // invalid index
        assert_eq!(proof_2.len(), 0);
        
        let proof_3 = merkle_tree.proof(5);
        // data point at index 5 refers to nodes[12]
        // thus proof_3 should be hashes of nodes [11, 6, 1]
        assert_eq!(proof_3.len(), 3);
        assert_eq!(proof_3[0], nodes[11].unwrap());
        assert_eq!(proof_3[1], nodes[6].unwrap());
        assert_eq!(proof_3[2], nodes[1].unwrap());

        let proof_4 = merkle_tree.proof(3);
        // data point at index 3 refers to nodes[10]
        // thus proof_4 should be hashes of nodes [9, 3, 2]
        assert_eq!(proof_4.len(), 3);
        assert_eq!(proof_4[0], nodes[9].unwrap());
        assert_eq!(proof_4[1], nodes[3].unwrap());
        assert_eq!(proof_4[2], nodes[2].unwrap());
    }

    #[test]
    fn merkle_verifying_v4() {
        let input_data: Vec<H256> = gen_merkle_tree_data4!();
        let merkle_tree = MerkleTree::new(&input_data);
        let mut proof;
        
        proof = merkle_tree.proof(0);
        assert!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 0, input_data.len()));

        proof = merkle_tree.proof(4);
        assert!(verify(&merkle_tree.root(), &input_data[4].hash(), &proof, 4, input_data.len()));
    }

    // define a slice of Hashable data of length 1
    macro_rules! gen_merkle_tree_data5 {
        () => {{
            vec![(hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into()]
        }};
    }

    #[test]
    fn merkle_nodes_v3() {
        // generate a merkle tree starting with 1 leaf node
        let input_data: Vec<H256> = gen_merkle_tree_data5!();
        let merkle_tree = MerkleTree::new(&input_data);
        let root = merkle_tree.root();
        let nodes = merkle_tree.nodes;
        
        assert_eq!(nodes.len(), 1);
        assert_eq!(
            nodes[0].unwrap(),
            (hex!("b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0")).into()
        );
        assert_eq!(
            root,
            (hex!("b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0")).into()
        );
    }

    #[test]
    fn merkle_nodes_v4() {
        // generate a merkle tree starting with 0 nodes
        let input_data: Vec<H256> = vec![];
        let merkle_tree = MerkleTree::new(&input_data);
        let root = merkle_tree.root();
        let nodes = merkle_tree.nodes;

        assert_eq!(nodes.len(), 0);
        assert_eq!(
            root,
            (hex!("0000000000000000000000000000000000000000000000000000000000000000")).into()
        );
    }

    #[test]
    fn merkle_verifying_v5() {
        // generate a merkle tree starting with 0 nodes
        let input_data: Vec<H256> = vec![];
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);

        let item: H256 = (hex!("0000000000000000000000000000000000000000000000000000000000000000")).into();

        assert_eq!(proof.len(), 0);
        assert_eq!(verify(&merkle_tree.root(), &item, &proof, 0, input_data.len()), false);
    }   
}

// DO NOT CHANGE THIS COMMENT, IT IS FOR AUTOGRADER. AFTER TEST