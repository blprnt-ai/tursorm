use darling::FromDeriveInput;
use darling::FromField;
use darling::FromMeta;
use proc_macro2::Ident;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use quote::format_ident;
use quote::quote;
use syn::DeriveInput;
use syn::Type;

#[derive(Debug, Clone, Copy, Default, FromMeta)]
enum OnDelete {
    Restrict,
    Cascade,
    SetNull,
    SetDefault,
    #[default]
    None,
}

#[derive(Debug, Clone, Copy, Default, FromMeta)]
enum OnUpdate {
    Restrict,
    Cascade,
    SetNull,
    SetDefault,
    #[default]
    None,
}

#[derive(Debug, FromField)]
#[darling(attributes(tursorm))]
struct FieldReceiver {
    pub ident: Option<Ident>,
    pub ty:    Type,

    #[darling(default)]
    pub primary_key: bool,

    #[darling(default)]
    pub auto_increment: bool,

    #[darling(default)]
    pub unique: bool,

    #[darling(default)]
    pub column_name: Option<String>,

    #[darling(default)]
    pub renamed_from: Option<String>,

    #[darling(default)]
    pub default: Option<String>,

    #[darling(default)]
    pub foreign_key: bool,

    #[darling(default)]
    pub references: Option<String>,

    #[darling(default)]
    pub on_delete: Option<OnDelete>,

    #[darling(default)]
    pub on_update: Option<OnUpdate>,
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(tursorm), supports(struct_named))]
struct EntityReceiver {
    pub ident: Ident,
    pub data:  darling::ast::Data<(), FieldReceiver>,

    #[darling(default)]
    pub table_name: Option<String>,
}

#[derive(Debug)]
struct ForeignKeyInfo {
    pub table_name:  String,
    pub column_name: String,
    pub on_delete:   OnDelete,
    pub on_update:   OnUpdate,
}

impl ToTokens for OnDelete {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let variant = match self {
            OnDelete::Restrict => quote! { tursorm::OnDelete::Restrict },
            OnDelete::Cascade => quote! { tursorm::OnDelete::Cascade },
            OnDelete::SetNull => quote! { tursorm::OnDelete::SetNull },
            OnDelete::SetDefault => quote! { tursorm::OnDelete::SetDefault },
            OnDelete::None => quote! { tursorm::OnDelete::None },
        };
        tokens.extend(variant);
    }
}

impl ToTokens for OnUpdate {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let variant = match self {
            OnUpdate::Restrict => quote! { tursorm::OnUpdate::Restrict },
            OnUpdate::Cascade => quote! { tursorm::OnUpdate::Cascade },
            OnUpdate::SetNull => quote! { tursorm::OnUpdate::SetNull },
            OnUpdate::SetDefault => quote! { tursorm::OnUpdate::SetDefault },
            OnUpdate::None => quote! { tursorm::OnUpdate::None },
        };
        tokens.extend(variant);
    }
}

impl ToTokens for ForeignKeyInfo {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let table_name = &self.table_name;
        let column_name = &self.column_name;
        let on_delete = &self.on_delete;
        let on_update = &self.on_update;
        tokens.extend(quote! {
            tursorm::ForeignKeyInfo {
                table_name: #table_name,
                column_name: #column_name,
                on_delete: #on_delete,
                on_update: #on_update,
            }
        });
    }
}

#[derive(Debug)]
struct FieldInfo {
    pub field_name:        Ident,
    pub variant_name:      Ident,
    pub column_name:       String,
    pub field_type:        Type,
    pub is_primary_key:    bool,
    pub is_optional:       bool,
    pub is_auto_increment: bool,
    pub is_unique:         bool,
    pub default_value:     Option<String>,
    pub renamed_from:      Option<String>,
    pub foreign_key:       Option<ForeignKeyInfo>,
}

#[derive(Debug)]
struct EntityInfo {
    pub struct_name: Ident,
    pub table_name:  String,
    pub fields:      Vec<FieldInfo>,
}

