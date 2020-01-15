mod append;
mod generate;

pub use append::ChainAppend;
pub use generate::ChainGenerate;

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

pub const NGRAM_CNT: usize = 2; // Use a bigram markov chain model

pub type ChainPrefix = [u32; NGRAM_CNT]; // indexes into MarkovChain.words

#[derive(Default, Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Datestamp {
    pub year: i16,
    pub day: u16,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ChainSuffix(u32);

impl ChainSuffix {
    const fn terminal(word_idx: u32) -> Self {
        let word_idx_31 = word_idx & ((1u32 << 31) - 1);
        Self(word_idx_31 | 1u32 << 31)
    }

    const fn nonterminal(word_idx: u32) -> Self {
        let word_idx_31 = word_idx & ((1u32 << 31) - 1);
        Self(word_idx_31)
    }

    const fn word_idx(&self) -> u32 {
        self.0 & ((1u32 << 31) - 1)
    }

    const fn is_terminal(&self) -> bool {
        (self.0 & (1u32 << 31)) > 0
    }
}

impl std::fmt::Debug for ChainSuffix {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.is_terminal() {
            write!(f, "Terminal({})", self.word_idx())
        } else {
            write!(f, "NonTerminal({})", self.word_idx())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChainEntry {
    pub prefix: ChainPrefix,
    pub suffix: ChainSuffix,
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
