#![feature(vec_into_raw_parts)]
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, LitStr, Token, parse::Parse, parse_macro_input};

struct ElementArgs {
    name: LitStr,
    blurb: LitStr,
}

impl Parse for ElementArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut blurb = None;

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: LitStr = input.parse()?;

            match &*key.to_string() {
                "name" => name = Some(value),
                "blurb" => blurb = Some(value),
                _ => return Err(syn::Error::new_spanned(key, "unsupported property")),
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            name: name.ok_or_else(|| input.error("missing `name`"))?,
            blurb: blurb.ok_or_else(|| input.error("missing `blurb`"))?,
        })
    }
}

#[proc_macro_attribute]
pub fn transform_element(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);

    let args = parse_macro_input!(attr as ElementArgs);
    let el_name = args.name.value();
    let blurb = args.blurb.value();

    let struct_name = &ast.ident;
    let register_fn = format_ident!("{}_register", el_name);
    let init_fn = format_ident!("{}_init", el_name);
    let transform_fn = format_ident!("{}_transform", el_name);
    let destroy_fn = format_ident!("{}_destroy", el_name);
    let api_fn = format_ident!("{}_get_api", el_name);
    let get_property_descriptions_fn = format_ident!("{}_get_property_descriptions", el_name);
    let recv_message_fn = format_ident!("{}_recv_message", el_name);
    let post_configuration_messages_fn = format_ident!("{}_post_configuration_messages", el_name);

    let g = quote! {
        #ast

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #init_fn(config: *const u8, len: usize) -> *mut ::std::ffi::c_void {
            if config.is_null() {
                return ::std::ptr::null_mut();
            }
            let config = unsafe { ::std::str::from_raw_parts(config, len) };

            let properties = match ::physim_core::plugin::deps::serde_json::from_str(config){
                Ok(properties) => properties,
                Err(_) => return ::std::ptr::null_mut(),
            };

            match ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe( || {
                Box::new(#struct_name::new( properties ) )
            })) {
                Ok(el) => {
                    Box::into_raw(el) as *mut ::std::ffi::c_void
                }
                Err(_) => {
                    eprintln!(
                        "Problem encountered in the {} element's new method. Aborting",
                        #el_name
                    );
                    ::std::process::abort();
                }
            }

        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #transform_fn(obj: *const ::std::ffi::c_void, state: *const Entity, state_len: usize, acceleration: *mut Acceleration, acceleration_len: usize) {
            let el: & #struct_name = unsafe { &*(obj as *const #struct_name) };
            let s =  unsafe { ::std::slice::from_raw_parts(state, state_len) };
            let n =  unsafe {  ::std::slice::from_raw_parts_mut(acceleration, acceleration_len) };
            if let Err(_) = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| { el.transform(s, n)})) {
                eprintln!("Problem encountered in the {} element's transform method. Aborting", #el_name);
                ::std::process::abort();
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #destroy_fn(obj: *mut ::std::ffi::c_void) {
            if obj.is_null() {
                return;
            }

            let result = ::std::panic::catch_unwind(|| {
                drop(Box::from_raw(obj as *mut #struct_name));
            });

            if result.is_err() {
                eprintln!("Problem encountered in the {} element's drop method. Aborting", #el_name);
                ::std::process::abort();
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #api_fn() -> *const ::physim_core::plugin::transform::TransformElementAPI {
            Box::into_raw(Box::new(::physim_core::plugin::transform::TransformElementAPI {
                init: #init_fn,
                transform: #transform_fn,
                destroy: #destroy_fn,
                get_property_descriptions: #get_property_descriptions_fn,
                recv_message: #recv_message_fn,
                post_configuration_messages: #post_configuration_messages_fn,
            }))
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn #register_fn(alloc: ::physim_core::plugin::RustStringAllocFn) -> ::physim_core::plugin::ElementMetaFFI {
            // Create CStrings to get proper *const c_char pointers
            let el_name = ::std::ffi::CString::new(#el_name.replace("\0", "")).expect("Failed to make CString");
            let pkg_name = ::std::ffi::CString::new(env!("CARGO_PKG_NAME").replace("\0", "")).expect("Failed to make CString");
            let pkg_version = ::std::ffi::CString::new(env!("CARGO_PKG_VERSION").replace("\0", "")).expect("Failed to make CString");
            let pkg_license = ::std::ffi::CString::new(env!("CARGO_PKG_LICENSE").replace("\0", "")).expect("Failed to make CString");
            let pkg_authors = ::std::ffi::CString::new(env!("CARGO_PKG_AUTHORS").replace("\0", "")).expect("Failed to make CString");
            let blurb = ::std::ffi::CString::new(#blurb.replace("\0", "")).expect("Failed to make CString");
            let pkg_repo = ::std::ffi::CString::new(env!("CARGO_PKG_REPOSITORY").replace("\0", "")).expect("Failed to make CString");

            ::physim_core::plugin::ElementMetaFFI {
                kind: ::physim_core::plugin::ElementKind::Transform,
                name: alloc(el_name.as_ptr()),
                plugin: alloc(pkg_name.as_ptr()),
                version: alloc(pkg_version.as_ptr()),
                license: alloc(pkg_license.as_ptr()),
                author: alloc(pkg_authors.as_ptr()),
                blurb: alloc(blurb.as_ptr()),
                repo: alloc(pkg_repo.as_ptr()),
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #get_property_descriptions_fn(obj: *mut ::std::ffi::c_void, alloc: ::physim_core::plugin::RustStringAllocFn) -> *mut ::std::ffi::c_char {
            if obj.is_null() {return ::std::ptr::null_mut()};
            let el: &mut #struct_name = unsafe { &mut *(obj as *mut #struct_name) };

            match ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe( || {
                let properties = el.get_property_descriptions();
                ::physim_core::plugin::deps::serde_json::to_string(&properties)
            })) {
                    Ok(Ok(s)) => {
                    // Successful JSON serialization
                    let c_s = ::std::ffi::CString::new(s.replace("\0", "")).expect("Failed to make CString");
                    alloc(c_s.as_ptr())
                }
                Ok(Err(_)) => {
                    // Serialization failed
                    ::std::ptr::null_mut()
                }
                Err(_) => {
                    eprintln!(
                        "Panic encountered in the {} element's get_property_descriptions method.",
                        #el_name
                    );
                    ::std::ptr::null_mut()
                }
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #recv_message_fn(obj: *mut ::std::ffi::c_void, msg: *const ::physim_core::messages::CMessage) {
            if obj.is_null() {return };
            let el: &mut #struct_name = unsafe { &mut *(obj as *mut #struct_name) };
            let msg = unsafe{::physim_core::messages::Message::from_c_ptr(msg)};

            if let Err(_) = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| { el.recv_message(&msg)})) {
                eprintln!("Problem encountered in the {} element's recv_message method. Aborting", #el_name);
                ::std::process::abort();
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #post_configuration_messages_fn(obj: *mut ::std::ffi::c_void) {
            if obj.is_null() {return };
            let el: &mut #struct_name = unsafe { &mut *(obj as *mut #struct_name) };
            if let Err(_) = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| { el.post_configuration_messages();})) {
                eprintln!("Problem encountered in the {} element's post_configuration_messages method. Aborting", #el_name);
                ::std::process::abort();
            }
        }
    };
    g.into()
}

#[proc_macro_attribute]
pub fn render_element(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);

    let args = parse_macro_input!(attr as ElementArgs);
    let el_name = args.name.value();
    let blurb = args.blurb.value();
    let name = &ast.ident;
    let create_element = format_ident!("{}_create_element", el_name);
    let register_fn = format_ident!("{}_register", el_name);

    let g = quote! {
        #ast

        #[unsafe(no_mangle)]
        fn #create_element(properties: HashMap<String, Value>) -> Box<dyn ::physim_core::plugin::render::RenderElement> {
            #name::create_element(properties)
        }

                #[unsafe(no_mangle)]
        unsafe extern "C" fn #register_fn(alloc: extern "C" fn(*const ::std::ffi::c_char) -> *mut ::std::ffi::c_char) -> ::physim_core::plugin::ElementMetaFFI {
            // Create CStrings to get proper *const c_char pointers
            let el_name = ::std::ffi::CString::new(#el_name.replace("\0", "")).expect("Failed to make CString");
            let pkg_name = ::std::ffi::CString::new(env!("CARGO_PKG_NAME").replace("\0", "")).expect("Failed to make CString");
            let pkg_version = ::std::ffi::CString::new(env!("CARGO_PKG_VERSION").replace("\0", "")).expect("Failed to make CString");
            let pkg_license = ::std::ffi::CString::new(env!("CARGO_PKG_LICENSE").replace("\0", "")).expect("Failed to make CString");
            let pkg_authors = ::std::ffi::CString::new(env!("CARGO_PKG_AUTHORS").replace("\0", "")).expect("Failed to make CString");
            let blurb = ::std::ffi::CString::new(#blurb.replace("\0", "")).expect("Failed to make CString");
            let pkg_repo = ::std::ffi::CString::new(env!("CARGO_PKG_REPOSITORY").replace("\0", "")).expect("Failed to make CString");

            ::physim_core::plugin::ElementMetaFFI {
                kind: ::physim_core::plugin::ElementKind::Render,
                name: alloc(el_name.as_ptr()),
                plugin: alloc(pkg_name.as_ptr()),
                version: alloc(pkg_version.as_ptr()),
                license: alloc(pkg_license.as_ptr()),
                author: alloc(pkg_authors.as_ptr()),
                blurb: alloc(blurb.as_ptr()),
                repo: alloc(pkg_repo.as_ptr()),
            }
        }
    };
    g.into()
}

#[proc_macro_attribute]
pub fn initialise_state_element(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);

    let args = parse_macro_input!(attr as ElementArgs);
    let el_name = args.name.value();
    let blurb = args.blurb.value();
    let name = &ast.ident;
    let create_element = format_ident!("{}_create_element", el_name);
    let register_fn = format_ident!("{}_register", el_name);

    let g = quote! {
        #ast

        #[unsafe(no_mangle)]
        fn #create_element(properties: ::std::collections::HashMap<String, ::physim_core::plugin::deps::serde_json::Value>) -> Box<dyn ::physim_core::plugin::generator::GeneratorElement> {
            #name::create_element(properties)
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn #register_fn(alloc: extern "C" fn(*const ::std::ffi::c_char) -> *mut ::std::ffi::c_char) -> ::physim_core::plugin::ElementMetaFFI {
            // Create CStrings to get proper *const c_char pointers
            let el_name = ::std::ffi::CString::new(#el_name.replace("\0", "")).expect("Failed to make CString");
            let pkg_name = ::std::ffi::CString::new(env!("CARGO_PKG_NAME").replace("\0", "")).expect("Failed to make CString");
            let pkg_version = ::std::ffi::CString::new(env!("CARGO_PKG_VERSION").replace("\0", "")).expect("Failed to make CString");
            let pkg_license = ::std::ffi::CString::new(env!("CARGO_PKG_LICENSE").replace("\0", "")).expect("Failed to make CString");
            let pkg_authors = ::std::ffi::CString::new(env!("CARGO_PKG_AUTHORS").replace("\0", "")).expect("Failed to make CString");
            let blurb = ::std::ffi::CString::new(#blurb.replace("\0", "")).expect("Failed to make CString");
            let pkg_repo = ::std::ffi::CString::new(env!("CARGO_PKG_REPOSITORY").replace("\0", "")).expect("Failed to make CString");

            ::physim_core::plugin::ElementMetaFFI {
                kind: ::physim_core::plugin::ElementKind::Initialiser,
                name: alloc(el_name.as_ptr()),
                plugin: alloc(pkg_name.as_ptr()),
                version: alloc(pkg_version.as_ptr()),
                license: alloc(pkg_license.as_ptr()),
                author: alloc(pkg_authors.as_ptr()),
                blurb: alloc(blurb.as_ptr()),
                repo: alloc(pkg_repo.as_ptr()),
            }
        }
    };
    g.into()
}

#[proc_macro_attribute]
pub fn synth_element(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);

    let args = parse_macro_input!(attr as ElementArgs);
    let el_name = args.name.value();
    let blurb = args.blurb.value();
    let name = &ast.ident;
    let create_element = format_ident!("{}_create_element", el_name);
    let register_fn = format_ident!("{}_register", el_name);

    let g = quote! {
        #ast

        #[unsafe(no_mangle)]
        fn #create_element(properties: ::std::collections::HashMap<String, ::physim_core::plugin::deps::serde_json::Value>) -> Box<dyn ::physim_core::plugin::generator::GeneratorElement> {
            #name::create_element(properties)
        }

                #[unsafe(no_mangle)]
        unsafe extern "C" fn #register_fn(alloc: extern "C" fn(*const ::std::ffi::c_char) -> *mut ::std::ffi::c_char) -> ::physim_core::plugin::ElementMetaFFI {
            // Create CStrings to get proper *const c_char pointers
            let el_name = ::std::ffi::CString::new(#el_name.replace("\0", "")).expect("Failed to make CString");
            let pkg_name = ::std::ffi::CString::new(env!("CARGO_PKG_NAME").replace("\0", "")).expect("Failed to make CString");
            let pkg_version = ::std::ffi::CString::new(env!("CARGO_PKG_VERSION").replace("\0", "")).expect("Failed to make CString");
            let pkg_license = ::std::ffi::CString::new(env!("CARGO_PKG_LICENSE").replace("\0", "")).expect("Failed to make CString");
            let pkg_authors = ::std::ffi::CString::new(env!("CARGO_PKG_AUTHORS").replace("\0", "")).expect("Failed to make CString");
            let blurb = ::std::ffi::CString::new(#blurb.replace("\0", "")).expect("Failed to make CString");
            let pkg_repo = ::std::ffi::CString::new(env!("CARGO_PKG_REPOSITORY").replace("\0", "")).expect("Failed to make CString");

            ::physim_core::plugin::ElementMetaFFI {
                kind: ::physim_core::plugin::ElementKind::Synth,
                name: alloc(el_name.as_ptr()),
                plugin: alloc(pkg_name.as_ptr()),
                version: alloc(pkg_version.as_ptr()),
                license: alloc(pkg_license.as_ptr()),
                author: alloc(pkg_authors.as_ptr()),
                blurb: alloc(blurb.as_ptr()),
                repo: alloc(pkg_repo.as_ptr()),
            }
        }
    };
    g.into()
}

#[proc_macro_attribute]
pub fn transmute_element(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);

    let args = parse_macro_input!(attr as ElementArgs);
    let el_name = args.name.value();
    let blurb = args.blurb.value();
    let name = &ast.ident;
    let create_element = format_ident!("{}_create_element", el_name);
    let register_fn = format_ident!("{}_register", el_name);

    let g = quote! {
        #ast

        #[unsafe(no_mangle)]
        fn #create_element(properties: ::std::collections::HashMap<String, ::physim_core::plugin::deps::serde_json::Value>) -> Box<dyn ::physim_core::plugin::transmute::TransmuteElement> {
            #name::create_element(properties)
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn #register_fn(alloc: extern "C" fn(*const ::std::ffi::c_char) -> *mut ::std::ffi::c_char) -> ::physim_core::plugin::ElementMetaFFI {
            // Create CStrings to get proper *const c_char pointers
            let el_name = ::std::ffi::CString::new(#el_name.replace("\0", "")).expect("Failed to make CString");
            let pkg_name = ::std::ffi::CString::new(env!("CARGO_PKG_NAME").replace("\0", "")).expect("Failed to make CString");
            let pkg_version = ::std::ffi::CString::new(env!("CARGO_PKG_VERSION").replace("\0", "")).expect("Failed to make CString");
            let pkg_license = ::std::ffi::CString::new(env!("CARGO_PKG_LICENSE").replace("\0", "")).expect("Failed to make CString");
            let pkg_authors = ::std::ffi::CString::new(env!("CARGO_PKG_AUTHORS").replace("\0", "")).expect("Failed to make CString");
            let blurb = ::std::ffi::CString::new(#blurb.replace("\0", "")).expect("Failed to make CString");
            let pkg_repo = ::std::ffi::CString::new(env!("CARGO_PKG_REPOSITORY").replace("\0", "")).expect("Failed to make CString");

            ::physim_core::plugin::ElementMetaFFI {
                kind: ::physim_core::plugin::ElementKind::Transmute,
                name: alloc(el_name.as_ptr()),
                plugin: alloc(pkg_name.as_ptr()),
                version: alloc(pkg_version.as_ptr()),
                license: alloc(pkg_license.as_ptr()),
                author: alloc(pkg_authors.as_ptr()),
                blurb: alloc(blurb.as_ptr()),
                repo: alloc(pkg_repo.as_ptr()),
            }
        }
    };
    g.into()
}

#[proc_macro_attribute]
pub fn integrator_element(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);

    let args = parse_macro_input!(attr as ElementArgs);
    let el_name = args.name.value();
    let blurb = args.blurb.value();
    let name = &ast.ident;
    let create_element = format_ident!("{}_create_element", el_name);
    let register_fn = format_ident!("{}_register", el_name);

    let g = quote! {
        #ast

        #[unsafe(no_mangle)]
        fn #create_element(properties: ::std::collections::HashMap<String, ::physim_core::plugin::deps::serde_json::Value>) -> Box<dyn ::physim_core::plugin::integrator::IntegratorElement> {
            #name::create_element(properties)
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn #register_fn(alloc: extern "C" fn(*const ::std::ffi::c_char) -> *mut ::std::ffi::c_char) -> ::physim_core::plugin::ElementMetaFFI {
            // Create CStrings to get proper *const c_char pointers
            let el_name = ::std::ffi::CString::new(#el_name.replace("\0", "")).expect("Failed to make CString");
            let pkg_name = ::std::ffi::CString::new(env!("CARGO_PKG_NAME").replace("\0", "")).expect("Failed to make CString");
            let pkg_version = ::std::ffi::CString::new(env!("CARGO_PKG_VERSION").replace("\0", "")).expect("Failed to make CString");
            let pkg_license = ::std::ffi::CString::new(env!("CARGO_PKG_LICENSE").replace("\0", "")).expect("Failed to make CString");
            let pkg_authors = ::std::ffi::CString::new(env!("CARGO_PKG_AUTHORS").replace("\0", "")).expect("Failed to make CString");
            let blurb = ::std::ffi::CString::new(#blurb.replace("\0", "")).expect("Failed to make CString");
            let pkg_repo = ::std::ffi::CString::new(env!("CARGO_PKG_REPOSITORY").replace("\0", "")).expect("Failed to make CString");

            ::physim_core::plugin::ElementMetaFFI {
                kind: ::physim_core::plugin::ElementKind::Integrator,
                name: alloc(el_name.as_ptr()),
                plugin: alloc(pkg_name.as_ptr()),
                version: alloc(pkg_version.as_ptr()),
                license: alloc(pkg_license.as_ptr()),
                author: alloc(pkg_authors.as_ptr()),
                blurb: alloc(blurb.as_ptr()),
                repo: alloc(pkg_repo.as_ptr()),
            }
        }
    };
    g.into()
}
