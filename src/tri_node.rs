use std::collections::HashMap;

#[derive(Default)]
struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_end_of_key: bool,
}

impl TrieNode {
    fn new() -> Self {
        TrieNode {
            children: HashMap::new(),
            is_end_of_key: false,
        }
    }
}

#[derive(Default)]
pub struct Trie {
    root: TrieNode,
}

impl Trie {
    pub fn insert(&mut self, key: &str) {
        let mut node = &mut self.root;
        for ch in key.chars() {
            node = node.children.entry(ch).or_default();
        }
        node.is_end_of_key = true;
    }

    /// returns the longest prefix that matches the input string
    pub fn search(&self, s: &str) -> Option<String> {
        let mut node = &self.root;

        let mut matched_prefix = String::new();
        for ch in s.chars() {
            if let Some(next_node) = node.children.get(&ch) {
                matched_prefix.push(ch);
                node = next_node;
                if node.is_end_of_key {
                    return Some(matched_prefix);
                }
            } else {
                break;
            }
        }
        None
    }
}

#[test]
fn test_trie_node() {
    let keys = ["foo", "bar"];

    let mut trie = Trie::default();
    for key in keys {
        trie.insert(key);
    }

    let test_str = "foobar";
    let Some(matched_part) = trie.search(test_str) else {
        panic!("No match found");
    };
    assert_eq!(matched_part, "foo");
}

#[test]
fn test_trie_node_fail_case() {
    let keys = ["foo", "bar", "foos"];

    let mut trie = Trie::default();
    for key in keys {
        trie.insert(key);
    }

    let test_str = "fobar";
    let None = trie.search(test_str) else {
        panic!("No match found");
    };
}
