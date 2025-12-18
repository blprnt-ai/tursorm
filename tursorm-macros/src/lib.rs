#![deny(warnings, unused_crate_dependencies)]

//! Procedural macros for tursorm
//!
//! This crate provides derive macros for defining database entities.

use convert_case::Case;
use convert_case::Casing;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::format_ident;
use quote::quote;
use syn::Attribute;
use syn::Data;
use syn::DeriveInput;
use syn::Field;
use syn::Fields;
use syn::Ident;
use syn::Lit;
use syn::Meta;
use syn::Type;
use syn::parse_macro_input;

/// Information about a struct field
struct FieldInfo {
    field_name:        Ident,
    variant_name:      Ident,
    column_name:       String,
    field_type:        Type,
    is_primary_key:    bool,
    is_optional:       bool,
    is_auto_increment: bool,
    is_unique:         bool,
    default_value:     Option<String>,
    renamed_from:      Option<String>,
}

/// Derive macro for creating a database entity model.
///
/// # Example
///
/// ```ignore
/// use tursorm::prelude::*;
///
/// #[derive(Clone, Debug, Entity)]
/// #[tursorm(table_name = "users")]
/// pub struct User {
///     #[tursorm(primary_key, auto_increment)]
///     pub id: i64,
///     pub name: String,
///     pub email: String,
///     #[tursorm(column_name = "created_at")]
///     pub created_at: Option<String>,
///     // Rename a column during migration (from "timestamp" to "updated_at")
///     #[tursorm(renamed_from = "timestamp")]
///     pub updated_at: i64,
/// }
/// ```
///
/// # Field Attributes
///
/// - `primary_key` - Mark field as primary key
/// - `auto_increment` - Mark field as auto-increment
/// - `unique` - Mark field as unique
/// - `column_name = "name"` - Override the database column name
/// - `renamed_from = "old_name"` - Rename column from old name during migration
/// - `default = "value"` - Set default SQL expression
#[proc_macro_derive(Entity, attributes(tursorm))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let expanded = impl_entity(&input);
    TokenStream::from(expanded)
}

