mod append;
mod generate;

pub use append::ChainAppend;
pub use generate::ChainGenerate;

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

pub const NGRAM_CNT: usize = 2; // Use a bigram markov chain model

pub type ChainPrefix = [u32; NGRAM_CNT]; // indexes into MarkovChain.words

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Datestamp {
    pub year: i16,
    pub day: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChainEntry {
    pub prefix: ChainPrefix,
    pub suffix_word_idx: u32,
    pub datestamp: Datestamp,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct TextSource {
    pub names: IndexSet<String>,
    pub entries: Vec<ChainEntry>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct MarkovChain {
    pub words: IndexSet<String>,
    pub sources: Vec<TextSource>,
}

impl MarkovChain {
    pub fn new() -> Self {
        Default::default()
    }
}
