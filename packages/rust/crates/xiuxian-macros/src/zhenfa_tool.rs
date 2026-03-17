use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse::Parser;
use syn::{Attribute, FnArg, Ident, ItemFn, LitStr, Meta, PatType, ReturnType};

#[derive(Default)]
struct ZhenfaToolAttr {
    name: Option<LitStr>,
    description: Option<LitStr>,
    tool_struct: Option<Ident>,
    mutation_scope: Option<LitStr>,
}

impl ZhenfaToolAttr {
    fn parse(attr: TokenStream) -> syn::Result<Self> {
        let mut parsed = Self::default();
        let parser = syn::meta::parser(|meta| {
            if meta.path.is_ident("name") {
                if parsed.name.is_some() {
                    return Err(meta.error("duplicate `name`"));
                }
                parsed.name = Some(meta.value()?.parse()?);
                return Ok(());
            }

            if meta.path.is_ident("description") {
                if parsed.description.is_some() {
                    return Err(meta.error("duplicate `description`"));
                }
                parsed.description = Some(meta.value()?.parse()?);
                return Ok(());
            }

            if meta.path.is_ident("tool_struct") {
                if parsed.tool_struct.is_some() {
                    return Err(meta.error("duplicate `tool_struct`"));
                }
                let lit: LitStr = meta.value()?.parse()?;
                parsed.tool_struct = Some(Ident::new(&lit.value(), lit.span()));
                return Ok(());
            }

            if meta.path.is_ident("mutation_scope") {
                if parsed.mutation_scope.is_some() {
                    return Err(meta.error("duplicate `mutation_scope`"));
                }
                parsed.mutation_scope = Some(meta.value()?.parse()?);
                return Ok(());
            }

            Err(meta.error(
                "unsupported key; expected `name`, `description`, `tool_struct`, or `mutation_scope`",
            ))
        });
        parser.parse2(attr.into())?;
        if parsed.name.is_none() {
            return Err(syn::Error::new(
                Span::call_site(),
                "`zhenfa_tool` requires `name = \"...\"`",
            ));
        }
        Ok(parsed)
    }
}

