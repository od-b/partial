#![allow(unused)]

use derive_partial::{FromPartial, IntoPartial, NewPartial, Partial};
use partial_traits;
use serde::{Deserialize, Serialize};

/// Explicit name, skip, rename, option
#[derive(Serialize, Deserialize, Partial)]
#[partial(name = CreateUser, derive(Debug, PartialEq, Deserialize))]
struct User {
    #[partial(skip)]
    /// user ID
    id: u64,
    #[partial(rename = "username")]
    /// username
    name: String,
    #[partial(option)]
    location: String,
    email: String,
}

/// Default name (no `name = ...`)
#[derive(Partial)]
#[partial(derive(Debug, PartialEq))]
struct Config {
    host: String,
    port: u16,
}

/// No derives
#[derive(Partial)]
struct Minimal {
    value: i32,
}

/// Type override
#[derive(Partial)]
#[partial(name = UpdateProfile, derive(Debug, PartialEq))]
struct Profile {
    #[partial(skip)]
    id: u64,
    #[partial(ty_override = "Option<String>")]
    bio: String,
    avatar_url: String,
}

/// Combined field attributes (option + rename)
#[derive(Partial)]
#[partial(name = PatchSettings, derive(Debug, PartialEq))]
struct Settings {
    #[partial(option, rename = "dark_mode")]
    theme_is_dark: bool,
    #[partial(option)]
    font_size: u32,
    language: String,
}

/// Serde attribute forwarding
#[derive(Serialize, Deserialize, Partial)]
#[partial(name = ApiRequest, derive(Debug, PartialEq, Deserialize))]
struct InternalRequest {
    #[partial(skip)]
    internal_id: u64,
    #[serde(rename = "req_type")]
    request_type: String,
    payload: String,
}

/// Custom doc
#[derive(Partial)]
#[partial(name = NewArticle, derive(Debug, PartialEq), doc = "Used to create a new article.")]
struct Article {
    #[partial(skip)]
    id: u64,
    title: String,
    body: String,
}

/// Public struct with pub fields
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

/// NewPartial standalone derive
#[derive(Debug, PartialEq, Partial, NewPartial)]
#[partial(name = CreatePost, derive(Debug, PartialEq))]
struct Post {
    #[partial(skip)]
    id: u64,
    title: String,
    body: String,
    #[partial(option)]
    tags: Vec<String>,
}

/// FromPartial + IntoPartial (skip only)
#[derive(Debug, PartialEq, Partial, FromPartial, IntoPartial)]
#[partial(name = NewEmployee, derive(Debug, PartialEq))]
struct Employee {
    #[partial(skip)]
    id: u64,
    name: String,
    department: String,
}

/// All four derives with rename + option + skip
#[derive(Debug, PartialEq, Partial, NewPartial, FromPartial, IntoPartial)]
#[partial(name = CreateOrder, derive(Debug, PartialEq))]
struct Order {
    #[partial(skip)]
    id: u64,
    #[partial(rename = "product")]
    product_name: String,
    quantity: u32,
    #[partial(option)]
    notes: String,
}

/// FromPartial + IntoPartial with no skipped fields
#[derive(Debug, PartialEq, Partial, FromPartial, IntoPartial)]
#[partial(name = PartialPoint, derive(Debug, PartialEq))]
struct Point {
    x: f64,
    y: f64,
}

/// IntoPartial with type override
#[derive(Debug, PartialEq, Partial, IntoPartial)]
#[partial(name = UpdateBio, derive(Debug, PartialEq))]
struct Bio {
    #[partial(ty_override = "Option<String>")]
    text: String,
}

fn main() {}

#[cfg(test)]
mod tests {
    use super::*;
    use partial_traits::{FromPartial, IntoPartial};

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

    #[test]
    fn new_partial_constructor() {
        let p = Post::new_partial(
            "My Post".into(),
            "Content".into(),
            Some(vec!["rust".into(), "macros".into()]),
        );
        assert_eq!(p.title, "My Post");
        assert_eq!(p.body, "Content");
        assert_eq!(p.tags, Some(vec!["rust".into(), "macros".into()]));
    }

