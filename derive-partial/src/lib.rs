use darling::{FromDeriveInput, FromField, FromMeta, ast};
use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{Ident, Meta, Type, parse_macro_input};

/// Struct-level `#[partial(...)]` options.
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(partial))]
struct PartialOpts {
    ident: Ident,
    vis: syn::Visibility,
    data: ast::Data<(), FieldOpts>,

    /// Name of the generated partial struct: `#[partial(name = CreateUser)]`
    /// Defaults to `Partial{OriginalName}` when omitted.
    #[darling(default)]
    name: Option<Ident>,

    /// Derives to apply: `#[partial(derive(Debug, Deserialize))]`
    #[darling(default)]
    derive: PathList,

    /// Optional module-level documentation for the generated struct.
    #[darling(default)]
    doc: Option<String>,
}

/// Field-level `#[partial(...)]` options.
#[derive(Debug, FromField)]
#[darling(attributes(partial), forward_attrs(doc, serde, cfg))]
struct FieldOpts {
    ident: Option<Ident>,
    vis: syn::Visibility,
    ty: Type,
    attrs: Vec<syn::Attribute>,

    /// Skip this field entirely: `#[partial(skip)]`
    #[darling(default)]
    skip: bool,

    /// Wrap the type in `Option<T>`: `#[partial(option)]`
    #[darling(default)]
    option: bool,

    /// Rename the field: `#[partial(rename = "new_name")]`
    #[darling(default)]
    rename: Option<String>,

    /// Override the type: `#[partial(ty = "NewType")]`
    #[darling(default)]
    ty_override: Option<String>,
}

/// Wrapper for parsing a list of paths from `derive(A, B, C)`.
#[derive(Debug, Default)]
struct PathList(Vec<syn::Path>);

impl FromMeta for PathList {
    fn from_list(items: &[darling::ast::NestedMeta]) -> darling::Result<Self> {
        let mut paths = Vec::new();
        for item in items {
            match item {
                darling::ast::NestedMeta::Meta(Meta::Path(p)) => paths.push(p.clone()),
                other => {
                    return Err(darling::Error::unexpected_type(
                        &other.to_token_stream().to_string(),
                    ));
                }
            }
        }
        Ok(PathList(paths))
    }
}

/// Derive macro that generates a partial version of a struct with a subset of its fields.
///
/// # Struct-level attributes
///
/// - `#[partial(name = PartialName)]` — sets the name of the generated struct.
///   Defaults to `Partial{OriginalName}` when omitted (e.g. `User` → `PartialUser`).
/// - `#[partial(derive(Debug, Serialize, ...))]` — adds `#[derive(...)]` to the generated struct.
/// - `#[partial(doc = "...")]` — sets a custom doc comment. When omitted, a default
///   `Partial version of [OriginalName].` is generated.
///
/// # Field-level attributes
///
/// - `#[partial(skip)]` — omits the field from the generated struct.
/// - `#[partial(rename = "new_name")]` — renames the field in the generated struct.
/// - `#[partial(option)]` — wraps the field type in `Option<T>`.
/// - `#[partial(ty = "NewType")]` — overrides the field type entirely.
///
/// Doc comments (`///`), `#[serde(...)]`, and `#[cfg(...)]` attributes on fields are
/// automatically forwarded to the generated struct.
///
/// # Example
///
/// ```ignore
/// #[derive(Partial)]
/// #[partial(name = CreateUser, derive(Debug, Deserialize))]
/// struct User {
///     #[partial(skip)]
///     id: u64,
///     #[partial(rename = "username")]
///     name: String,
///     #[partial(option)]
///     location: String,
///     email: String,
/// }
///
/// // Generates:
/// // #[derive(Debug, Deserialize)]
/// // struct CreateUser {
/// //     username: String,
/// //     location: Option<String>,
/// //     email: String,
/// // }
/// ```
#[proc_macro_derive(Partial, attributes(partial))]
pub fn derive_partial(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let opts = match PartialOpts::from_derive_input(&input) {
        Ok(o) => o,
        Err(e) => return e.write_errors().into(),
    };

    let partial_name = opts
        .name
        .unwrap_or_else(|| format_ident!("Partial{}", opts.ident));
    let vis = &opts.vis;

    // Build derive attribute
    let derive_attr = if opts.derive.0.is_empty() {
        quote! {}
    } else {
        let derives = &opts.derive.0;
        quote! { #[derive(#(#derives),*)] }
    };

    // Build doc attribute
    let doc_attr = match &opts.doc {
        Some(doc) => quote! { #[doc = #doc] },
        None => {
            let doc = format!("Partial version of [`{}`].", opts.ident);
            quote! { #[doc = #doc] }
        }
    };

    // Process fields
    let fields = opts
        .data
        .take_struct()
        .expect("Partial can only be derived on structs")
        .fields;

    let partial_fields: Vec<_> = fields
        .iter()
        .filter(|f| !f.skip)
        .map(|f| {
            let field_name = match &f.rename {
                Some(new_name) => format_ident!("{}", new_name),
                None => f.ident.clone().expect("named fields required"),
            };
            let field_vis = &f.vis;
            let field_ty = match &f.ty_override {
                Some(ty_str) => {
                    let ty: Type = syn::parse_str(ty_str).expect("invalid type override");
                    ty
                }
                None => f.ty.clone(),
            };

            // Collect non-partial attributes (e.g. doc comments, serde attrs)
            let preserved_attrs: Vec<_> = f
                .attrs
                .iter()
                .filter(|a| !a.path().is_ident("partial"))
                .collect();

            if f.option {
                quote! {
                    #(#preserved_attrs)*
                    #field_vis #field_name: Option<#field_ty>,
                }
            } else {
                quote! {
                    #(#preserved_attrs)*
                    #field_vis #field_name: #field_ty,
                }
            }
        })
        .collect();

    let expanded = quote! {
        #doc_attr
        #derive_attr
        #vis struct #partial_name {
            #(#partial_fields)*
        }
    };

    expanded.into()
}