#[allow(clippy::too_many_lines)]
pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let config = match ZhenfaToolAttr::parse(attr) {
        Ok(config) => config,
        Err(error) => return error.to_compile_error().into(),
    };

    let function: ItemFn = match syn::parse(item) {
        Ok(function) => function,
        Err(error) => return error.to_compile_error().into(),
    };

    let is_async = function.sig.asyncness.is_some();

    if !function.sig.generics.params.is_empty() {
        return syn::Error::new_spanned(
            &function.sig.generics,
            "`zhenfa_tool` does not support generic functions",
        )
        .to_compile_error()
        .into();
    }

    if !matches!(&function.sig.output, ReturnType::Type(_, _)) {
        return syn::Error::new_spanned(
            &function.sig.output,
            "`zhenfa_tool` function must return `Result<String, ZhenfaError>`",
        )
        .to_compile_error()
        .into();
    }

    let inputs: Vec<&FnArg> = function.sig.inputs.iter().collect();
    if inputs.len() != 2 {
        return syn::Error::new_spanned(
            &function.sig.inputs,
            "`zhenfa_tool` function must accept exactly two arguments: `(&ZhenfaContext, Args)`",
        )
        .to_compile_error()
        .into();
    }

    let args_ty = match inputs[1] {
        FnArg::Typed(PatType { ty, .. }) => ty.as_ref(),
        arg @ FnArg::Receiver(_) => {
            return syn::Error::new_spanned(
                arg,
                "`zhenfa_tool` second argument must be a typed args struct",
            )
            .to_compile_error()
            .into();
        }
    };

    let fn_ident = &function.sig.ident;
    let vis = &function.vis;
    let tool_name = config
        .name
        .unwrap_or_else(|| LitStr::new("missing.name", Span::call_site()));
    let description = config.description.unwrap_or_else(|| {
        let fallback = extract_doc_summary(&function.attrs).unwrap_or_else(|| {
            format!(
                "Native zhenfa tool generated from `{}`.",
                function.sig.ident
            )
        });
        LitStr::new(&fallback, function.sig.ident.span())
    });
    let struct_ident = config
        .tool_struct
        .unwrap_or_else(|| default_tool_struct_ident(&function.sig.ident));
    let mutation_scope_impl = config.mutation_scope.map(|scope| {
        quote! {
            fn mutation_scope(
                &self,
                _ctx: &::xiuxian_zhenfa::ZhenfaContext,
                _args: &::xiuxian_zhenfa::serde_json::Value,
            ) -> ::core::option::Option<::std::string::String> {
                ::core::option::Option::Some(#scope.to_string())
            }
        }
    });

    // Generate the function call based on whether it's async or sync
    let fn_call = if is_async {
        quote! { #fn_ident(ctx, parsed_args).await }
    } else {
        quote! { #fn_ident(ctx, parsed_args) }
    };

    quote! {
        #function

        #[doc = #description]
        #[derive(Clone, Copy, Debug, Default)]
        #vis struct #struct_ident;

        #[::xiuxian_zhenfa::async_trait::async_trait]
        impl ::xiuxian_zhenfa::ZhenfaTool for #struct_ident {
            fn id(&self) -> &str {
                #tool_name
            }

            fn definition(&self) -> ::xiuxian_zhenfa::serde_json::Value {
                let schema = ::xiuxian_zhenfa::schemars::schema_for!(#args_ty);
                let parameters = ::xiuxian_zhenfa::serde_json::to_value(&schema)
                    .unwrap_or_else(|error| {
                        let _ = error;
                        ::xiuxian_zhenfa::serde_json::json!({
                            "type": "object",
                            "properties": {}
                        })
                    });
                ::xiuxian_zhenfa::serde_json::json!({
                    "name": self.id(),
                    "description": #description,
                    "parameters": parameters
                })
            }

            async fn call_native(
                &self,
                ctx: &::xiuxian_zhenfa::ZhenfaContext,
                args: ::xiuxian_zhenfa::serde_json::Value,
            ) -> ::core::result::Result<::std::string::String, ::xiuxian_zhenfa::ZhenfaError> {
                let parsed_args: #args_ty = ::xiuxian_zhenfa::serde_json::from_value(args)
                    .map_err(|error| {
                        ::xiuxian_zhenfa::ZhenfaError::invalid_arguments(format!(
                            "invalid {} params: {}",
                            #tool_name,
                            error
                        ))
                    })?;
                #fn_call
            }

            #mutation_scope_impl
        }
    }
    .into()
}

fn extract_doc_summary(attrs: &[Attribute]) -> Option<String> {
    let docs: Vec<String> = attrs
        .iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("doc") {
                return None;
            }
            let meta = attr.meta.clone();
            let Meta::NameValue(name_value) = meta else {
                return None;
            };
            let syn::Expr::Lit(expr_lit) = name_value.value else {
                return None;
            };
            let syn::Lit::Str(line) = expr_lit.lit else {
                return None;
            };
            let trimmed = line.value().trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
        .collect();
    if docs.is_empty() {
        None
    } else {
        Some(docs.join(" "))
    }
}

fn default_tool_struct_ident(fn_ident: &Ident) -> Ident {
    let mut result = String::new();
    for segment in fn_ident
        .to_string()
        .split('_')
        .filter(|segment| !segment.is_empty())
    {
        let mut chars = segment.chars();
        if let Some(first) = chars.next() {
            result.extend(first.to_uppercase());
            result.push_str(chars.as_str());
        }
    }
    if result.is_empty() {
        result.push_str("Generated");
    }
    result.push_str("Tool");
    Ident::new(&result, fn_ident.span())
}