fn impl_entity(input: &DeriveInput) -> TokenStream2 {
    let name = &input.ident;
    let entity_name = format_ident!("{}Entity", name);
    let column_enum_name = format_ident!("{}Column", name);
    let active_model_name = format_ident!("{}ActiveModel", name);

    // Parse table name from attributes
    let table_name = get_table_name(&input.attrs, name);

    // Get fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("Entity derive only supports structs with named fields"),
        },
        _ => panic!("Entity derive only supports structs"),
    };

    let field_info: Vec<FieldInfo> = fields.iter().map(parse_field_info).collect();

    // Generate Column enum variants
    let column_variants: Vec<_> = field_info
        .iter()
        .map(|f| {
            let variant_name = &f.variant_name;
            quote! { #variant_name }
        })
        .collect();

    // Generate column name mappings
    let column_name_arms: Vec<_> = field_info
        .iter()
        .map(|f| {
            let variant_name = &f.variant_name;
            let col_name = &f.column_name;
            quote! { Self::#variant_name => #col_name }
        })
        .collect();

    // Generate column type mappings
    let column_type_arms: Vec<_> = field_info
        .iter()
        .map(|f| {
            let variant_name = &f.variant_name;
            let col_type = rust_type_to_column_type(&f.field_type, f.is_optional);
            quote! { Self::#variant_name => #col_type }
        })
        .collect();

    // Find primary key
    let primary_key_field = field_info
        .iter()
        .find(|f| f.is_primary_key)
        .expect("Entity must have a primary key field marked with #[tursorm(primary_key)]");

    let pk_variant = &primary_key_field.variant_name;
    let pk_field_name = &primary_key_field.field_name;

    // Generate FromRow implementation
    let from_row_fields: Vec<_> = field_info
        .iter()
        .enumerate()
        .map(|(idx, f)| {
            let field_name = &f.field_name;
            if f.is_optional {
                quote! {
                    #field_name: tursorm::FromValue::from_value_opt(row.get_value(#idx)?)?
                }
            } else {
                quote! {
                    #field_name: tursorm::FromValue::from_value(row.get_value(#idx)?)?
                }
            }
        })
        .collect();

    // Generate ActiveModel fields
    let active_model_fields: Vec<_> = field_info
        .iter()
        .map(|f| {
            let field_name = &f.field_name;
            let field_type = &f.field_type;
            quote! {
                pub #field_name: tursorm::ActiveValue<#field_type>
            }
        })
        .collect();

    // Generate ActiveModel from Model
    let active_model_from_model_fields: Vec<_> = field_info
        .iter()
        .map(|f| {
            let field_name = &f.field_name;
            quote! {
                #field_name: tursorm::ActiveValue::Set(model.#field_name.clone())
            }
        })
        .collect();

    // Generate insert columns and values (skip auto_increment fields that are NotSet)
    let insert_set_arms: Vec<_> = field_info
        .iter()
        .map(|f| {
            let field_name = &f.field_name;
            let col_name = &f.column_name;
            if f.is_auto_increment {
                quote! {
                    if let tursorm::ActiveValue::Set(ref v) = self.#field_name {
                        columns.push(#col_name);
                        values.push(tursorm::IntoValue::into_value(v.clone()));
                    }
                }
            } else {
                quote! {
                    if let tursorm::ActiveValue::Set(ref v) = self.#field_name {
                        columns.push(#col_name);
                        values.push(tursorm::IntoValue::into_value(v.clone()));
                    }
                }
            }
        })
        .collect();

    // Generate update set arms (exclude primary key)
    let update_set_arms: Vec<_> = field_info
        .iter()
        .filter(|f| !f.is_primary_key)
        .map(|f| {
            let field_name = &f.field_name;
            let col_name = &f.column_name;
            quote! {
                if let tursorm::ActiveValue::Set(ref v) = self.#field_name {
                    sets.push((#col_name, tursorm::IntoValue::into_value(v.clone())));
                }
            }
        })
        .collect();

    let pk_column_name = &primary_key_field.column_name;
    let pk_is_auto_increment = primary_key_field.is_auto_increment;

    // All column names for SELECT *
    let all_columns: Vec<_> = field_info.iter().map(|f| f.column_name.as_str()).collect();
    let all_columns_str = all_columns.join(", ");

    // Column count
    let column_count = field_info.len();

    // Generate is_nullable arms
    let is_nullable_arms: Vec<_> = field_info
        .iter()
        .map(|f| {
            let variant_name = &f.variant_name;
            let is_nullable = f.is_optional;
            quote! { Self::#variant_name => #is_nullable }
        })
        .collect();

    // Generate is_primary_key arms
    let is_primary_key_arms: Vec<_> = field_info
        .iter()
        .map(|f| {
            let variant_name = &f.variant_name;
            let is_pk = f.is_primary_key;
            quote! { Self::#variant_name => #is_pk }
        })
        .collect();

    // Generate is_auto_increment arms
    let is_auto_increment_arms: Vec<_> = field_info
        .iter()
        .map(|f| {
            let variant_name = &f.variant_name;
            let is_auto = f.is_auto_increment;
            quote! { Self::#variant_name => #is_auto }
        })
        .collect();

    // Generate is_unique arms
    let is_unique_arms: Vec<_> = field_info
        .iter()
        .map(|f| {
            let variant_name = &f.variant_name;
            let is_unique = f.is_unique;
            quote! { Self::#variant_name => #is_unique }
        })
        .collect();

    // Generate default_value arms
    let default_value_arms: Vec<_> = field_info
        .iter()
        .map(|f| {
            let variant_name = &f.variant_name;
            match &f.default_value {
                Some(val) => quote! { Self::#variant_name => Some(#val) },
                None => quote! { Self::#variant_name => None },
            }
        })
        .collect();

    // Generate renamed_from arms
    let renamed_from_arms: Vec<_> = field_info
        .iter()
        .map(|f| {
            let variant_name = &f.variant_name;
            match &f.renamed_from {
                Some(old_name) => quote! { Self::#variant_name => Some(#old_name) },
                None => quote! { Self::#variant_name => None },
            }
        })
        .collect();

    quote! {
        /// Column enum for #name
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum #column_enum_name {
            #(#column_variants),*
        }

        impl tursorm::ColumnTrait for #column_enum_name {
            fn name(&self) -> &'static str {
                match self {
                    #(#column_name_arms),*
                }
            }

            fn column_type(&self) -> tursorm::ColumnType {
                match self {
                    #(#column_type_arms),*
                }
            }

            fn is_nullable(&self) -> bool {
                match self {
                    #(#is_nullable_arms),*
                }
            }

            fn is_primary_key(&self) -> bool {
                match self {
                    #(#is_primary_key_arms),*
                }
            }

            fn is_auto_increment(&self) -> bool {
                match self {
                    #(#is_auto_increment_arms),*
                }
            }

            fn is_unique(&self) -> bool {
                match self {
                    #(#is_unique_arms),*
                }
            }

            fn default_value(&self) -> Option<&'static str> {
                match self {
                    #(#default_value_arms),*
                }
            }

            fn renamed_from(&self) -> Option<&'static str> {
                match self {
                    #(#renamed_from_arms),*
                }
            }

            fn all() -> &'static [Self] {
                &[#(Self::#column_variants),*]
            }
        }

        impl std::fmt::Display for #column_enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.name())
            }
        }

        /// Entity marker struct for #name
        #[derive(Clone, Copy, Debug, Default)]
        pub struct #entity_name;

        impl tursorm::EntityTrait for #entity_name {
            type Model = #name;
            type Column = #column_enum_name;
            type ActiveModel = #active_model_name;

            fn table_name() -> &'static str {
                #table_name
            }

            fn primary_key() -> Self::Column {
                #column_enum_name::#pk_variant
            }

            fn primary_key_auto_increment() -> bool {
                #pk_is_auto_increment
            }

            fn all_columns() -> &'static str {
                #all_columns_str
            }

            fn column_count() -> usize {
                #column_count
            }
        }

        impl tursorm::FromRow for #name {
            fn from_row(row: &turso::Row) -> tursorm::Result<Self> {
                Ok(Self {
                    #(#from_row_fields),*
                })
            }
        }

        impl tursorm::ModelTrait for #name {
            type Entity = #entity_name;

            fn get_primary_key_value(&self) -> tursorm::Value {
                tursorm::IntoValue::into_value(self.#pk_field_name.clone())
            }
        }

        impl #name {
            /// Start a find query for this entity
            ///
            /// # Example
            ///
            /// ```ignore
            /// let users = User::find()
            ///     .filter(Condition::eq(UserColumn::Status, "active"))
            ///     .all(&conn)
            ///     .await?;
            /// ```
            pub fn find() -> tursorm::Select<#entity_name> {
                tursorm::Select::new()
            }

            /// Find by primary key
            ///
            /// # Example
            ///
            /// ```ignore
            /// let user = User::find_by_id(1).one(&conn).await?;
            /// ```
            pub fn find_by_id<V: tursorm::IntoValue>(id: V) -> tursorm::Select<#entity_name> {
                tursorm::Select::new().filter(
                    tursorm::Condition::eq(#column_enum_name::#pk_variant, id)
                )
            }
        }

        impl #entity_name {
            /// Create a new default ActiveModel for this entity
            ///
            /// This is a convenience method to avoid fully-qualified syntax.
            /// Instead of `<Entity as EntityTrait>::ActiveModel::default()`,
            /// you can simply use `Entity::active_model()`.
            pub fn active_model() -> #active_model_name {
                #active_model_name::default()
            }
        }

        /// ActiveModel for #name - used for insert/update operations
        #[derive(Clone, Debug, Default)]
        pub struct #active_model_name {
            #(#active_model_fields),*
        }

        impl tursorm::ActiveModelTrait for #active_model_name {
            type Entity = #entity_name;

            fn get_insert_columns_and_values(&self) -> (Vec<&'static str>, Vec<tursorm::Value>) {
                let mut columns = Vec::new();
                let mut values = Vec::new();
                #(#insert_set_arms)*
                (columns, values)
            }

            fn get_update_sets(&self) -> Vec<(&'static str, tursorm::Value)> {
                let mut sets = Vec::new();
                #(#update_set_arms)*
                sets
            }

            fn get_primary_key_value(&self) -> Option<tursorm::Value> {
                match &self.#pk_field_name {
                    tursorm::ActiveValue::Set(v) => Some(tursorm::IntoValue::into_value(v.clone())),
                    tursorm::ActiveValue::NotSet => None,
                }
            }

            fn primary_key_column() -> &'static str {
                #pk_column_name
            }
        }

        impl From<#name> for #active_model_name {
            fn from(model: #name) -> Self {
                Self {
                    #(#active_model_from_model_fields),*
                }
            }
        }
    }
}

