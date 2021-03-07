use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    FieldsUnnamed, FieldsNamed, Variant,
};
use syn::{Fields, ItemEnum, MetaNameValue};
use syn::{Ident, LitStr, Lit};

struct Format {
    message: LitStr,
}

impl Parse for Format {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let message = input.parse::<LitStr>()?;
        Ok(Format { message })
    }
}

struct Base {
    module: MetaNameValue,
}

impl Parse for Base {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let module = input.parse::<MetaNameValue>()?;
        Ok(Base { module })
    }
}

fn impl_unit(
    module_name: &Ident,
    enum_name: &Ident,
    variant: &Variant,
    format_attribute: &Format,
    is_base: &bool,
) -> proc_macro2::TokenStream {
    let variant_name = &variant.ident;
    let py_exc_name = if *is_base { enum_name } else { &variant.ident };
    let fmt = &format_attribute.message;

    quote! {
        #enum_name::#variant_name => self::#module_name::#py_exc_name::new_err(#fmt)
    }
}

fn impl_unnamed_fields(
    module_name: &Ident,
    enum_name: &Ident,
    variant: &Variant,
    fields: &FieldsUnnamed,
    format_attribute: &Format,
    is_base: &bool,
) -> proc_macro2::TokenStream {
    let variant_name = &variant.ident;
    let py_exc_name = if *is_base { enum_name } else { &variant.ident };
    let field_names: Vec<_> = fields
        .unnamed
        .iter()
        .enumerate()
        .map(|item| format_ident!("arg{}", item.0))
        .collect();
    let fmt = &format_attribute.message;

    quote! {
        #enum_name::#variant_name(#(#field_names),*) =>
            self::#module_name::#py_exc_name::new_err(
                format!(
                    #fmt,
                    #(#field_names),*
                )
            )
    }
}

fn impl_named_fields(
    module_name: &Ident,
    enum_name: &Ident,
    variant: &Variant,
    fields: &FieldsNamed,
    format_attribute: &Format,
    is_base: &bool,
) -> proc_macro2::TokenStream {
    let variant_name = &variant.ident;
    let py_exc_name = if *is_base { enum_name } else { &variant.ident };
    let field_names: Vec<_> = fields
        .named
        .iter()
        .map(|item| item.ident.as_ref().unwrap())
        .collect();
    let fmt = &format_attribute.message;

    quote! {
        #enum_name::#variant_name(#(#field_names),*) =>
            self::#module_name::#py_exc_name::new_err(
                format!(
                    #fmt,
                    #(#field_names = #field_names),*
                )
            )
    }
}

fn impl_create_exception(
    module_name: &Ident,
    enum_name: &Ident,
    variant: &Variant,
    is_base: &bool,
) -> proc_macro2::TokenStream {
    let variant_name = &variant.ident;
    let py_exception = &format_ident!("PyException");
    let (variant_name, py_exc_name) = if *is_base { (enum_name, py_exception) } else { (variant_name, enum_name) };

    quote! {
        create_exception!(#module_name, #variant_name, #py_exc_name);
    }
}

#[proc_macro_derive(PythonException, attributes(format, base))]
pub fn pyexc_macro(input: TokenStream) -> TokenStream {
    let item: ItemEnum = syn::parse(input).expect("Route can only be derived for enums.");
    let mut exception_formats = Vec::new();
    let mut python_exceptions = Vec::new();

    if item.variants.is_empty() {
        panic!("Cannot derive `Routes` on empty enum")
    }

    let variants_with_format = item.variants.iter().filter_map(|variant| {
        variant
            .attrs
            .iter()
            .find(|attr| attr.path.is_ident("format"))
            .map(|attr| {
                let format = attr
                    .parse_args::<Format>()
                    .expect("Invalid syntax for `format` attribute");

                (variant, format)
            })
    });

    let mut variants_with_base = item.variants.iter().filter_map(|variant| {
        variant
            .attrs
            .iter()
            .find(|attr| attr.path.is_ident("base"))
            .map(|attr| {
                let base = attr
                    .parse_args::<Base>()
                    .expect("Invalid syntax for `base` attribute");

                (variant, base)
            })
    });

    let first_base = variants_with_base.nth(0);
    let base_count = variants_with_base.count();

    if base_count > 1 {
        panic!("There can only be one base exception.")
    }

    let (base_variant, base) = match first_base {
        Some(item) => item,
        None => panic!("There must be one base exception."),
    };

    // Re-used as base exception's name
    let enum_name = item.ident;
    let module_name = format_ident!("{}", match base.module.lit {
        Lit::Str(litstr) => litstr.value(),
        _ => panic!("`module` argument of `base` must be a string.")
    });

    for (variant, format_attribute) in variants_with_format {
        let variant_name = &variant.ident;
        let is_base = variant_name == &base_variant.ident;

        exception_formats.push(match &variant.fields {
            Fields::Unnamed(fields) => {
                impl_unnamed_fields(&module_name, &enum_name, variant, fields, &format_attribute, &is_base)
            }
            Fields::Named(fields) => {
                impl_named_fields(&module_name, &enum_name, variant, fields, &format_attribute, &is_base)
            }
            Fields::Unit => impl_unit(&module_name, &enum_name, variant, &format_attribute, &is_base),
        });

        python_exceptions.push(impl_create_exception(&module_name, &enum_name, variant, &is_base))
    }

    let tokens = quote! {
        use pyo3::PyErr;


        mod #module_name {
            use pyo3::create_exception;
            use pyo3::exceptions::PyException;

            #(#python_exceptions)*
        }

        impl std::convert::From<#enum_name> for PyErr {
            fn from(err: #enum_name) -> PyErr {
                match err {
                    #(#exception_formats),*
                }
            }
        }
    };

    tokens.into()
}