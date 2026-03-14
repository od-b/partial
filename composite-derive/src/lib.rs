use darling::{FromDeriveInput, FromField, FromMeta, ast};
use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{Ident, Meta, Type, parse_macro_input};

/// Struct-level `#[composite(...)]` options.
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(composite))]
struct CompositeOpts {
    ident: Ident,
    vis: syn::Visibility,
    data: ast::Data<(), FieldOpts>,

    /// Name of the generated composite struct: `#[composite(name = CreateUser)]`
    /// Defaults to `Composite{OriginalName}` when omitted.
    #[darling(default)]
    name: Option<Ident>,

    /// Derives to apply: `#[composite(derive(Debug, Deserialize))]`
    #[darling(default)]
    derive: PathList,

    /// Optional module-level documentation for the generated struct.
    #[darling(default)]
    doc: Option<String>,
}

/// Field-level `#[composite(...)]` options.
#[derive(Debug, FromField)]
#[darling(attributes(composite), forward_attrs(doc, serde, cfg))]
struct FieldOpts {
    ident: Option<Ident>,
    vis: syn::Visibility,
    ty: Type,
    attrs: Vec<syn::Attribute>,

    /// Skip this field entirely: `#[composite(skip)]`
    #[darling(default)]
    skip: bool,

    /// Wrap the type in `Option<T>`: `#[composite(option)]`
    #[darling(default)]
    option: bool,

    /// Rename the field: `#[composite(rename = "new_name")]`
    #[darling(default)]
    rename: Option<String>,

    /// Override the type: `#[composite(ty = "NewType")]`
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
    /// The name this field has in the composite struct.
    fn composite_name(&self) -> Ident {
        match &self.rename {
            Some(new_name) => format_ident!("{}", new_name),
            None => self.ident.clone().expect("named fields required"),
        }
    }

    /// The original field name in the source struct.
    fn original_name(&self) -> Ident {
        self.ident.clone().expect("named fields required")
    }

