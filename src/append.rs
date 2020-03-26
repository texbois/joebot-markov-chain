use crate::{ChainEntry, ChainPrefix, ChainSuffix, Datestamp, MarkovChain, TextSource, NGRAM_CNT};
use chrono::{Datelike, NaiveDateTime};
use indexmap::IndexSet;
use std::convert::TryInto;
use std::iter::FromIterator;

use vkopt_message_parser::reader::{fold_html, EventResult, MessageEvent};

pub trait ChainAppend {
    fn append_text(&mut self, input_file: &str, source_names: Vec<String>, datestamp: Datestamp);

    fn append_message_dump(&mut self, input_file: &str);
}

#[derive(Default)]
struct ExtractedMessage {
    names: Vec<String>,
    datestamp: Datestamp,
    body: String,
}

impl ChainAppend for MarkovChain {
    fn append_text(&mut self, input_file: &str, source_names: Vec<String>, datestamp: Datestamp) {
        let text = std::fs::read_to_string(input_file).unwrap();
        let source = source_by_names(&mut self.sources, source_names);
        push_text_entries(&text, datestamp, &mut source.entries, &mut self.words, true);
    }

    fn append_message_dump(&mut self, input_file: &str) {
        let last_msg = fold_html(
            input_file,
            Default::default(),
            |mut msg: ExtractedMessage, event| match event {
                MessageEvent::Start(0) => {
                    if !msg.body.is_empty() {
                        append_message(self, msg);
                    }
                    EventResult::Consumed(Default::default())
                }
                MessageEvent::FullNameExtracted(full_name) => {
                    msg.names.push(full_name.to_owned());
                    EventResult::Consumed(msg)
                }
                MessageEvent::ShortNameExtracted(short_name) => {
                    msg.names.push(short_name.to_owned());
                    EventResult::Consumed(msg)
                }
                MessageEvent::DateExtracted(date) => {
                    let timestamp =
                        NaiveDateTime::parse_from_str(date, "%Y.%m.%d %H:%M:%S").unwrap();
                    msg.datestamp = Datestamp {
                        year: timestamp.year() as i16,
                        day: timestamp.ordinal() as u16,
                    };
                    EventResult::Consumed(msg)
                }
                MessageEvent::BodyPartExtracted(body) => {
                    msg.body.push_str(body);
                    EventResult::Consumed(msg)
                }
                _ => EventResult::Consumed(msg),
            },
        )
        .unwrap();
        if !last_msg.body.is_empty() {
            append_message(self, last_msg);
        }
    }
}

fn source_by_names(sources: &mut Vec<TextSource>, names: Vec<String>) -> &mut TextSource {
    let idx = sources
        .iter()
        .position(|s| names.iter().any(|n| s.names.contains(n)))
        .unwrap_or_else(|| {
            let new_source = TextSource {
                names: IndexSet::from_iter(names.into_iter()),
                ..Default::default()
            };
            sources.push(new_source);
            sources.len() - 1
        });
    sources.get_mut(idx).unwrap()
}

fn append_message(chain: &mut MarkovChain, message: ExtractedMessage) {
    let source = source_by_names(&mut chain.sources, message.names);
    push_text_entries(
        &message.body,
        message.datestamp,
        &mut source.entries,
        &mut chain.words,
        false,
    );
}

