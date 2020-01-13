use joebot_markov_chain::{ChainAppend, Datestamp, MarkovChain};
use bincode;
use std::fs::File;

fn main() {
    let mut chain = MarkovChain::new();
    chain.append_message_dump("messages.html");
    chain.append_text(
        "tests/fixtures/text",
        vec!["text".into()],
        Datestamp { year: 2020, day: 0 },
    );
    println!(
        "Chain entries mem allocation: {} bytes",
        chain.sources.iter().map(|s| s.entries.capacity()).sum::<usize>()
            * std::mem::size_of::<joebot_markov_chain::ChainEntry>()
    );
    bincode::serialize_into(&File::create("chain.bin").unwrap(), &chain).unwrap();
}
