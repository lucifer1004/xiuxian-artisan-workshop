use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Error, Expr, Lit, MetaNameValue, Result as SynResult, Token, parse_macro_input};

struct XiuxianConfigArgs {
    namespace: String,
    internal_path: Option<String>,
    orphan_file: Option<String>,
    array_merge: Option<String>,
}

struct XiuxianConfigResolvedArgs {
    namespace: String,
    internal_path: String,
    orphan_file: String,
    array_merge_strategy: proc_macro2::TokenStream,
}

impl Parse for XiuxianConfigArgs {
    fn parse(input: ParseStream<'_>) -> SynResult<Self> {
        let mut namespace: Option<String> = None;
        let mut internal_path: Option<String> = None;
        let mut orphan_file: Option<String> = None;
        let mut array_merge: Option<String> = None;

        while !input.is_empty() {
            let meta: MetaNameValue = input.parse()?;
            let Some(ident) = meta.path.get_ident() else {
                return Err(Error::new_spanned(meta.path, "expected identifier key"));
            };
            let value = parse_string_literal(meta.value)?;
            match ident.to_string().as_str() {
                "namespace" => namespace = Some(value),
                "internal_path" => internal_path = Some(value),
                "orphan_file" => orphan_file = Some(value),
                "array_merge" => array_merge = Some(value),
                _ => {
                    return Err(Error::new_spanned(
                        ident,
                        "unsupported xiuxian_config argument; expected `namespace`, `internal_path`, `orphan_file`, or `array_merge`",
                    ));
                }
            }
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        let Some(namespace) = namespace else {
            return Err(Error::new(
                proc_macro2::Span::call_site(),
                "missing required argument `namespace = \"...\"`",
            ));
        };

        Ok(Self {
            namespace,
            internal_path,
            orphan_file,
            array_merge,
        })
    }
}

fn parse_string_literal(expr: Expr) -> SynResult<String> {
    match expr {
        Expr::Lit(expr_lit) => match expr_lit.lit {
            Lit::Str(value) => Ok(value.value()),
            _ => Err(Error::new_spanned(
                expr_lit,
                "expected string literal value",
            )),
        },
        other => Err(Error::new_spanned(other, "expected string literal value")),
    }
}

fn resolve_array_merge_strategy(value: &str) -> SynResult<proc_macro2::TokenStream> {
    match value {
        "overwrite" => Ok(quote!(xiuxian_config_core::ArrayMergeStrategy::Overwrite)),
        "append" => Ok(quote!(xiuxian_config_core::ArrayMergeStrategy::Append)),
        _ => Err(Error::new(
            proc_macro2::Span::call_site(),
            "invalid `array_merge`; expected \"overwrite\" or \"append\"",
        )),
    }
}

fn resolve_xiuxian_config_args(args: XiuxianConfigArgs) -> SynResult<XiuxianConfigResolvedArgs> {
    let namespace = args.namespace;
    let internal_path = args
        .internal_path
        .unwrap_or_else(|| format!("resources/config/{namespace}.toml"));
    let orphan_file = args
        .orphan_file
        .unwrap_or_else(|| format!("{namespace}.toml"));
    let array_merge = args.array_merge.unwrap_or_else(|| "overwrite".to_string());

    Ok(XiuxianConfigResolvedArgs {
        namespace,
        internal_path,
        orphan_file,
        array_merge_strategy: resolve_array_merge_strategy(array_merge.as_str())?,
    })
}

fn generate_spec_helpers(
    namespace: &str,
    internal_path: &str,
    orphan_file: &str,
    array_merge_strategy: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        fn __xiuxian_config_namespace() -> &'static str {
            #namespace
        }

        fn __xiuxian_config_embedded_toml() -> &'static str {
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", #internal_path))
        }

        fn __xiuxian_config_spec() -> xiuxian_config_core::ConfigCascadeSpec<'static> {
            xiuxian_config_core::ConfigCascadeSpec::new(
                Self::__xiuxian_config_namespace(),
                Self::__xiuxian_config_embedded_toml(),
                #orphan_file
            )
            .with_array_merge_strategy(#array_merge_strategy)
        }
    }
}