    /// The type this field has in the composite struct.
    fn composite_type(&self) -> Type {
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

    /// Whether this field can be directly assigned from composite to original
    /// (i.e. no type transformation was applied).
    fn is_direct(&self) -> bool {
        !self.option && self.ty_override.is_none()
    }

    /// Whether this field requires extra arguments (skipped, option-wrapped, or type-overridden).
    fn needs_extra_args(&self) -> bool {
        self.skip || !self.is_direct()
    }

    /// Generate the token stream for this field's assignment in `into_composite`.
    fn into_composite_assignment(&self) -> proc_macro2::TokenStream {
        let orig = self.original_name();
        let composite = self.composite_name();

        if self.option && self.ty_override.is_some() {
            quote! { #composite: Some(self.#orig.into()) }
        } else if self.option {
            quote! { #composite: Some(self.#orig) }
        } else if self.ty_override.is_some() {
            quote! { #composite: self.#orig.into() }
        } else {
            quote! { #composite: self.#orig }
        }
    }
}

/// All the pieces extracted from a `#[derive(Composite)]`-annotated struct.
struct ParsedInput {
    original_name: Ident,
    composite_name: Ident,
    vis: syn::Visibility,
    derive_attr: proc_macro2::TokenStream,
    doc_attr: proc_macro2::TokenStream,
    fields: Vec<FieldOpts>,
}

fn parse_input(input: &syn::DeriveInput) -> Result<ParsedInput, TokenStream> {
    let opts =
        CompositeOpts::from_derive_input(input).map_err(|e| TokenStream::from(e.write_errors()))?;

    let composite_name = opts
        .name
        .unwrap_or_else(|| format_ident!("Composite{}", opts.ident));

    let derive_attr = if opts.derive.0.is_empty() {
        quote! {}
    } else {
        let derives = &opts.derive.0;
        quote! { #[derive(#(#derives),*)] }
    };

    let doc_attr = match &opts.doc {
        Some(doc) => quote! { #[doc = #doc] },
        None => {
            let doc = format!("Composite version of [`{}`].", opts.ident);
            quote! { #[doc = #doc] }
        }
    };

    let fields = opts
        .data
        .take_struct()
        .expect("Composite can only be derived on structs")
        .fields;

    Ok(ParsedInput {
        original_name: opts.ident,
        composite_name,
        vis: opts.vis,
        derive_attr,
        doc_attr,
        fields,
    })
}

// ----------------------------------------------------------------------------
// Composite — generates the composite struct definition
// ----------------------------------------------------------------------------

/// Derive macro that generates a composite version of a struct with a subset of its fields.
///
/// # Struct-level attributes
///
/// - `#[composite(name = CompositeName)]` — sets the name of the generated struct.
///   Defaults to `Composite{OriginalName}` when omitted (e.g. `User` → `CompositeUser`).
/// - `#[composite(derive(Debug, Serialize, ...))]` — adds `#[derive(...)]` to the generated struct.
/// - `#[composite(doc = "...")]` — sets a custom doc comment. When omitted, a default
///   `Composite version of [OriginalName].` is generated.
///
/// # Field-level attributes
///
/// - `#[composite(skip)]` — omits the field from the generated struct.
/// - `#[composite(rename = "new_name")]` — renames the field in the generated struct.
/// - `#[composite(option)]` — wraps the field type in `Option<T>`.
/// - `#[composite(ty_override = "NewType")]` — overrides the field type entirely.
///
/// Doc comments (`///`), `#[serde(...)]`, and `#[cfg(...)]` attributes on fields are
/// automatically forwarded to the generated struct.
///
/// # Companion derives
///
/// - [`NewComposite`] — generates a `fn new_composite(...)` constructor on the original struct.
/// - [`FromComposite`] — implements `composite_traits::FromComposite` for the original struct.
/// - [`IntoComposite`] — implements `composite_traits::IntoComposite` for the original struct.
///
/// # Example
///
/// ```ignore
/// #[derive(Composite, NewComposite, FromComposite, IntoComposite)]
/// #[composite(name = CreateUser, derive(Debug, Deserialize))]
/// struct User {
///     #[composite(skip)]
///     id: u64,
///     #[composite(rename = "username")]
///     name: String,
///     #[composite(option)]
///     location: String,
///     email: String,
/// }
/// ```
#[proc_macro_derive(Composite, attributes(composite))]
pub fn derive_composite(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let parsed = match parse_input(&input) {
        Ok(p) => p,
        Err(e) => return e,
    };

    let composite_name = &parsed.composite_name;
    let vis = &parsed.vis;
    let derive_attr = &parsed.derive_attr;
    let doc_attr = &parsed.doc_attr;

    let composite_fields: Vec<_> = parsed
        .fields
        .iter()
        .filter(|f| !f.skip)
        .map(|f| {
            let name = f.composite_name();
            let field_vis = &f.vis;
            let ty = f.composite_type();

            let preserved_attrs: Vec<_> = f
                .attrs
                .iter()
                .filter(|a| !a.path().is_ident("composite"))
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
        #vis struct #composite_name {
            #(#composite_fields)*
        }
    }
    .into()
}

// ----------------------------------------------------------------------------
// NewComposite — generates `OriginalStruct::new_composite(...) -> CompositeStruct`
// ----------------------------------------------------------------------------

/// Generates a `fn new_composite(...)` associated function on the original struct
/// that takes all composite-struct fields as arguments and returns a new composite instance.
///
/// Requires [`Composite`] to also be derived (to define the target struct).
///
/// # Example
///
/// ```ignore
/// #[derive(Composite, NewComposite)]
/// #[composite(name = CreateUser, derive(Debug))]
/// struct User {
///     #[composite(skip)]
///     id: u64,
///     name: String,
/// }
///
/// let user_name = "Alice".to_string();
/// let composite = User::new_composite(user_name);
/// ```
#[proc_macro_derive(NewComposite, attributes(composite))]
pub fn derive_new_composite(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let parsed = match parse_input(&input) {
        Ok(p) => p,
        Err(e) => return e,
    };

    let original_name = &parsed.original_name;
    let vis = &parsed.vis;
    let composite_name = &parsed.composite_name;

    let (params, field_inits): (Vec<_>, Vec<_>) = parsed
        .fields
        .iter()
        .filter(|f| !f.skip)
        .map(|f| {
            let name = f.composite_name();
            let ty = f.composite_type();
            (quote! { #name: #ty }, quote! { #name })
        })
        .unzip();

    quote! {
        impl #original_name {
            /// Create a new composite struct from its fields.
            #vis fn new_composite(#(#params),*) -> #composite_name {
                #composite_name { #(#field_inits),* }
            }
        }
    }
    .into()
}

// ----------------------------------------------------------------------------
// FromComposite — implements `composite_traits::FromComposite<CompositeStruct>`
// ----------------------------------------------------------------------------

/// Implements `composite_traits::FromComposite<CompositeStruct>` for the original struct.
///
/// Constructs the original struct from the composite plus any extra arguments for
/// fields that cannot be directly mapped (skipped, option-wrapped, or type-overridden).
///
/// The `Args` associated type is a struct containing the extra field values, with fields
/// named to match the original struct's field names.
///
/// Requires [`Composite`] to also be derived and the `composite-traits` crate.
///
/// # Example
///
/// ```ignore
/// #[derive(Composite, FromComposite)]
/// #[composite(name = CreateUser, derive(Debug))]
/// struct User {
///     #[composite(skip)]
///     id: u64,
///     name: String,
/// }
///
/// let user = User::from_composite(composite, CreateUserMissing { id: 42 });
/// ```
#[proc_macro_derive(FromComposite, attributes(composite))]
pub fn derive_from_composite(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let parsed = match parse_input(&input) {
        Ok(p) => p,
        Err(e) => return e,
    };

    let original_name = &parsed.original_name;
    let composite_name = &parsed.composite_name;
    let vis = &parsed.vis;

    // Fields that need extra arguments: skipped, option-wrapped, or type-overridden
    let extra_fields: Vec<_> = parsed
        .fields
        .iter()
        .filter(|f| f.needs_extra_args())
        .collect();

    // Generate the missing fields struct name
    let missing_struct_name = format_ident!("{}Missing", composite_name);

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
                let composite = f.composite_name();
                quote! { #orig: composite.#composite }
            }
        })
        .collect();

    quote! {
        #missing_struct_def

        impl composite_traits::FromComposite<#composite_name> for #original_name {
            type Args = #missing_struct_name;
            fn from_composite(composite: #composite_name, missing: #missing_struct_name) -> Self {
                Self { #(#from_assignments),* }
            }
        }
    }
    .into()
}

// ----------------------------------------------------------------------------
// IntoComposite — implements `composite_traits::IntoComposite<CompositeStruct>`
// ----------------------------------------------------------------------------

/// Implements `composite_traits::IntoComposite<CompositeStruct>` for the original struct.
///
/// Converts the original struct into its composite representation, discarding
/// skipped fields and wrapping option-marked fields with `Some(...)`.
///
/// Requires [`Composite`] to also be derived and the `composite-traits` crate.
///
/// # Example
///
/// ```ignore
/// #[derive(Composite, IntoComposite)]
/// #[composite(name = CreateUser, derive(Debug))]
/// struct User {
///     #[composite(skip)]
///     id: u64,
///     name: String,
/// }
///
/// let user = User { id: 1, name: "alice".into() };
/// let composite: CreateUser = user.into_composite();
/// ```
#[proc_macro_derive(IntoComposite, attributes(composite))]
pub fn derive_into_composite(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let parsed = match parse_input(&input) {
        Ok(p) => p,
        Err(e) => return e,
    };

    let original_name = &parsed.original_name;
    let composite_name = &parsed.composite_name;

    let into_assignments: Vec<_> = parsed
        .fields
        .iter()
        .filter(|f| !f.skip)
        .map(|f| f.into_composite_assignment())
        .collect();

    quote! {
        impl composite_traits::IntoComposite<#composite_name> for #original_name {
            fn into_composite(self) -> #composite_name {
                #composite_name { #(#into_assignments),* }
            }
        }
    }
    .into()
}