fn get_table_name(attrs: &[Attribute], struct_name: &Ident) -> String {
    for attr in attrs {
        if attr.path().is_ident("tursorm") {
            if let Ok(nested) = attr.parse_args::<syn::Meta>() {
                if let Meta::NameValue(nv) = nested {
                    if nv.path.is_ident("table_name") {
                        if let syn::Expr::Lit(expr_lit) = &nv.value {
                            if let Lit::Str(lit_str) = &expr_lit.lit {
                                return lit_str.value();
                            }
                        }
                    }
                }
            }
            // Try parsing as a list
            if let Ok(list) = attr.meta.require_list() {
                let tokens = list.tokens.to_string();
                if let Some(start) = tokens.find("table_name") {
                    let rest = &tokens[start..];
                    if let Some(eq_pos) = rest.find('=') {
                        let value_part = rest[eq_pos + 1..].trim();
                        if value_part.starts_with('"') {
                            if let Some(end) = value_part[1..].find('"') {
                                return value_part[1..end + 1].to_string();
                            }
                        }
                    }
                }
            }
        }
    }
    // Default: convert struct name to snake_case and pluralize
    let snake = struct_name.to_string().to_case(Case::Snake);
    format!("{}s", snake)
}

fn parse_field_info(field: &Field) -> FieldInfo {
    let field_name = field.ident.clone().expect("Field must have a name");
    let variant_name = format_ident!("{}", field_name.to_string().to_case(Case::Pascal));
    let field_type = field.ty.clone();

    let mut column_name = field_name.to_string().to_case(Case::Snake);
    let mut is_primary_key = false;
    let mut is_auto_increment = false;
    let mut is_unique = false;
    let mut default_value: Option<String> = None;
    let mut renamed_from: Option<String> = None;

    // Check if the type is Option<T>
    let is_optional = is_option_type(&field_type);

    // Parse field attributes
    for attr in &field.attrs {
        if attr.path().is_ident("tursorm") {
            if let Ok(list) = attr.meta.require_list() {
                let tokens = list.tokens.to_string();

                if tokens.contains("primary_key") {
                    is_primary_key = true;
                }

                if tokens.contains("auto_increment") {
                    is_auto_increment = true;
                }

                if tokens.contains("unique") && !tokens.contains("primary_key") {
                    // unique as a standalone attribute (not part of primary_key)
                    is_unique = true;
                }

                // Parse column_name
                if let Some(start) = tokens.find("column_name") {
                    let rest = &tokens[start..];
                    if let Some(eq_pos) = rest.find('=') {
                        let value_part = rest[eq_pos + 1..].trim();
                        if value_part.starts_with('"') {
                            if let Some(end) = value_part[1..].find('"') {
                                column_name = value_part[1..end + 1].to_string();
                            }
                        }
                    }
                }

                // Parse default value
                if let Some(start) = tokens.find("default") {
                    let rest = &tokens[start..];
                    if let Some(eq_pos) = rest.find('=') {
                        let value_part = rest[eq_pos + 1..].trim();
                        if value_part.starts_with('"') {
                            if let Some(end) = value_part[1..].find('"') {
                                default_value = Some(value_part[1..end + 1].to_string());
                            }
                        }
                    }
                }

                // Parse renamed_from (for migrations)
                if let Some(start) = tokens.find("renamed_from") {
                    let rest = &tokens[start..];
                    if let Some(eq_pos) = rest.find('=') {
                        let value_part = rest[eq_pos + 1..].trim();
                        if value_part.starts_with('"') {
                            if let Some(end) = value_part[1..].find('"') {
                                renamed_from = Some(value_part[1..end + 1].to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Validate: auto_increment can only be used on integer types
    if is_auto_increment && !is_integer_type(&field_type) {
        panic!(
            "Field `{}` has `auto_increment` attribute but is not an integer type. \
             The `auto_increment` attribute can only be used on integer fields \
             (i8, i16, i32, i64, u8, u16, u32, u64, isize, usize).",
            field_name
        );
    }

    FieldInfo {
        field_name,
        variant_name,
        column_name,
        field_type,
        is_primary_key,
        is_optional,
        is_auto_increment,
        is_unique,
        default_value,
        renamed_from,
    }
}

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

fn is_integer_type(ty: &Type) -> bool {
    let inner_type = if is_option_type(ty) { extract_option_inner_type(ty).unwrap_or(ty) } else { ty };

    if let Type::Path(type_path) = inner_type {
        if let Some(segment) = type_path.path.segments.last() {
            let type_name = segment.ident.to_string();
            return matches!(
                type_name.as_str(),
                "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "isize" | "usize"
            );
        }
    }
    false
}

fn rust_type_to_column_type(ty: &Type, is_optional: bool) -> TokenStream2 {
    let inner_type = if is_optional { extract_option_inner_type(ty).unwrap_or(ty) } else { ty };

    let base_type = match inner_type {
        Type::Path(type_path) => {
            let segment = type_path.path.segments.last().unwrap();
            let type_name = segment.ident.to_string();
            match type_name.as_str() {
                "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" => {
                    quote! { tursorm::ColumnType::Integer }
                }
                "f32" | "f64" => quote! { tursorm::ColumnType::Float },
                "String" | "str" => quote! { tursorm::ColumnType::Text },
                "Vec" => {
                    // Vec<u8> is Blob, other Vec<T> are Text (JSON arrays)
                    if let Some(inner) = extract_vec_inner_type(inner_type) {
                        if let Type::Path(inner_path) = inner {
                            if let Some(seg) = inner_path.path.segments.last() {
                                if seg.ident == "u8" {
                                    return quote! { tursorm::ColumnType::Blob };
                                }
                            }
                        }
                    }
                    quote! { tursorm::ColumnType::Text }
                }
                "bool" => quote! { tursorm::ColumnType::Integer },
                _ => quote! { tursorm::ColumnType::Text },
            }
        }
        _ => quote! { tursorm::ColumnType::Text },
    };

    base_type
}

fn extract_option_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
                }
            }
        }
    }
    None
}

fn extract_vec_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Vec" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
                }
            }
        }
    }
    None
}