fn generate_api_key_policy_helpers() -> proc_macro2::TokenStream {
    quote! {
        fn __xiuxian_config_is_env_name(value: &str) -> bool {
            let mut chars = value.chars();
            let Some(first) = chars.next() else {
                return false;
            };
            if !(first == '_' || first.is_ascii_uppercase()) {
                return false;
            }
            chars.all(|character| character == '_' || character.is_ascii_uppercase() || character.is_ascii_digit())
        }

        fn __xiuxian_config_is_api_key_env_reference(value: &str) -> bool {
            let trimmed = value.trim();
            if let Some(env_name) = trimmed.strip_prefix("env:") {
                return Self::__xiuxian_config_is_env_name(env_name.trim());
            }
            if trimmed.starts_with("${") && trimmed.ends_with('}') && trimmed.len() > 3 {
                let env_name = &trimmed[2..trimmed.len() - 1];
                return Self::__xiuxian_config_is_env_name(env_name.trim());
            }
            Self::__xiuxian_config_is_env_name(trimmed)
        }

        fn __xiuxian_config_collect_api_key_violations(
            value: &toml::Value,
            path: &str,
            violations: &mut Vec<String>,
        ) {
            match value {
                toml::Value::Table(table) => {
                    for (key, nested) in table {
                        let next_path = if path.is_empty() {
                            key.to_string()
                        } else {
                            format!("{path}.{key}")
                        };
                        if key == "api_key" {
                            match nested.as_str() {
                                Some(raw) if Self::__xiuxian_config_is_api_key_env_reference(raw) => {}
                                Some(_) => violations.push(next_path),
                                None => violations.push(next_path),
                            }
                        } else {
                            Self::__xiuxian_config_collect_api_key_violations(
                                nested,
                                next_path.as_str(),
                                violations,
                            );
                        }
                    }
                }
                toml::Value::Array(items) => {
                    for (index, nested) in items.iter().enumerate() {
                        let next_path = format!("{path}[{index}]");
                        Self::__xiuxian_config_collect_api_key_violations(
                            nested,
                            next_path.as_str(),
                            violations,
                        );
                    }
                }
                _ => {}
            }
        }

        fn __xiuxian_config_validate_api_key_policy(value: &toml::Value) -> Result<(), String> {
            let mut violations = Vec::new();
            Self::__xiuxian_config_collect_api_key_violations(value, "", &mut violations);
            if violations.is_empty() {
                return Ok(());
            }
            violations.sort();
            violations.dedup();
            let joined = violations.join(", ");
            Err(format!(
                "Plaintext `api_key` values are forbidden in namespace [{}]. \
    Only environment-variable references are allowed (e.g. `OPENAI_API_KEY`, `env:OPENAI_API_KEY`, `${{OPENAI_API_KEY}}`). \
    Invalid path(s): {joined}",
                Self::__xiuxian_config_namespace()
            ))
        }
    }
}

fn generate_loading_helpers() -> proc_macro2::TokenStream {
    quote! {
        /// Return merged TOML value from embedded defaults and cascading overrides.
        ///
        /// # Errors
        ///
        /// Returns an error when embedded/default TOML is invalid, when
        /// conflict enforcement fails, or when one override file cannot be
        /// parsed.
        pub(crate) fn __xiuxian_config_merged_value() -> Result<toml::Value, String> {
            let merged = xiuxian_config_core::resolve_and_merge_toml(Self::__xiuxian_config_spec())
                .map_err(|error| error.to_string())?;
            Self::__xiuxian_config_validate_api_key_policy(&merged)?;
            Ok(merged)
        }

        /// Return merged TOML value from embedded defaults and cascading overrides
        /// with explicit path roots.
        ///
        /// # Errors
        ///
        /// Returns an error when embedded/default TOML is invalid, when
        /// conflict enforcement fails, or when one override file cannot be
        /// parsed.
        pub(crate) fn __xiuxian_config_merged_value_with_paths(
            project_root: Option<&std::path::Path>,
            config_home: Option<&std::path::Path>,
        ) -> Result<toml::Value, String> {
            let merged = xiuxian_config_core::resolve_and_merge_toml_with_paths(
                Self::__xiuxian_config_spec(),
                project_root,
                config_home,
            )
            .map_err(|error| error.to_string())?;
            Self::__xiuxian_config_validate_api_key_policy(&merged)?;
            Ok(merged)
        }

        /// Load configuration from embedded defaults and cascading overrides.
        ///
        /// # Errors
        ///
        /// Returns an error when embedded/default TOML is invalid, when
        /// conflict enforcement fails, or when merged TOML cannot deserialize
        /// into the target config struct.
        pub fn load() -> Result<Self, String>
        where
            Self: serde::de::DeserializeOwned,
        {
            let merged = Self::__xiuxian_config_merged_value()?;
            merged.try_into().map_err(|error| {
                format!(
                    "failed to deserialize merged config for [{}]: {error}",
                    Self::__xiuxian_config_namespace()
                )
            })
        }

        /// Load configuration from embedded defaults and cascading overrides
        /// with explicit path roots.
        ///
        /// # Errors
        ///
        /// Returns an error when embedded/default TOML is invalid, when
        /// conflict enforcement fails, or when merged TOML cannot deserialize
        /// into the target config struct.
        pub fn load_with_paths(
            project_root: Option<&std::path::Path>,
            config_home: Option<&std::path::Path>,
        ) -> Result<Self, String>
        where
            Self: serde::de::DeserializeOwned,
        {
            let merged =
                Self::__xiuxian_config_merged_value_with_paths(project_root, config_home)?;
            merged.try_into().map_err(|error| {
                format!(
                    "failed to deserialize merged config for [{}]: {error}",
                    Self::__xiuxian_config_namespace()
                )
            })
        }
    }
}

fn generate_xiuxian_config_impl_tokens(
    input_struct: &syn::ItemStruct,
    resolved: &XiuxianConfigResolvedArgs,
) -> proc_macro2::TokenStream {
    let struct_ident = &input_struct.ident;
    let spec_helpers = generate_spec_helpers(
        resolved.namespace.as_str(),
        resolved.internal_path.as_str(),
        resolved.orphan_file.as_str(),
        &resolved.array_merge_strategy,
    );
    let api_key_helpers = generate_api_key_policy_helpers();
    let loading_helpers = generate_loading_helpers();

    quote! {
        #input_struct

        impl #struct_ident {
            #spec_helpers
            #api_key_helpers
            #loading_helpers
        }
    }
}

pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as XiuxianConfigArgs);
    let input_struct = parse_macro_input!(item as syn::ItemStruct);
    let resolved_args = match resolve_xiuxian_config_args(args) {
        Ok(resolved) => resolved,
        Err(error) => return error.to_compile_error().into(),
    };

    generate_xiuxian_config_impl_tokens(&input_struct, &resolved_args).into()
}
