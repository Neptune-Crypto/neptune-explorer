use neptune_cash::api::export::AdditionRecord;
use neptune_cash::api::export::Digest;
use neptune_cash::api::export::TransparentInput;
use neptune_cash::api::export::Utxo;
use neptune_cash::api::export::UtxoTriple;

#[derive(Debug, Clone)]
pub struct TransparentUtxoTuple {
    utxo: Utxo,
    sender_randomness: Digest,
    receiver_preimage: Option<Digest>,
    receiver_digest: Digest,
    aocl_leaf_index: Option<u64>,
    spent_in_block: Vec<Digest>,
    confirmed_in_block: Option<Digest>,
    addition_record: AdditionRecord,
}

impl TransparentUtxoTuple {
    pub fn new_from_transparent_input(
        transparent_input: &TransparentInput,
        spent_in_block: Digest,
    ) -> Self {
        Self {
            utxo: transparent_input.utxo.clone(),
            sender_randomness: transparent_input.sender_randomness,
            receiver_preimage: Some(transparent_input.receiver_preimage),
            receiver_digest: transparent_input.receiver_preimage.hash(),
            aocl_leaf_index: Some(transparent_input.aocl_leaf_index),
            spent_in_block: vec![spent_in_block],
            confirmed_in_block: None,
            addition_record: transparent_input.addition_record(),
        }
    }

    pub fn upgrade_with_transparent_output(&mut self, confirmed_in_block: Digest) {
        self.set_confirmed_in_block(confirmed_in_block);
    }

    pub fn new_from_transparent_output(
        utxo_triple: &UtxoTriple,
        aocl_leaf_index: Option<u64>,
        confirmed_in_block: Digest,
    ) -> Self {
        Self {
            utxo: utxo_triple.utxo.clone(),
            sender_randomness: utxo_triple.sender_randomness,
            receiver_preimage: None,
            receiver_digest: utxo_triple.receiver_digest,
            aocl_leaf_index,
            spent_in_block: vec![],
            confirmed_in_block: Some(confirmed_in_block),
            addition_record: utxo_triple.addition_record(),
        }
    }

    pub fn upgrade_with_transparent_input(
        &mut self,
        transparent_input: &TransparentInput,
        spent_in_block: Digest,
    ) {
        self.set_receiver_preimage(transparent_input.receiver_preimage);
        self.set_spent_in_block(spent_in_block);
    }

    pub fn set_receiver_preimage(&mut self, receiver_preimage: Digest) {
        if self.receiver_preimage.is_none() && receiver_preimage.hash() == self.receiver_digest {
            self.receiver_preimage = Some(receiver_preimage);
        }
    }

    pub fn set_spent_in_block(&mut self, block_digest: Digest) {
        if !self.spent_in_block.contains(&block_digest) {
            self.spent_in_block.push(block_digest);
        }
    }

    pub fn set_confirmed_in_block(&mut self, block_digest: Digest) {
        if self.confirmed_in_block.is_none() {
            self.confirmed_in_block = Some(block_digest);
        }
    }

    pub fn utxo(&self) -> Utxo {
        self.utxo.clone()
    }

    pub fn sender_randomness(&self) -> Digest {
        self.sender_randomness
    }

    pub fn receiver_preimage(&self) -> Option<Digest> {
        self.receiver_preimage
    }

    pub fn receiver_digest(&self) -> Digest {
        if let Some(receiver_preimage) = self.receiver_preimage {
            receiver_preimage.hash()
        } else {
            self.receiver_digest
        }
    }

    pub fn aocl_leaf_index(&self) -> Option<u64> {
        self.aocl_leaf_index
    }

    pub fn spent_in_block(&self) -> Vec<Digest> {
        self.spent_in_block.clone()
    }

    pub fn confirmed_in_block(&self) -> Option<Digest> {
        self.confirmed_in_block
    }

    pub fn addition_record(&self) -> AdditionRecord {
        self.addition_record
    }
}