impl FieldReceiver {
    pub fn to_field_info(self) -> FieldInfo {
        let field_name = self.ident.expect("Expected named field");
        let is_optional = is_option_type(&self.ty);
        let variant_name = to_pascal_case(&field_name);

        let column_name = self.column_name.unwrap_or_else(|| field_name.to_string());

        let foreign_key = if self.foreign_key {
            if self.references.is_none() {
                panic!("Foreign key must have a references attribute");
            }

            let (table, col) = parse_references(self.references.unwrap());
            Some(ForeignKeyInfo {
                table_name:  table.to_string(),
                column_name: col.to_string(),
                on_delete:   self.on_delete.unwrap_or_default(),
                on_update:   self.on_update.unwrap_or_default(),
            })
        } else {
            None
        };

        FieldInfo {
            field_name,
            variant_name,
            column_name,
            field_type: self.ty,
            is_primary_key: self.primary_key,
            is_optional,
            is_auto_increment: self.auto_increment,
            is_unique: self.unique,
            default_value: self.default,
            renamed_from: self.renamed_from,
            foreign_key,
        }
    }
}

impl EntityReceiver {
    pub fn to_entity_info(self) -> EntityInfo {
        let table_name = self.table_name.unwrap_or_else(|| to_snake_case(&self.ident));

        let fields =
            self.data.take_struct().expect("Expected struct").fields.into_iter().map(|f| f.to_field_info()).collect();

        EntityInfo { struct_name: self.ident, table_name, fields }
    }
}

#[proc_macro_derive(Entity, attributes(tursorm))]
pub fn derive_entity(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    let receiver = match EntityReceiver::from_derive_input(&input) {
        Ok(r) => r,
        Err(e) => return e.write_errors().into(),
    };

    let entity_info = receiver.to_entity_info();

    let expanded = impl_entity(&entity_info);
    proc_macro::TokenStream::from(expanded)
}

