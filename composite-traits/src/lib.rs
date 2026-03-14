/// Construct the original type from its composite representation plus any fields
/// that were skipped (or otherwise not directly convertible).
///
/// `Args` is a struct containing the extra field values needed to complete the original
/// struct — typically for fields that are skipped, option-wrapped, or type-overridden.
///
/// # Example
///
/// ```ignore
/// // original struct
/// struct User {
///     id: u64,
///     name: String,
/// }
///
/// // some related struct
/// struct CreateUser {
///     name: String,
/// }
///
/// // to construct the original struct, we need this
/// struct CreateUserMissing {
///     id: u64,
/// }
///
/// impl FromComposite<CreateUser> for User {
///     type Args = CreateUserMissing;
///
///     fn from_composite(composite: CreateUser, missing: CreateUserMissing) -> Self {
///         User { id: missing.id, name: composite.username, .. }
///     }
/// }
/// ```
pub trait FromComposite<T> {
    type Args;

    fn from_composite(composite: T, args: Self::Args) -> Self;
}

/// Convert the original type into its composite representation,
/// discarding any skipped fields.
///
/// # Example
///
/// ```ignore
/// impl IntoComposite<CreateUser> for User {
///     fn into_composite(self) -> CreateUser {
///         CreateUser { username: self.name, location: Some(self.location), .. }
///     }
/// }
/// ```
pub trait IntoComposite<T> {
    fn into_composite(self) -> T;
}
