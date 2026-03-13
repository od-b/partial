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

impl FieldOpts {
    /// The name this field has in the partial struct.
    fn partial_name(&self) -> Ident {
        match &self.rename {
            Some(new_name) => format_ident!("{}", new_name),
            None => self.ident.clone().expect("named fields required"),
        }
    }

    /// The original field name in the source struct.
    fn original_name(&self) -> Ident {
        self.ident.clone().expect("named fields required")
    }

    /// The type this field has in the partial struct.
    fn partial_type(&self) -> Type {
        let base = match &self.ty_override {
            Some(ty_str) => syn::parse_str(ty_str).expect("invalid type override"),
            None => self.ty.clone(),
        };
        if self.option {
            syn::parse_quote! { Option<#base> }
        } else {
            base
        }
    }

    /// Whether this field can be directly assigned from partial to original
    /// (i.e. no type transformation was applied).
    fn is_direct(&self) -> bool {
        !self.option && self.ty_override.is_none()
    }

    /// Whether this field requires extra arguments (skipped, option-wrapped, or type-overridden).
    fn needs_extra_args(&self) -> bool {
        self.skip || !self.is_direct()
    }

    /// Generate the token stream for this field's assignment in `into_partial`.
    fn into_partial_assignment(&self) -> proc_macro2::TokenStream {
        let orig = self.original_name();
        let partial = self.partial_name();

        if self.option && self.ty_override.is_some() {
            quote! { #partial: Some(self.#orig.into()) }
        } else if self.option {
            quote! { #partial: Some(self.#orig) }
        } else if self.ty_override.is_some() {
            quote! { #partial: self.#orig.into() }
        } else {
            quote! { #partial: self.#orig }
        }
    }
}

/// All the pieces extracted from a `#[derive(Partial)]`-annotated struct.
struct ParsedInput {
    original_name: Ident,
    partial_name: Ident,
    vis: syn::Visibility,
    derive_attr: proc_macro2::TokenStream,
    doc_attr: proc_macro2::TokenStream,
    fields: Vec<FieldOpts>,
}

fn parse_input(input: &syn::DeriveInput) -> Result<ParsedInput, TokenStream> {
    let opts =
        PartialOpts::from_derive_input(input).map_err(|e| TokenStream::from(e.write_errors()))?;

    let partial_name = opts
        .name
        .unwrap_or_else(|| format_ident!("Partial{}", opts.ident));

    let derive_attr = if opts.derive.0.is_empty() {
        quote! {}
    } else {
        let derives = &opts.derive.0;
        quote! { #[derive(#(#derives),*)] }
    };

    let doc_attr = match &opts.doc {
        Some(doc) => quote! { #[doc = #doc] },
        None => {
            let doc = format!("Partial version of [`{}`].", opts.ident);
            quote! { #[doc = #doc] }
        }
    };

    let fields = opts
        .data
        .take_struct()
        .expect("Partial can only be derived on structs")
        .fields;

    Ok(ParsedInput {
        original_name: opts.ident,
        partial_name,
        vis: opts.vis,
        derive_attr,
        doc_attr,
        fields,
    })
}

// ----------------------------------------------------------------------------
// Partial — generates the partial struct definition
// ----------------------------------------------------------------------------

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
/// - `#[partial(ty_override = "NewType")]` — overrides the field type entirely.
///
/// Doc comments (`///`), `#[serde(...)]`, and `#[cfg(...)]` attributes on fields are
/// automatically forwarded to the generated struct.
///
/// # Companion derives
///
/// - [`NewPartial`] — generates a `fn new_partial(...)` constructor on the original struct.
/// - [`FromPartial`] — implements `partial_traits::FromPartial` for the original struct.
/// - [`IntoPartial`] — implements `partial_traits::IntoPartial` for the original struct.
///
/// # Example
///
/// ```ignore
/// #[derive(Partial, NewPartial, FromPartial, IntoPartial)]
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
/// ```
#[proc_macro_derive(Partial, attributes(partial))]
pub fn derive_partial(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let parsed = match parse_input(&input) {
        Ok(p) => p,
        Err(e) => return e,
    };

    let partial_name = &parsed.partial_name;
    let vis = &parsed.vis;
    let derive_attr = &parsed.derive_attr;
    let doc_attr = &parsed.doc_attr;

    let partial_fields: Vec<_> = parsed
        .fields
        .iter()
        .filter(|f| !f.skip)
        .map(|f| {
            let name = f.partial_name();
            let field_vis = &f.vis;
            let ty = f.partial_type();

            let preserved_attrs: Vec<_> = f
                .attrs
                .iter()
                .filter(|a| !a.path().is_ident("partial"))
                .collect();

            quote! {
                #(#preserved_attrs)*
                #field_vis #name: #ty,
            }
        })
        .collect();

    quote! {
        #doc_attr
        #derive_attr
        #vis struct #partial_name {
            #(#partial_fields)*
        }
    }
    .into()
}

// ----------------------------------------------------------------------------
// NewPartial — generates `OriginalStruct::new_partial(...) -> PartialStruct`
// ----------------------------------------------------------------------------

/// Generates a `fn new_partial(...)` associated function on the original struct
/// that takes all partial-struct fields as arguments and returns a new partial instance.
///
/// Requires [`Partial`] to also be derived (to define the target struct).
///
/// # Example
///
/// ```ignore
/// #[derive(Partial, NewPartial)]
/// #[partial(name = CreateUser, derive(Debug))]
/// struct User {
///     #[partial(skip)]
///     id: u64,
///     name: String,
/// }
///
/// let user_name = "Alice".to_string();
/// let partial = User::new_partial(user_name);
/// ```
#[proc_macro_derive(NewPartial, attributes(partial))]
pub fn derive_new_partial(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let parsed = match parse_input(&input) {
        Ok(p) => p,
        Err(e) => return e,
    };

    let original_name = &parsed.original_name;
    let vis = &parsed.vis;
    let partial_name = &parsed.partial_name;

    let (params, field_inits): (Vec<_>, Vec<_>) = parsed
        .fields
        .iter()
        .filter(|f| !f.skip)
        .map(|f| {
            let name = f.partial_name();
            let ty = f.partial_type();
            (quote! { #name: #ty }, quote! { #name })
        })
        .unzip();

    quote! {
        impl #original_name {
            /// Create a new partial struct from its fields.
            #vis fn new_partial(#(#params),*) -> #partial_name {
                #partial_name { #(#field_inits),* }
            }
        }
    }
    .into()
}

// ----------------------------------------------------------------------------
// FromPartial — implements `partial_traits::FromPartial<PartialStruct>`
// ----------------------------------------------------------------------------

/// Implements `partial_traits::FromPartial<PartialStruct>` for the original struct.
///
/// Constructs the original struct from the partial plus any extra arguments for
/// fields that cannot be directly mapped (skipped, option-wrapped, or type-overridden).
///
/// The `Args` associated type is a struct containing the extra field values, with fields
/// named to match the original struct's field names.
///
/// Requires [`Partial`] to also be derived and the `partial-traits` crate.
///
/// # Example
///
/// ```ignore
/// #[derive(Partial, FromPartial)]
/// #[partial(name = CreateUser, derive(Debug))]
/// struct User {
///     #[partial(skip)]
///     id: u64,
///     name: String,
/// }
///
/// let user = User::from_partial(partial, CreateUserMissing { id: 42 });
/// ```
#[proc_macro_derive(FromPartial, attributes(partial))]
pub fn derive_from_partial(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let parsed = match parse_input(&input) {
        Ok(p) => p,
        Err(e) => return e,
    };

    let original_name = &parsed.original_name;
    let partial_name = &parsed.partial_name;
    let vis = &parsed.vis;

    // Fields that need extra arguments: skipped, option-wrapped, or type-overridden
    let extra_fields: Vec<_> = parsed
        .fields
        .iter()
        .filter(|f| f.needs_extra_args())
        .collect();

    // Generate the missing fields struct name
    let missing_struct_name = format_ident!("{}Missing", partial_name);

    // Generate struct fields for the missing fields
    let missing_struct_fields: Vec<_> = extra_fields
        .iter()
        .map(|f| {
            let name = f.original_name();
            let ty = &f.ty;
            quote! { #name: #ty }
        })
        .collect();

    // Generate struct definition with only Debug derive for internal struct
    let missing_struct_def = quote! {
        #[derive(Debug)]
        #vis struct #missing_struct_name {
            #(#missing_struct_fields),*
        }
    };

    let from_assignments: Vec<_> = parsed
        .fields
        .iter()
        .map(|f| {
            let orig = f.original_name();
            if f.needs_extra_args() {
                quote! { #orig: missing.#orig }
            } else {
                let partial = f.partial_name();
                quote! { #orig: partial.#partial }
            }
        })
        .collect();

    quote! {
        #missing_struct_def

        impl partial_traits::FromPartial<#partial_name> for #original_name {
            type Args = #missing_struct_name;
            fn from_partial(partial: #partial_name, missing: #missing_struct_name) -> Self {
                Self { #(#from_assignments),* }
            }
        }
    }
    .into()
}

// ----------------------------------------------------------------------------
// IntoPartial — implements `partial_traits::IntoPartial<PartialStruct>`
// ----------------------------------------------------------------------------

/// Implements `partial_traits::IntoPartial<PartialStruct>` for the original struct.
///
/// Converts the original struct into its partial representation, discarding
/// skipped fields and wrapping option-marked fields with `Some(...)`.
///
/// Requires [`Partial`] to also be derived and the `partial-traits` crate.
///
/// # Example
///
/// ```ignore
/// #[derive(Partial, IntoPartial)]
/// #[partial(name = CreateUser, derive(Debug))]
/// struct User {
///     #[partial(skip)]
///     id: u64,
///     name: String,
/// }
///
/// let user = User { id: 1, name: "alice".into() };
/// let partial: CreateUser = user.into_partial();
/// ```
#[proc_macro_derive(IntoPartial, attributes(partial))]
pub fn derive_into_partial(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let parsed = match parse_input(&input) {
        Ok(p) => p,
        Err(e) => return e,
    };

    let original_name = &parsed.original_name;
    let partial_name = &parsed.partial_name;

    let into_assignments: Vec<_> = parsed
        .fields
        .iter()
        .filter(|f| !f.skip)
        .map(|f| f.into_partial_assignment())
        .collect();

    quote! {
        impl partial_traits::IntoPartial<#partial_name> for #original_name {
            fn into_partial(self) -> #partial_name {
                #partial_name { #(#into_assignments),* }
            }
        }
    }
    .into()
}
