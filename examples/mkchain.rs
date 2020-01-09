use joebot_markov_chain::{ChainAppend, Datestamp, MarkovChain};
use serde_json;
use std::fs::File;

fn main() {
    let mut chain = MarkovChain::new();
    chain.append_message_dump("tests/fixtures/messages.html");
    chain.append_text(
        "tests/fixtures/text",
        vec!["text".into()],
        Datestamp { year: 2020, day: 0 },
    );
    serde_json::to_writer(&File::create("chain.json").unwrap(), &chain).unwrap();
}
