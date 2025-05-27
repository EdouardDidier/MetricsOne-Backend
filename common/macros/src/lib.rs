use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, DeriveInput, Expr, ExprAssign, ExprLit, Fields, Lit, parse_macro_input};

#[proc_macro_derive(SqlNames, attributes(sql_names))]
pub fn derive_sql_names(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Check if the derive is applied to a Struct, and that it has named fields
    if let syn::Data::Struct(ref data) = input.data {
        if let Fields::Named(ref fields) = data.fields {
            let struct_name = input.ident;

            let table_name =
                parse_struct_attrs(&input.attrs).unwrap_or(struct_name.to_string().to_lowercase());

            let mut skip = false;
            let mut field_vals = Vec::new();

            for field in fields.named.iter() {
                for attr in &field.attrs {
                    if attr.path().is_ident("sql_names") {
                        let _ = attr.parse_nested_meta(|meta| {
                            if meta.path.is_ident("skip") {
                                skip = true;
                                return Ok(());
                            }

                            Err(meta.error("Unrecognized `sql_names` attribute"))
                        });

                        if skip {
                            break;
                        }
                    }
                }

                if skip {
                    continue;
                }

                let field_name = field.ident.as_ref().unwrap();
                let sql_field = format!("{}", field_name.to_string().to_lowercase());

                field_vals.push(quote!(#sql_field));
            }

            // Get the number of fields for the array constructor
            let field_len = field_vals.len();

            // Implementation of the constants for the dervived struct
            return TokenStream::from(quote!(
                impl #struct_name {
                    pub const SQL_FIELDS: [&str; #field_len] = [#(#field_vals),*];
                    pub const SQL_TABLE: &str = #table_name;
                }
            ));
        }
    }

    TokenStream::from(
        syn::Error::new(
            input.ident.span(),
            "Only structs with named fields can derive `SqlNames`",
        )
        .to_compile_error(),
    )
}

fn parse_struct_attrs(attrs: &Vec<Attribute>) -> Option<String> {
    let mut res = None;

    for attr in attrs {
        if attr.path().is_ident("sql_names") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("table_name") {
                    let Ok(expr) = attr.parse_args::<ExprAssign>() else {
                        return Err(
                            meta.error("Expected an assing expression for attribute `table_name`")
                        );
                    };

                    match &*expr.right {
                        Expr::Lit(ExprLit {
                            lit: Lit::Str(s), ..
                        }) => {
                            res = Some(s.value().to_string());
                            return Ok(());
                        }
                        _ => {
                            return Err(
                                meta.error("Expected a string literal for attribute `table_name`")
                            );
                        }
                    }
                }

                Err(meta.error("Unrecognized `sql_names` attribute"))
            });
        }
    }

    res
}
