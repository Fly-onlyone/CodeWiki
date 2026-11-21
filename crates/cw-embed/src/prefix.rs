//! The text some embedding models expect before a query and before a
//! document. Without it they still answer, but retrieval is worse. A model is
//! known by its name, whichever server serves it.

/// What one family of models wants put in front of a text.
pub struct Rule {
    /// Short name added to a vector's name as `+id`: vectors made with and
    /// without a prefix must never be compared.
    pub id: &'static str,
    /// Every part must occur in the lowercased model name, and the first must
    /// also start the name or follow a character that is not alphanumeric.
    pub parts: &'static [&'static str],
    /// Put in front of what a user searches for.
    pub query: &'static str,
    /// Put in front of each document.
    pub document: &'static str,
}

const BGE_QUERY: &str = "Represent this sentence for searching relevant passages: ";

// Each prefix is the one on the model's card:
// - gemma: https://huggingface.co/unsloth/embeddinggemma-300m (config_sentence_transformers.json,
//   prompts "query" and "document"; Google's card is
//   https://ai.google.dev/gemma/docs/embeddinggemma/model_card)
// - qwen3: https://huggingface.co/Qwen/Qwen3-Embedding-0.6B (the query is
//   `Instruct: {task}\nQuery:{query}`; the task is the card's web-search example, as in the
//   old catalog; documents get nothing)
// - nomic: https://huggingface.co/nomic-ai/nomic-embed-text-v1.5 ("search_query: ",
//   "search_document: ")
// - mxbai: https://huggingface.co/mixedbread-ai/mxbai-embed-large-v1 (query prompt only)
// - bge: https://huggingface.co/BAAI/bge-small-en-v1.5 (v1.5: the query instruction is
//   optional but recommended for retrieval; documents get nothing)
// - e5: https://huggingface.co/intfloat/multilingual-e5-large ("query: " and "passage: ")
/// The known families. The first match wins.
pub const RULES: &[Rule] = &[
    Rule {
        id: "gemma",
        parts: &["embeddinggemma"],
        query: "task: search result | query: ",
        document: "title: none | text: ",
    },
    Rule {
        id: "qwen3",
        parts: &["qwen3-embedding"],
        query: "Instruct: Given a web search query, retrieve relevant passages that answer the query\nQuery:",
        document: "",
    },
    Rule {
        id: "nomic",
        parts: &["nomic-embed-text"],
        query: "search_query: ",
        document: "search_document: ",
    },
    Rule {
        id: "mxbai",
        parts: &["mxbai-embed"],
        query: BGE_QUERY,
        document: "",
    },
    Rule {
        id: "bge",
        parts: &["bge-", "-v1.5"],
        query: BGE_QUERY,
        document: "",
    },
    Rule {
        id: "e5",
        parts: &["e5-"],
        query: "query: ",
        document: "passage: ",
    },
];

/// The rule for a model name as a server gives it, if the model is known.
pub fn rule(model: &str) -> Option<&'static Rule> {
    let name = model.to_lowercase();
    RULES.iter().find(|rule| matches(rule, &name))
}

fn matches(rule: &Rule, name: &str) -> bool {
    let Some((first, rest)) = rule.parts.split_first() else {
        return false;
    };
    let starts_a_word = name.match_indices(first).any(|(at, _)| {
        name[..at]
            .chars()
            .next_back()
            .is_none_or(|before| !before.is_alphanumeric())
    });
    starts_a_word && rest.iter().all(|part| name.contains(part))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_rule_matches_the_names_its_servers_give() {
        let id = |m: &str| rule(m).map(|r| r.id);
        for m in [
            "embeddinggemma",
            "embeddinggemma:300m",
            "google/embeddinggemma-300m",
            "text-embedding-embeddinggemma-300m",
        ] {
            assert_eq!(id(m), Some("gemma"), "{m}");
        }
        for m in [
            "Qwen/Qwen3-Embedding-0.6B",
            "qwen3-embedding:0.6b",
            "text-embedding-qwen3-embedding-0.6b",
        ] {
            assert_eq!(id(m), Some("qwen3"), "{m}");
        }
        for m in [
            "nomic-embed-text",
            "nomic-embed-text-v1-GGUF",
            "text-embedding-nomic-embed-text-v1.5",
        ] {
            assert_eq!(id(m), Some("nomic"), "{m}");
        }
        assert_eq!(id("mixedbread-ai/mxbai-embed-large-v1"), Some("mxbai"));
        assert_eq!(id("BAAI/bge-small-en-v1.5"), Some("bge"));
        assert_eq!(id("intfloat/multilingual-e5-large"), Some("e5"));
        for m in [
            "text-embedding-3-small",
            "bge-m3",
            "granite-embedding",
            "gte-modernbert-base",
        ] {
            assert_eq!(id(m), None, "{m}");
        }
    }

    #[test]
    fn a_part_must_start_a_word() {
        assert!(rule("subge-small-v1.5").is_none());
        assert!(rule("xe5-large").is_none());
    }

    #[test]
    fn the_qwen3_query_keeps_the_instruction_of_the_old_catalog() {
        assert_eq!(
            rule("qwen3-embedding").unwrap().query,
            "Instruct: Given a web search query, retrieve relevant passages that answer the query\nQuery:"
        );
    }
}
