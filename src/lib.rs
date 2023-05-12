// Copyright 2019-2023 Parity Technologies (UK) Ltd.
// This file is dual-licensed as Apache-2.0 or GPL-3.0.
// see LICENSE for license details.

extern crate proc_macro;

use std::str::FromStr;
use darling::{ast::NestedMeta, FromMeta};
use proc_macro::TokenStream;
use proc_macro_error::{abort_call_site, proc_macro_error};
use subxt_codegen::{utils::Uri, CodegenError};
use syn::parse_macro_input;
use proc_macro2::TokenStream as TokenStream2;
use codec::Decode;
use quote::{quote};
use syn::parse_quote;
use subxt_codegen::{ TypeGenerator };

#[derive(Clone, Debug)]
struct OuterAttribute(syn::Attribute);

impl syn::parse::Parse for OuterAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self(input.call(syn::Attribute::parse_outer)?[0].clone()))
    }
}

#[derive(Debug, FromMeta)]
struct RuntimeMetadataArgs {
    #[darling(default)]
    runtime_metadata_path: Option<String>,
    #[darling(default)]
    runtime_metadata_url: Option<String>,
}

// Note: docs for this are in the subxt library; don't add any here as they will be appended.
#[proc_macro_attribute]
#[proc_macro_error]
pub fn runtime_call(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(darling::Error::from(e).write_errors());
        }
    };
    let item_mod = parse_macro_input!(input as syn::ItemMod);
    let args = match RuntimeMetadataArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => return TokenStream::from(e.write_errors()),
    };

    match (args.runtime_metadata_path, args.runtime_metadata_url) {
        (Some(rest_of_path), None) => {
            let root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
            let root_path = std::path::Path::new(&root);
            let path = root_path.join(rest_of_path);
            generate_runtime_api_from_path(
                item_mod,
                path,
            )
            .map_or_else(|err| err.into_compile_error().into(), Into::into)
        }
        (None, Some(url_string)) => {
            let url = Uri::from_str(&url_string).unwrap_or_else(|_| {
                abort_call_site!("Cannot download metadata; invalid url: {}", url_string)
            });
            generate_runtime_api_from_url(
                item_mod,
                &url,
            )
            .map_or_else(|err| err.into_compile_error().into(), Into::into)
        }
        (None, None) => {
            abort_call_site!(
                "One of 'runtime_metadata_path' or 'runtime_metadata_url' must be provided"
            )
        }
        (Some(_), Some(_)) => {
            abort_call_site!(
                "Only one of 'runtime_metadata_path' or 'runtime_metadata_url' can be provided"
            )
        }
    }
}

fn generate_runtime_api_from_path(
    item_mod: syn::ItemMod,
    path: impl AsRef<std::path::Path>,
) -> Result<TokenStream2, CodegenError> {
    let to_err = |err| CodegenError::Io(path.as_ref().to_string_lossy().into(), err);

    let mut file = std::fs::File::open(&path).map_err(to_err)?;
    let mut bytes = Vec::new();
    use std::io::Read;
    file.read_to_end(&mut bytes).map_err(to_err)?;

    generate_runtime_api_from_bytes(
        item_mod,
        &bytes,
    )
}

fn generate_runtime_api_from_url(
    item_mod: syn::ItemMod,
    url: &Uri,
) -> Result<TokenStream2, CodegenError> {
    use subxt_codegen::utils::{ MetadataVersion, fetch_metadata_bytes_blocking };

    // Fetch latest unstable version, if that fails fall back to the latest stable.
    let bytes = match fetch_metadata_bytes_blocking(url, MetadataVersion::Unstable) {
        Ok(bytes) => bytes,
        Err(_) => fetch_metadata_bytes_blocking(url, MetadataVersion::Latest)?,
    };

    generate_runtime_api_from_bytes(
        item_mod,
        &bytes,
    )
}

fn generate_runtime_api_from_bytes(
    item_mod: syn::ItemMod,
    bytes: &[u8],
) -> Result<TokenStream2, CodegenError> {
    let metadata = frame_metadata::RuntimeMetadataPrefixed::decode(&mut &bytes[..])?;
    let types = match &metadata.1 {
        frame_metadata::RuntimeMetadata::V14(m) => &m.types,
        frame_metadata::RuntimeMetadata::V15(m) => &m.types,
        _ => panic!("This macro expects V14 (or compatible but unstable V15) metadata")
    };

    generate_runtime_types(item_mod, types)
}

fn generate_runtime_types(
    item_mod: syn::ItemMod,
    types: &scale_info::PortableRegistry
) -> Result<TokenStream2, CodegenError> {
    let item_mod_attrs = item_mod.attrs.clone();
    let mod_ident = &item_mod.ident;

    // This is a hack; we look for the "subxt" module inside the module we're generating here.
    // this relies on the module being generated being publically visible (and not eg in a function).
    // To avoid this, you could re-add `crate_path` as an arg to the macro, and expose this macro behind
    // a separate crate (like subxt does), importing and declaring the necessary types in that crate.
    let crate_path = subxt_codegen::CratePath::new(parse_quote!(crate::#mod_ident::subxt));

    let mut type_substitutes = subxt_codegen::TypeSubstitutes::new();
    let lsb_path: syn::Path = parse_quote!(crate::#mod_ident::subxt::utils::bits::Lsb0);
    let msb_path: syn::Path = parse_quote!(crate::#mod_ident::subxt::utils::bits::Msb0);

    type_substitutes.insert(
        parse_quote!(bitvec::order::Lsb0),
        lsb_path.try_into().expect("valid Lsb0 absolute path")
    ).expect("can insert Lsb0 substitute");
    type_substitutes.insert(
        parse_quote!(bitvec::order::Msb0),
        msb_path.try_into().expect("valid Msb0 absolute path")
    ).expect("can insert Msb0 substitute");

    let mut derives = subxt_codegen::DerivesRegistry::new();
    derives.extend_for_all([parse_quote!(codec::Encode)], []);

    let type_gen = TypeGenerator::new(
        types,
        "ty",
        type_substitutes,
        derives,
        crate_path,
        true,
    );
    let types_mod = type_gen.generate_types_mod()?;

    Ok(quote! {
        #( #item_mod_attrs )*
        #[allow(dead_code, unused_imports, non_camel_case_types)]
        #[allow(clippy::all)]
        pub mod #mod_ident {

            // This is a hack to provide the bits we need from external crates,
            // which are ordinarily exposed via the subxt crate (though we substitute
            // in BitVec over scale-bits types because the latter currently isn't no_std).
            // currently we need to import these dependencies with specific feature flags.
            // see above comment for how we could tidy this a bit.
            pub mod subxt {
                pub mod ext {
                    pub use codec;
                }
                pub mod utils {
                    pub mod bits {
                        pub use bitvec::order::{ Msb0, Lsb0 };
                        pub use bitvec::vec::BitVec as DecodedBits;
                    }
                }
            }

            #types_mod
        }
    })
}
