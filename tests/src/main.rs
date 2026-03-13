#![allow(unused)]

use derive_partial::Partial;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Partial)]
#[partial(name = CreateUser, derive(Debug, PartialEq, Deserialize))]
struct User {
    /// user ID
    #[partial(skip)]
    id: u64,
    /// username
    #[partial(rename = "username")]
    name: String,
    #[partial(option)]
    location: String,
    email: String,
}

#[derive(Partial)]
#[partial(derive(Debug, PartialEq))]
struct Config {
    host: String,
    port: u16,
}

#[derive(Partial)]
struct Minimal {
    value: i32,
}

#[derive(Partial)]
#[partial(name = UpdateProfile, derive(Debug, PartialEq))]
struct Profile {
    #[partial(skip)]
    id: u64,
    #[partial(ty_override = "Option<String>")]
    bio: String,
    avatar_url: String,
}

#[derive(Partial)]
#[partial(name = PatchSettings, derive(Debug, PartialEq))]
struct Settings {
    #[partial(option, rename = "dark_mode")]
    theme_is_dark: bool,
    #[partial(option)]
    font_size: u32,
    language: String,
}

#[derive(Serialize, Deserialize, Partial)]
#[partial(name = ApiRequest, derive(Debug, PartialEq, Deserialize))]
struct InternalRequest {
    #[partial(skip)]
    internal_id: u64,
    #[serde(rename = "req_type")]
    request_type: String,
    payload: String,
}

#[derive(Partial)]
#[partial(name = NewArticle, derive(Debug, PartialEq), doc = "Used to create a new article.")]
struct Article {
    #[partial(skip)]
    id: u64,
    title: String,
    body: String,
}

pub mod models {
    use derive_partial::Partial;

    #[derive(Partial)]
    #[partial(name = CreateItem, derive(Debug, PartialEq))]
    pub struct Item {
        #[partial(skip)]
        pub id: u64,
        pub name: String,
        pub price: f64,
    }
}

fn main() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_name_with_skip_rename_option() {
        let u = CreateUser {
            username: "alice".into(),
            location: Some("NYC".into()),
            email: "alice@example.com".into(),
        };
        assert_eq!(u.username, "alice");
        assert_eq!(u.location, Some("NYC".into()));
        assert_eq!(u.email, "alice@example.com");
    }

    #[test]
    fn option_field_accepts_none() {
        let u = CreateUser {
            username: "bob".into(),
            location: None,
            email: "bob@example.com".into(),
        };
        assert_eq!(u.location, None);
    }

    #[test]
    fn default_name() {
        let c = PartialConfig {
            host: "localhost".into(),
            port: 8080,
        };
        assert_eq!(c.host, "localhost");
        assert_eq!(c.port, 8080);
    }

    #[test]
    fn no_derives() {
        let m = PartialMinimal { value: 42 };
        assert_eq!(m.value, 42);
    }

    #[test]
    fn type_override() {
        let p = UpdateProfile {
            bio: None,
            avatar_url: "https://example.com/avatar.png".into(),
        };
        assert_eq!(p.bio, None);

        let p2 = UpdateProfile {
            bio: Some("Hello world".into()),
            avatar_url: "https://example.com/avatar.png".into(),
        };
        assert_eq!(p2.bio, Some("Hello world".into()));
    }

    #[test]
    fn combined_option_and_rename() {
        let s = PatchSettings {
            dark_mode: Some(true),
            font_size: None,
            language: "en".into(),
        };
        assert_eq!(s.dark_mode, Some(true));
        assert_eq!(s.font_size, None);
        assert_eq!(s.language, "en");
    }

    #[test]
    fn serde_attribute_forwarding() {
        let json = r#"{"req_type": "GET", "payload": "data"}"#;
        let req: ApiRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.request_type, "GET");
        assert_eq!(req.payload, "data");
    }

    #[test]
    fn custom_doc() {
        let a = NewArticle {
            title: "Hello".into(),
            body: "World".into(),
        };
        assert_eq!(a.title, "Hello");
        assert_eq!(a.body, "World");
    }

    #[test]
    fn visibility_across_modules() {
        let item = models::CreateItem {
            name: "Widget".into(),
            price: 9.99,
        };
        assert_eq!(item.name, "Widget");
        assert!((item.price - 9.99).abs() < f64::EPSILON);
    }
}
