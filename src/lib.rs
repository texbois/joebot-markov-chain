use indexmap::set::IndexSet;
use std::collections::HashMap;
use std::convert::TryInto;
use vkopt_message_parser::reader::{fold_html, EventResult, MessageEvent};

const NGRAM_CNT: usize = 2; // Use a bigram markov chain model

type ChainPrefix = [u32; NGRAM_CNT]; // indexes into MarkovChain.words

#[derive(Debug)]
struct ChainEdge {
    author_idx: u32,
    suffix_word_idx: u32,
}

#[derive(Default, Debug, PartialEq, Eq, Hash)]
struct MessageAuthor {
    full_name: String,
    short_name: String,
}

#[derive(Default, Debug)]
struct MarkovChain {
    words: IndexSet<String>,
    authors: IndexSet<MessageAuthor>,
    nodes: HashMap<ChainPrefix, Vec<ChainEdge>>,
}

#[derive(Default)]
struct ExtractedMessage {
    author: MessageAuthor,
    date: String,
    body: String,
}

impl MarkovChain {
    pub fn build_from_message_dump(input_file: &str) -> Self {
        let mut chain: Self = Default::default();
        let _ = fold_html(
            input_file,
            Default::default(),
            |mut msg: ExtractedMessage, event| match event {
                MessageEvent::FullNameExtracted(full_name) => {
                    msg.author.full_name.push_str(full_name);
                    EventResult::Consumed(msg)
                }
                MessageEvent::ShortNameExtracted(short_name) => {
                    msg.author.short_name.push_str(short_name);
                    EventResult::Consumed(msg)
                }
                MessageEvent::DateExtracted(date) => {
                    msg.date.push_str(date);
                    EventResult::Consumed(msg)
                }
                MessageEvent::BodyExtracted(body) => {
                    msg.body = body;
                    append_message(&mut chain, msg);
                    EventResult::Consumed(Default::default())
                }
                _ => EventResult::Consumed(msg),
            },
        )
        .unwrap();
        chain
    }
}

fn append_message(chain: &mut MarkovChain, message: ExtractedMessage) {
    let author_idx = chain.authors.insert_full(message.author).0 as u32;
    let word_indexes = message
        .body
        .split(&[' ', '\n'][..])
        .map(|word| chain.words.insert_full(word.to_owned()).0 as u32)
        .collect::<Vec<_>>();

    for ngram in word_indexes.windows(NGRAM_CNT + 1) {
        let (prefix_words, suffix) = ngram.split_at(NGRAM_CNT);
        let prefix: ChainPrefix = prefix_words.try_into().unwrap();
        let node = chain.nodes.entry(prefix).or_insert(Vec::new());
        node.push(ChainEdge {
            author_idx,
            suffix_word_idx: suffix[0],
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authors() {
        let chain = MarkovChain::build_from_message_dump("tests/fixtures/messages.html");
        assert_eq!(
            chain.authors.get_index(0),
            Some(&MessageAuthor {
                full_name: "Sota Sota".into(),
                short_name: "sota".into()
            })
        );
        assert_eq!(
            chain.authors.get_index(1),
            Some(&MessageAuthor {
                full_name: "Denko Denko".into(),
                short_name: "denko".into()
            })
        );
        println!("{:#?}", chain);
    }

    #[test]
    fn test_words_order() {
        let chain = MarkovChain::build_from_message_dump("tests/fixtures/messages.html");
        assert_eq!(chain.words.get_index(0), Some(&"Привет".into()));
        assert_eq!(chain.words.get_index(1), Some(&"Denko".into()));
    }

    #[test]
    fn test_no_empty_words() {
        let chain = MarkovChain::build_from_message_dump("tests/fixtures/messages.html");
        let enumerated_words = chain.words.iter().enumerate();
        let empty_words =
            enumerated_words.filter_map(|(i, w)| if w.is_empty() { Some(i) } else { None });

        assert_eq!(vec![0usize; 0], empty_words.collect::<Vec<_>>());
    }
}
