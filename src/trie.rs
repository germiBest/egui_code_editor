use prefix_tree::Trie;

use crate::Syntax;

pub fn trie_from_syntax(syntax: &Syntax) -> Trie {
    let mut trie = Trie::default();

    syntax.keywords.iter().for_each(|word| trie.push(word));
    syntax.types.iter().for_each(|word| trie.push(word));
    syntax.special.iter().for_each(|word| trie.push(word));

    trie
}