fn push_text_entries(
    text: &str,
    datestamp: Datestamp,
    entries: &mut Vec<ChainEntry>,
    words: &mut IndexSet<String>,
    treat_ending_punctuation_as_terminal: bool,
) {
    let word_indexes = text
        .split(&[' ', '\n'][..])
        .filter(|word| !word.is_empty())
        .map(|word| words.insert_full(word.to_owned()).0 as u32)
        .collect::<Vec<_>>();

    if word_indexes.len() < NGRAM_CNT + 1 {
        return;
    }

    let last_ngram = &word_indexes[word_indexes.len() - (NGRAM_CNT + 1)..word_indexes.len()];

    let mut starting = true;
    for ngram in word_indexes.windows(NGRAM_CNT + 1) {
        let (prefix_words, suffix) = ngram.split_at(NGRAM_CNT);
        let terminal = if treat_ending_punctuation_as_terminal {
            words
                .get_index(suffix[0] as usize)
                .unwrap()
                .ends_with(|c| c == '.' || c == '?' || c == '!')
        } else {
            ngram == last_ngram
        };
        entries.push(ChainEntry {
            prefix: if starting {
                ChainPrefix::starting(prefix_words.try_into().unwrap())
            } else {
                ChainPrefix::nonstarting(prefix_words.try_into().unwrap())
            },
            suffix: if terminal {
                ChainSuffix::terminal(suffix[0])
            } else {
                ChainSuffix::nonterminal(suffix[0])
            },
            datestamp,
        });
        starting = terminal;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::indexset;

    #[test]
    fn test_authors() {
        let mut chain = MarkovChain::new();
        chain.append_message_dump("tests/fixtures/messages.html");
        assert_eq!(
            chain.sources[0].names,
            indexset!["Sota Sota".into(), "sota".into()]
        );
        assert_eq!(
            chain.sources[1].names,
            indexset!["Denko Denko".into(), "denko".into()]
        );
    }

    #[test]
    fn test_word_nodes() {
        let mut chain = MarkovChain::new();
        chain.append_message_dump("tests/fixtures/messages.html");
        assert_eq!(chain.words.get_index(0), Some(&"Привет".into()));
        assert_eq!(chain.words.get_index(1), Some(&"Denko".into()));
        assert_eq!(chain.words.get_index(2), Some(&"Пью".into()));

        assert_eq!(
            chain.sources[0].entries[0],
            ChainEntry {
                prefix: ChainPrefix::starting([0, 1]),
                suffix: ChainSuffix::nonterminal(2),
                datestamp: Datestamp {
                    year: 2018,
                    day: 21
                }
            }
        );
        assert_eq!(
            chain.sources[0].entries.last(),
            Some(&ChainEntry {
                prefix: ChainPrefix::nonstarting([3, 4]),
                suffix: ChainSuffix::terminal(5),
                datestamp: Datestamp {
                    year: 2018,
                    day: 21
                }
            })
        );
    }

    #[test]
    fn test_no_empty_words() {
        let mut chain = MarkovChain::new();
        chain.append_message_dump("tests/fixtures/messages.html");
        let enumerated_words = chain.words.iter().enumerate();
        let empty_words =
            enumerated_words.filter_map(|(i, w)| if w.is_empty() { Some(i) } else { None });
        assert_eq!(empty_words.collect::<Vec<_>>(), vec![0usize; 0]);
    }

    #[test]
    fn test_text() {
        let mut chain = MarkovChain::new();
        chain.append_text(
            "tests/fixtures/text",
            vec!["angus".into(), "sol onset".into()],
            Datestamp { year: 0, day: 0 },
        );
        assert_eq!(
            chain.words,
            indexset![
                "useless".into(),
                "unreliable".into(),
                "heavily".into(),
                "distorted".into(),
                "probe.".into(),
                "flashing".into(),
                "red.".into()
            ]
        );
        assert_eq!(
            chain.sources[0].names,
            indexset!["angus".into(), "sol onset".into()]
        );
        assert_eq!(
            chain.sources[0].entries,
            vec![
                ChainEntry {
                    prefix: ChainPrefix::starting([0, 1]),
                    suffix: ChainSuffix::nonterminal(2),
                    datestamp: Datestamp { year: 0, day: 0 }
                },
                ChainEntry {
                    prefix: ChainPrefix::nonstarting([1, 2]),
                    suffix: ChainSuffix::nonterminal(3),
                    datestamp: Datestamp { year: 0, day: 0 }
                },
                ChainEntry {
                    prefix: ChainPrefix::nonstarting([2, 3]),
                    suffix: ChainSuffix::terminal(4),
                    datestamp: Datestamp { year: 0, day: 0 }
                },
                ChainEntry {
                    prefix: ChainPrefix::starting([3, 4]),
                    suffix: ChainSuffix::nonterminal(5),
                    datestamp: Datestamp { year: 0, day: 0 }
                },
                ChainEntry {
                    prefix: ChainPrefix::nonstarting([4, 5]),
                    suffix: ChainSuffix::terminal(6),
                    datestamp: Datestamp { year: 0, day: 0 }
                }
            ]
        );
    }
}
