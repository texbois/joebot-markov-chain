pub mod chain;

use chain::{ChainEdge, ChainPrefix, MarkovChain};
use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};

const MAX_TRIES: usize = 20;

pub fn generate<'a>(author_name: &'a str, chain: &MarkovChain, min_words: usize) -> String {
    let author_idx = chain
        .authors
        .iter()
        .enumerate()
        .find_map(|(i, a)| {
            if a.short_name == author_name {
                Some(i)
            } else {
                None
            }
        })
        .unwrap() as u32;

    let author_edges = chain
        .nodes
        .iter()
        .flat_map(|(prefixes, edges)| {
            edges
                .iter()
                .filter(|e| e.author_idx == author_idx)
                .map(|e| (prefixes.clone(), e.clone()))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let mut rng = SmallRng::from_entropy();
    generate_sequence(&mut rng, &author_edges, min_words)
        .into_iter()
        .filter_map(|word_idx| chain.words.get_index(word_idx as usize).map(|w| w.as_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn generate_sequence(
    rng: &mut SmallRng,
    edges: &Vec<(ChainPrefix, ChainEdge)>,
    min_words: usize,
) -> Vec<u32> {
    fn advance_sequence(
        current_edges: &Vec<(ChainPrefix, ChainEdge)>,
        all_edges: &Vec<(ChainPrefix, ChainEdge)>,
        rng: &mut SmallRng,
        generated: &mut Vec<u32>,
        min_words: usize,
    ) {
        let (prefixes, edge) = current_edges.choose(rng).unwrap();
        for word in prefixes.iter() {
            generated.push(*word);
        }
        generated.push(edge.suffix_word_idx);

        if generated.len() < min_words {
            let next_edges = all_edges
                .iter()
                .filter(|(p, _)| p.contains(&edge.suffix_word_idx))
                .cloned()
                .collect::<Vec<_>>();

            if !next_edges.is_empty() {
                advance_sequence(&next_edges, all_edges, rng, generated, min_words);
            }
        }
    };

    let mut tries = 0;
    let mut generated: Vec<u32> = Vec::with_capacity(min_words as usize);
    while generated.len() < min_words && tries < MAX_TRIES {
        generated.clear();
        advance_sequence(edges, edges, rng, &mut generated, min_words);
        tries += 1;
    }
    generated
}

#[cfg(test)]
mod tests {
    use super::*;
    use chain::MessageAuthor;

    #[test]
    fn test_determined_generation() {
        let mut chain: MarkovChain = Default::default();
        chain.words.insert("депрессия".into());
        chain.words.insert("с".into());
        chain.words.insert("собаками".into());

        chain.authors.insert(MessageAuthor { short_name: "дана".into(), full_name: "".into() });
        chain.authors.insert(MessageAuthor { short_name: "джилл".into(), full_name: "".into() });

        chain.nodes.insert([0, 1], vec![ChainEdge { author_idx: 0, suffix_word_idx: 2, timestamp: 0 }]);

        let generated = generate("дана", &chain, 3);
        assert_eq!(generated, "депрессия с собаками");
    }
}