fn impl_entity(entity_info: &EntityInfo) -> TokenStream2 {
    let name = &entity_info.struct_name;
    let entity_name = format_ident!("{}Entity", name);
    let column_enum_name = format_ident!("{}Column", name);
    let active_model_name = format_ident!("{}ActiveModel", name);

    let table_name = entity_info.table_name.clone();

    let column_variants: Vec<_> = entity_info
        .fields
        .iter()
        .map(|f| {
            let variant_name = &f.variant_name;
            quote! { #variant_name }
        })
        .collect();

    let column_name_arms: Vec<_> = entity_info
        .fields
        .iter()
        .map(|f| {
            let variant_name = &f.variant_name;
            let col_name = &f.column_name;
            quote! { Self::#variant_name => #col_name }
        })
        .collect();

    let column_type_arms: Vec<_> = entity_info
        .fields
        .iter()
        .map(|f| {
            let variant_name = &f.variant_name;
            let col_type = rust_type_to_column_type(&f.field_type, f.is_optional);
            quote! { Self::#variant_name => #col_type }
        })
        .collect();

    let primary_key_fields = entity_info.fields.iter().filter(|f| f.is_primary_key).collect::<Vec<_>>();

    if primary_key_fields.is_empty() {
        panic!("Entity must have a primary key field marked with #[tursorm(primary_key)]");
    } else if primary_key_fields.len() > 1 {
        panic!("Entity must have only one primary key field marked with #[tursorm(primary_key)]");
    }

    let primary_key_field = primary_key_fields[0];

    let pk_variant = &primary_key_field.variant_name;
    let pk_field_name = &primary_key_field.field_name;

    let from_row_fields: Vec<_> = entity_info
        .fields
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

    let active_model_fields: Vec<_> = entity_info
        .fields
        .iter()
        .map(|f| {
            let field_name = &f.field_name;
            let field_type = &f.field_type;
            quote! {
                pub #field_name: tursorm::ActiveValue<#field_type>
            }
        })
        .collect();

    let active_model_from_model_fields: Vec<_> = entity_info
        .fields
        .iter()
        .map(|f| {
            let field_name = &f.field_name;
            quote! {
                #field_name: tursorm::ActiveValue::Set(model.#field_name.clone())
            }
        })
        .collect();

    let insert_set_arms: Vec<_> = entity_info
        .fields
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

    let update_set_arms: Vec<_> = entity_info
        .fields
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

    let all_columns: Vec<_> = entity_info.fields.iter().map(|f| f.column_name.as_str()).collect();
    let all_columns_str = all_columns.join(", ");

    let column_count = entity_info.fields.len();

    let is_nullable_arms: Vec<_> = entity_info
        .fields
        .iter()
        .map(|f| {
            let variant_name = &f.variant_name;
            let is_nullable = f.is_optional;
            quote! { Self::#variant_name => #is_nullable }
        })
        .collect();

    let is_primary_key_arms: Vec<_> = entity_info
        .fields
        .iter()
        .map(|f| {
            let variant_name = &f.variant_name;
            let is_pk = f.is_primary_key;
            quote! { Self::#variant_name => #is_pk }
        })
        .collect();

    let is_auto_increment_arms: Vec<_> = entity_info
        .fields
        .iter()
        .map(|f| {
            let variant_name = &f.variant_name;
            let is_auto = f.is_auto_increment;
            quote! { Self::#variant_name => #is_auto }
        })
        .collect();

    let is_unique_arms: Vec<_> = entity_info
        .fields
        .iter()
        .map(|f| {
            let variant_name = &f.variant_name;
            let is_unique = f.is_unique;
            quote! { Self::#variant_name => #is_unique }
        })
        .collect();

    let default_value_arms: Vec<_> = entity_info
        .fields
        .iter()
        .map(|f| {
            let variant_name = &f.variant_name;
            match &f.default_value {
                Some(val) => quote! { Self::#variant_name => Some(#val) },
                None => quote! { Self::#variant_name => None },
            }
        })
        .collect();

    let renamed_from_arms: Vec<_> = entity_info
        .fields
        .iter()
        .map(|f| {
            let variant_name = &f.variant_name;
            match &f.renamed_from {
                Some(old_name) => quote! { Self::#variant_name => Some(#old_name) },
                None => quote! { Self::#variant_name => None },
            }
        })
        .collect();

    let foreign_key_arms: Vec<_> = entity_info
        .fields
        .iter()
        .map(|f| {
            let variant_name = &f.variant_name;
            match &f.foreign_key {
                Some(fk) => {
                    let table_name = &fk.table_name;
                    let column_name = &fk.column_name;
                    let on_delete = fk.on_delete;
                    let on_update = fk.on_update;
                    quote! {
                        Self::#variant_name => Some(tursorm::ForeignKeyInfo {
                            table_name: String::from(#table_name),
                            column_name: String::from(#column_name),
                            on_delete: #on_delete,
                            on_update: #on_update,
                        })
                    }
                }
                None => quote! { Self::#variant_name => None },
            }
        })
        .collect();

    quote! {

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

            fn foreign_key(&self) -> Option<ForeignKeyInfo> {
                match self {
                    #(#foreign_key_arms),*
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
            fn from_row(row: &tursorm::Row) -> tursorm::Result<Self> {
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

        impl #entity_name {





            pub fn active_model() -> #active_model_name {
                #active_model_name::default()
            }
        }


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

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

fn to_pascal_case(ident: &Ident) -> Ident {
    let s = ident.to_string();
    let pascal: String = s
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect();
    Ident::new(&pascal, ident.span())
}

fn to_snake_case(ident: &Ident) -> String {
    let s = ident.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_lowercase().next().unwrap());
    }
    result
}

fn parse_references(refs: String) -> (String, String) {
    let parts: Vec<&str> = refs.splitn(2, '.').collect();
    match parts.as_slice() {
        [table, column] => (table.to_string(), column.to_string()),
        [table] => (table.to_string(), "id".to_string()),
        _ => panic!("Invalid references format: {}", refs),
    }
}
