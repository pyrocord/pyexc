use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    FieldsNamed, FieldsUnnamed, Variant,
};
use syn::{Fields, ItemEnum, MetaNameValue};
use syn::{Ident, Lit, LitStr, Token};

struct Format {
    message: LitStr,
}

impl Parse for Format {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let message = input.parse::<LitStr>()?;
        Ok(Format { message })
    }
}

struct Inherits {
    module: Option<String>,
    exception: String,
}

struct Base {
    module: MetaNameValue,
    inherits: Option<String>,
}

impl Parse for Base {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let module = input.parse::<MetaNameValue>()?;
        let inherits = match input.parse::<Token![,]>() {
            Ok(_) => {
                match input.parse::<MetaNameValue>() {
                    Ok(mnv) => match mnv.lit {
                        Lit::Str(litstr) => Some(litstr.value()),
                        _ => panic!("`inherits` argument of `base` must be a string."),
                    },
                    Err(_) => panic!("expected `inherits` argument after comma."),
                }
            },
            Err(_) => None
        };

        Ok(Base { module, inherits })
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
    inherits_spec: &Inherits,
) -> proc_macro2::TokenStream {
    let variant_name = &variant.ident;
    let py_exception = &format_ident!("{}", inherits_spec.exception);
    let (variant_name, py_exc_name) = if *is_base {
        (enum_name, py_exception)
    } else {
        (variant_name, enum_name)
    };

    quote! {
        create_exception!(#module_name, #variant_name, #py_exc_name);
    }
}

fn impl_use_base_exc(inherits: &Inherits) -> proc_macro2::TokenStream {
    if let Some(module) = &inherits.module {
        let module_name = format_ident!("{}", module);
        let exception_name = format_ident!("{}", inherits.exception);
        quote! {
            use self::#module_name::#exception_name;
        }
    } else {
        quote! {}
    }
}

fn get_inherit_spec(spec: &Vec<String>) -> Inherits {
    let length = spec.len();
    if length == 0 || length > 2 {
        panic!("`inherits` argument of `base` must follow `MODULE.EXCEPTION` format.")
    }

    let [module, exception] = [&spec[0], &spec[1]];

    Inherits {
        exception: String::from(module),
        module: Some(String::from(exception)),
    }
}

fn get_default_inherit_spec() -> Inherits {
    Inherits {
        module: None,
        exception: String::from("PyException"),
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

    let (base_variant, base) = variants_with_base
        .nth(0)
        .expect("There must be one base exception");
    let base_count = variants_with_base.count();

    if base_count > 1 {
        panic!("There can only be one base exception.")
    }

    // Re-used as base exception's name
    let enum_name = item.ident;
    let module_name = format_ident!(
        "{}",
        match base.module.lit {
            Lit::Str(litstr) => litstr.value(),
            _ => panic!("`module` argument of `base` must be a string."),
        }
    );

    let inherits_spec: Inherits = base
        .inherits
        .map(|litstr| {
            let spec = litstr;
            let split: Vec<String> = spec.split('.').map(|item| String::from(item)).collect();
            get_inherit_spec(&split)
        })
        .unwrap_or_else(get_default_inherit_spec);

    for (variant, format_attribute) in variants_with_format {
        let variant_name = &variant.ident;
        let is_base = variant_name == &base_variant.ident;

        exception_formats.push(match &variant.fields {
            Fields::Unnamed(fields) => impl_unnamed_fields(
                &module_name,
                &enum_name,
                variant,
                fields,
                &format_attribute,
                &is_base,
            ),
            Fields::Named(fields) => impl_named_fields(
                &module_name,
                &enum_name,
                variant,
                fields,
                &format_attribute,
                &is_base,
            ),
            Fields::Unit => impl_unit(
                &module_name,
                &enum_name,
                variant,
                &format_attribute,
                &is_base,
            ),
        });

        python_exceptions.push(impl_create_exception(
            &module_name,
            &enum_name,
            variant,
            &is_base,
            &inherits_spec,
        ))
    }

    let base_exc_use = impl_use_base_exc(&inherits_spec);

    let tokens = quote! {
        mod #module_name {
            use pyo3::create_exception;
            use pyo3::exceptions::PyException;
            #base_exc_use

            #(#python_exceptions)*
        }

        impl std::convert::From<#enum_name> for pyo3::PyErr {
            fn from(err: #enum_name) -> pyo3::PyErr {
                match err {
                    #(#exception_formats),*
                }
            }
        }
    };

    tokens.into()
}
