#![allow(unused)]

use composite_derive::{FromComposite, IntoComposite, NewComposite, Composite};
use composite_traits::*;
use serde::{Deserialize, Serialize};

/// Explicit name, skip, rename, option
#[derive(Serialize, Deserialize, Composite)]
#[composite(name = CreateUser, derive(Debug, PartialEq, Deserialize))]
struct User {
    #[composite(skip)]
    /// user ID
    id: u64,
    #[composite(rename = "username")]
    /// username
    name: String,
    #[composite(option)]
    location: String,
    email: String,
}

/// Default name (no `name = ...`)
#[derive(Composite)]
#[composite(derive(Debug, PartialEq))]
struct Config {
    host: String,
    port: u16,
}

/// No derives
#[derive(Composite)]
struct Minimal {
    value: i32,
}

/// Type override
#[derive(Composite)]
#[composite(name = UpdateProfile, derive(Debug, PartialEq))]
struct Profile {
    #[composite(skip)]
    id: u64,
    #[composite(ty_override = "Option<String>")]
    bio: String,
    avatar_url: String,
}

/// Combined field attributes (option + rename)
#[derive(Composite)]
#[composite(name = PatchSettings, derive(Debug, PartialEq))]
struct Settings {
    #[composite(option, rename = "dark_mode")]
    theme_is_dark: bool,
    #[composite(option)]
    font_size: u32,
    language: String,
}

/// Serde attribute forwarding
#[derive(Serialize, Deserialize, Composite)]
#[composite(name = ApiRequest, derive(Debug, PartialEq, Deserialize))]
struct InternalRequest {
    #[composite(skip)]
    internal_id: u64,
    #[serde(rename = "req_type")]
    request_type: String,
    payload: String,
}

/// Custom doc
#[derive(Composite)]
#[composite(name = NewArticle, derive(Debug, PartialEq), doc = "Used to create a new article.")]
struct Article {
    #[composite(skip)]
    id: u64,
    title: String,
    body: String,
}

/// Public struct with pub fields
pub mod models {
    use super::Composite;

    #[derive(Composite)]
    #[composite(name = CreateItem, derive(Debug, PartialEq))]
    pub struct Item {
        #[composite(skip)]
        pub id: u64,
        pub name: String,
        pub price: f64,
    }
}

/// NewComposite standalone derive
#[derive(Debug, PartialEq, Composite, NewComposite)]
#[composite(name = CreatePost, derive(Debug, PartialEq))]
struct Post {
    #[composite(skip)]
    id: u64,
    title: String,
    body: String,
    #[composite(option)]
    tags: Vec<String>,
}

/// FromComposite + IntoComposite (skip only)
#[derive(Debug, PartialEq, Composite, FromComposite, IntoComposite)]
#[composite(name = NewEmployee, derive(Debug, PartialEq))]
struct Employee {
    #[composite(skip)]
    id: u64,
    name: String,
    department: String,
}

/// All four derives with rename + option + skip
#[derive(Debug, PartialEq, Composite, NewComposite, FromComposite, IntoComposite)]
#[composite(name = CreateOrder, derive(Debug, PartialEq))]
struct Order {
    #[composite(skip)]
    id: u64,
    #[composite(rename = "product")]
    product_name: String,
    quantity: u32,
    #[composite(option)]
    notes: String,
}

/// FromComposite + IntoComposite with no skipped fields
#[derive(Debug, PartialEq, Composite, FromComposite, IntoComposite)]
#[composite(name = CompositePoint, derive(Debug, PartialEq))]
struct Point {
    x: f64,
    y: f64,
}

/// IntoComposite with type override
#[derive(Debug, PartialEq, Composite, IntoComposite)]
#[composite(name = UpdateBio, derive(Debug, PartialEq))]
struct Bio {
    #[composite(ty_override = "Option<String>")]
    text: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use composite_traits::{FromComposite, IntoComposite};

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
        let c = CompositeConfig {
            host: "localhost".into(),
            port: 8080,
        };
        assert_eq!(c.host, "localhost");
        assert_eq!(c.port, 8080);
    }

    #[test]
    fn no_derives() {
        let m = CompositeMinimal { value: 42 };
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
    fn new_composite_constructor() {
        let p = Post::new_composite(
            "My Post".into(),
            "Content".into(),
            Some(vec!["rust".into(), "macros".into()]),
        );
        assert_eq!(p.title, "My Post");
        assert_eq!(p.body, "Content");
        assert_eq!(p.tags, Some(vec!["rust".into(), "macros".into()]));
    }

    #[test]
    fn new_composite_option_none() {
        let p = Post::new_composite("Untitled".into(), "Empty".into(), None);
        assert_eq!(p.tags, None);
    }

    #[test]
    fn from_composite_with_skipped_field() {
        let composite = NewEmployee {
            name: "Alice".into(),
            department: "Engineering".into(),
        };
        let emp = Employee::from_composite(composite, NewEmployeeMissing { id: 42 });
        assert_eq!(emp.id, 42);
        assert_eq!(emp.name, "Alice");
        assert_eq!(emp.department, "Engineering");
    }

    #[test]
    fn from_composite_with_skip_rename_option() {
        let composite = CreateOrder {
            product: "Widget".into(),
            quantity: 5,
            notes: Some("Rush order".into()),
        };
        // Args: id (skipped) + notes (option-wrapped, original type String)
        let order = Order::from_composite(
            composite,
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
    fn from_composite_no_extra_args() {
        let composite = CompositePoint { x: 1.0, y: 2.0 };
        let pt = Point::from_composite(composite, CompositePointMissing {});
        assert_eq!(pt.x, 1.0);
        assert_eq!(pt.y, 2.0);
    }

    #[test]
    fn into_composite_drops_skipped_fields() {
        let emp = Employee {
            id: 99,
            name: "Bob".into(),
            department: "Sales".into(),
        };
        let composite: NewEmployee = emp.into_composite();
        assert_eq!(composite.name, "Bob");
        assert_eq!(composite.department, "Sales");
    }

    #[test]
    fn into_composite_renames_and_wraps_option() {
        let order = Order {
            id: 10,
            product_name: "Gadget".into(),
            quantity: 3,
            notes: "Handle with care".into(),
        };
        let composite: CreateOrder = order.into_composite();
        assert_eq!(composite.product, "Gadget");
        assert_eq!(composite.quantity, 3);
        assert_eq!(composite.notes, Some("Handle with care".into()));
    }

    #[test]
    fn into_composite_no_skipped_fields() {
        let pt = Point { x: 3.0, y: 4.0 };
        let composite: CompositePoint = pt.into_composite();
        assert_eq!(composite.x, 3.0);
        assert_eq!(composite.y, 4.0);
    }

    #[test]
    fn into_composite_type_override() {
        let b = Bio {
            text: "Hello".into(),
        };
        let composite: UpdateBio = b.into_composite();
        // String -> Option<String> via .into()
        assert_eq!(composite.text, Some("Hello".into()));
    }

    #[test]
    fn roundtrip_from_into() {
        let emp = Employee {
            id: 7,
            name: "Carol".into(),
            department: "HR".into(),
        };
        let composite: NewEmployee = emp.into_composite();
        let restored = Employee::from_composite(composite, NewEmployeeMissing { id: 7 });
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
        let composite: CompositePoint = pt.into_composite();
        let restored = Point::from_composite(composite, CompositePointMissing {});
        assert_eq!(restored, Point { x: 1.5, y: 2.5 });
    }

    #[test]
    fn new_composite_then_from_composite() {
        let composite = Order::new_composite("Wrench".into(), 10, None);
        let order = Order::from_composite(
            composite,
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
