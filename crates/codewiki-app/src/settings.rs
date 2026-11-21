//! What the Settings page is made of, apart from how it is drawn: its
//! categories, the fields a user types into, and the news shown beside one.
//! No UI here; `pages.rs` builds a category's page from these and
//! `settings_view.rs` draws it.

use crate::icons::{self, Glyph};

/// A group of settings, with a page of its own. The order is the order of
/// the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Appearance,
    Search,
    Backend,
    Repositories,
    Updates,
    Keyboard,
    Diagnostics,
}

impl Category {
    pub const ALL: [Self; 7] = [
        Self::Appearance,
        Self::Search,
        Self::Backend,
        Self::Repositories,
        Self::Updates,
        Self::Keyboard,
        Self::Diagnostics,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Appearance => "Appearance",
            Self::Search => "Semantic search",
            Self::Backend => "Backend",
            Self::Repositories => "Repositories",
            Self::Updates => "Updates",
            Self::Keyboard => "Keyboard",
            Self::Diagnostics => "Diagnostics",
        }
    }

    pub fn icon(self) -> Glyph {
        match self {
            Self::Appearance => icons::THEME,
            Self::Search => icons::SEARCH,
            Self::Backend => icons::BACKEND,
            Self::Repositories => icons::REPOSITORY,
            Self::Updates => icons::UPDATE,
            Self::Keyboard => icons::COMMANDS,
            Self::Diagnostics => icons::DIAGNOSTICS,
        }
    }

    /// The category's name in a startup argument: `settings:<slug>`.
    pub fn slug(self) -> &'static str {
        match self {
            Self::Appearance => "appearance",
            Self::Search => "search",
            Self::Backend => "backend",
            Self::Repositories => "repositories",
            Self::Updates => "updates",
            Self::Keyboard => "keyboard",
            Self::Diagnostics => "diagnostics",
        }
    }

    /// The category a slug names. An empty or unknown one is the first.
    pub fn from_slug(slug: &str) -> Self {
        let slug = slug.trim().to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|category| category.slug() == slug)
            .unwrap_or(Self::Appearance)
    }
}

/// A setting the user types: each has one text input, which lives as long as
/// the app does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldId {
    OpenAiKey,
    OpenAiBaseUrl,
    OllamaHost,
    LmStudioUrl,
    LmStudioKey,
    LlamaCppUrl,
    LlamaCppKey,
    LemonadeUrl,
    LemonadeKey,
    MistralRsUrl,
    EmbedModel,
    RemoteUrl,
    RemoteToken,
}

impl FieldId {
    pub const ALL: [Self; 13] = [
        Self::OpenAiKey,
        Self::OpenAiBaseUrl,
        Self::OllamaHost,
        Self::LmStudioUrl,
        Self::LmStudioKey,
        Self::LlamaCppUrl,
        Self::LlamaCppKey,
        Self::LemonadeUrl,
        Self::LemonadeKey,
        Self::MistralRsUrl,
        Self::EmbedModel,
        Self::RemoteUrl,
        Self::RemoteToken,
    ];

    /// Where the backend keeps the setting: its section and field in
    /// `GET /v1/settings`. `None` for the two the app keeps itself, the
    /// address and token of a backend on another machine.
    pub fn place(self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::OpenAiKey => Some(("openai", "api_key")),
            Self::OpenAiBaseUrl => Some(("openai", "base_url")),
            Self::OllamaHost => Some(("ollama", "host")),
            Self::LmStudioUrl => Some(("lmstudio", "url")),
            Self::LmStudioKey => Some(("lmstudio", "api_key")),
            Self::LlamaCppUrl => Some(("llamacpp", "url")),
            Self::LlamaCppKey => Some(("llamacpp", "api_key")),
            Self::LemonadeUrl => Some(("lemonade", "url")),
            Self::LemonadeKey => Some(("lemonade", "api_key")),
            Self::MistralRsUrl => Some(("mistralrs", "url")),
            Self::EmbedModel => Some(("embed", "model")),
            Self::RemoteUrl | Self::RemoteToken => None,
        }
    }

    /// Whether what is typed is hidden, and never shown again once saved.
    pub fn secret(self) -> bool {
        matches!(
            self,
            Self::OpenAiKey
                | Self::LmStudioKey
                | Self::LlamaCppKey
                | Self::LemonadeKey
                | Self::RemoteToken
        )
    }

    /// What an empty input shows.
    pub fn placeholder(self) -> &'static str {
        match self {
            Self::OpenAiKey => "Paste the key",
            Self::OpenAiBaseUrl => "https://api.openai.com/v1",
            Self::OllamaHost => "http://127.0.0.1:11434",
            Self::LmStudioUrl | Self::MistralRsUrl => "http://127.0.0.1:1234",
            Self::LlamaCppUrl => "http://127.0.0.1:9931",
            Self::LemonadeUrl => "http://127.0.0.1:13305",
            Self::LmStudioKey | Self::LlamaCppKey | Self::LemonadeKey => {
                "Only if the server asks for one"
            }
            Self::EmbedModel => "The model's name, as the server lists it",
            Self::RemoteUrl => "http://host:7411",
            Self::RemoteToken => "The backend's token",
        }
    }
}

/// News about one setting, shown beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// The change is on its way to the backend.
    Working,
    Saved,
    /// Why the change was refused.
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_categories_are_the_seven_in_their_order() {
        let labels: Vec<&str> = Category::ALL.iter().map(|c| c.label()).collect();
        assert_eq!(
            labels,
            [
                "Appearance",
                "Semantic search",
                "Backend",
                "Repositories",
                "Updates",
                "Keyboard",
                "Diagnostics"
            ]
        );
    }

    #[test]
    fn a_slug_names_one_category() {
        for category in Category::ALL {
            assert_eq!(Category::from_slug(category.slug()), category);
        }
        assert_eq!(Category::from_slug(" Search "), Category::Search);
        // `settings:` alone, a name nobody knows, and the page of the AI
        // model that CodeWiki once had, open the first.
        assert_eq!(Category::from_slug("ai"), Category::Appearance);
        assert_eq!(Category::from_slug(""), Category::Appearance);
        assert_eq!(Category::from_slug("colours"), Category::Appearance);
    }

    #[test]
    fn every_category_has_an_icon_of_its_own() {
        let icons: Vec<Glyph> = Category::ALL.iter().map(|c| c.icon()).collect();
        for (index, icon) in icons.iter().enumerate() {
            assert!(
                !icons[index + 1..].contains(icon),
                "{:?}",
                Category::ALL[index]
            );
        }
    }

    #[test]
    fn a_field_is_kept_by_the_backend_or_by_the_app() {
        let mut places = Vec::new();
        for field in FieldId::ALL {
            assert!(!field.placeholder().is_empty());
            match field.place() {
                Some(place) => {
                    assert!(!places.contains(&place), "{field:?}");
                    places.push(place);
                }
                None => assert!(matches!(field, FieldId::RemoteUrl | FieldId::RemoteToken)),
            }
        }
        // The keys and the token are the secrets.
        let secrets: Vec<FieldId> = FieldId::ALL.into_iter().filter(|f| f.secret()).collect();
        assert_eq!(
            secrets,
            [
                FieldId::OpenAiKey,
                FieldId::LmStudioKey,
                FieldId::LlamaCppKey,
                FieldId::LemonadeKey,
                FieldId::RemoteToken
            ]
        );
        for field in secrets.iter().filter_map(|f| f.place()) {
            assert_eq!(field.1, "api_key");
        }
    }
}