    #[test]
    fn new_partial_option_none() {
        let p = Post::new_partial("Untitled".into(), "Empty".into(), None);
        assert_eq!(p.tags, None);
    }

    #[test]
    fn from_partial_with_skipped_field() {
        let partial = NewEmployee {
            name: "Alice".into(),
            department: "Engineering".into(),
        };
        let emp = Employee::from_partial(partial, NewEmployeeMissing { id: 42 });
        assert_eq!(emp.id, 42);
        assert_eq!(emp.name, "Alice");
        assert_eq!(emp.department, "Engineering");
    }

    #[test]
    fn from_partial_with_skip_rename_option() {
        let partial = CreateOrder {
            product: "Widget".into(),
            quantity: 5,
            notes: Some("Rush order".into()),
        };
        // Args: id (skipped) + notes (option-wrapped, original type String)
        let order = Order::from_partial(
            partial,
            CreateOrderMissing {
                id: 1,
                notes: "Noted".into(),
            },
        );
        assert_eq!(order.id, 1);
        assert_eq!(order.product_name, "Widget");
        assert_eq!(order.quantity, 5);
        assert_eq!(order.notes, "Noted");
    }

    #[test]
    fn from_partial_no_extra_args() {
        let partial = PartialPoint { x: 1.0, y: 2.0 };
        let pt = Point::from_partial(partial, PartialPointMissing {});
        assert_eq!(pt.x, 1.0);
        assert_eq!(pt.y, 2.0);
    }

    #[test]
    fn into_partial_drops_skipped_fields() {
        let emp = Employee {
            id: 99,
            name: "Bob".into(),
            department: "Sales".into(),
        };
        let partial: NewEmployee = emp.into_partial();
        assert_eq!(partial.name, "Bob");
        assert_eq!(partial.department, "Sales");
    }

    #[test]
    fn into_partial_renames_and_wraps_option() {
        let order = Order {
            id: 10,
            product_name: "Gadget".into(),
            quantity: 3,
            notes: "Handle with care".into(),
        };
        let partial: CreateOrder = order.into_partial();
        assert_eq!(partial.product, "Gadget");
        assert_eq!(partial.quantity, 3);
        assert_eq!(partial.notes, Some("Handle with care".into()));
    }

    #[test]
    fn into_partial_no_skipped_fields() {
        let pt = Point { x: 3.0, y: 4.0 };
        let partial: PartialPoint = pt.into_partial();
        assert_eq!(partial.x, 3.0);
        assert_eq!(partial.y, 4.0);
    }

    #[test]
    fn into_partial_type_override() {
        let b = Bio {
            text: "Hello".into(),
        };
        let partial: UpdateBio = b.into_partial();
        // String → Option<String> via .into()
        assert_eq!(partial.text, Some("Hello".into()));
    }

    #[test]
    fn roundtrip_from_into() {
        let emp = Employee {
            id: 7,
            name: "Carol".into(),
            department: "HR".into(),
        };
        let partial: NewEmployee = emp.into_partial();
        let restored = Employee::from_partial(partial, NewEmployeeMissing { id: 7 });
        assert_eq!(
            restored,
            Employee {
                id: 7,
                name: "Carol".into(),
                department: "HR".into(),
            }
        );
    }

    #[test]
    fn roundtrip_no_skipped() {
        let pt = Point { x: 1.5, y: 2.5 };
        let partial: PartialPoint = pt.into_partial();
        let restored = Point::from_partial(partial, PartialPointMissing {});
        assert_eq!(restored, Point { x: 1.5, y: 2.5 });
    }

    #[test]
    fn new_partial_then_from_partial() {
        let partial = Order::new_partial("Wrench".into(), 10, None);
        let order = Order::from_partial(
            partial,
            CreateOrderMissing {
                id: 42,
                notes: "No notes".into(),
            },
        );
        assert_eq!(order.id, 42);
        assert_eq!(order.product_name, "Wrench");
        assert_eq!(order.quantity, 10);
        assert_eq!(order.notes, "No notes");
    }
}
