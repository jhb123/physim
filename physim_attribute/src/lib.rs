#![feature(vec_into_raw_parts)]
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, LitStr, parse_macro_input};

#[proc_macro_attribute]
pub fn transform_element(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);
    let mut prefix: Option<LitStr> = None;
    let mut blurb: Option<LitStr> = None;

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("name") {
            prefix = meta.value()?.parse()?;
            Ok(())
        } else if meta.path.is_ident("blurb") {
            blurb = meta.value()?.parse()?;
            Ok(())
        } else {
            Err(meta.error("unsupported property"))
        }
    });

    parse_macro_input!(attr with parser);

    if prefix.is_none() {
        panic!("Must specify a name")
    }
    if blurb.is_none() {
        panic!("Must specify a blurb")
    }

    let el_name = prefix.clone().unwrap();
    let blurb = blurb.unwrap();

    let prefix = prefix.unwrap().value();

    let struct_name = &ast.ident;
    let register_fn = format_ident!("{}_register", prefix);
    let init_fn = format_ident!("{}_init", prefix);
    let transform_fn = format_ident!("{}_transform", prefix);
    let destroy_fn = format_ident!("{}_destroy", prefix);
    let api_fn = format_ident!("{}_get_api", prefix);
    let get_property_descriptions_fn = format_ident!("{}_get_property_descriptions", prefix);
    let recv_message_fn = format_ident!("{}_recv_message", prefix);
    let post_configuration_messages_fn = format_ident!("{}_post_configuration_messages", prefix);

    let g = quote! {
        #ast

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #init_fn(config: *const u8, len: usize) -> *mut std::ffi::c_void {
            if config.is_null() {
                return std::ptr::null_mut();
            }
            let config = unsafe { std::str::from_raw_parts(config, len) };

            let properties = match serde_json::from_str(config){
                Ok(properties) => properties,
                Err(_) => return std::ptr::null_mut(),
            };

            match std::panic::catch_unwind(std::panic::AssertUnwindSafe( || {
                Box::new(#struct_name::new( properties ) )
            })) {
                Ok(el) => {
                    Box::into_raw(el) as *mut std::ffi::c_void
                }
                Err(_) => {
                    eprintln!(
                        "Problem encountered in the {} element's new method. Aborting",
                        #prefix
                    );
                    std::process::abort();
                }
            }

        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #transform_fn(obj: *const std::ffi::c_void, state: *const Entity, state_len: usize, acceleration: *mut Acceleration, acceleration_len: usize) {
            let el: & #struct_name = unsafe { &*(obj as *const #struct_name) };
            let s =  unsafe { std::slice::from_raw_parts(state, state_len) };
            let n =  unsafe {  std::slice::from_raw_parts_mut(acceleration, acceleration_len) };
            if let Err(_) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| { el.transform(s, n)})) {
                eprintln!("Problem encountered in the {} element's transform method. Aborting", #prefix);
                std::process::abort();
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #destroy_fn(obj: *mut std::ffi::c_void) {
            if obj.is_null() {
                return;
            }

            let result = std::panic::catch_unwind(|| {
                drop(Box::from_raw(obj as *mut #struct_name));
            });

            if result.is_err() {
                eprintln!("Problem encountered in the {} element's drop method. Aborting", #prefix);
                std::process::abort();
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
        unsafe extern "C" fn #register_fn() -> ::physim_core::plugin::ElementMeta {
            ::physim_core::plugin::ElementMeta::new(
                ::physim_core::plugin::ElementKind::Transform,
                #el_name,
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_LICENSE"),
                env!("CARGO_PKG_AUTHORS"),
                #blurb,
                env!("CARGO_PKG_REPOSITORY")
            )
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #get_property_descriptions_fn(obj: *mut std::ffi::c_void) -> *mut std::ffi::c_char {
            if obj.is_null() {return std::ptr::null_mut()};
            let el: &mut #struct_name = unsafe { &mut *(obj as *mut #struct_name) };

            match std::panic::catch_unwind(std::panic::AssertUnwindSafe( || {
                let properties = el.get_property_descriptions();
                serde_json::to_string(&properties)
            })) {
                    Ok(Ok(s)) => {
                    // Successful JSON serialization
                    std::ffi::CString::new(s).unwrap().into_raw()
                }
                Ok(Err(_)) => {
                    // Serialization failed
                    std::ptr::null_mut()
                }
                Err(_) => {
                    eprintln!(
                        "Panic encountered in the {} element's get_property_descriptions method.",
                        #prefix
                    );
                    std::ptr::null_mut()
                }
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #recv_message_fn(obj: *mut std::ffi::c_void, msg: *mut std::ffi::c_void) {
            if obj.is_null() {return };
            let el: &mut #struct_name = unsafe { &mut *(obj as *mut #struct_name) };
            let msg = unsafe {
                let msg = (*(msg as *mut physim_core::messages::CMessage)).clone();
                msg.to_message()
            };
            if let Err(_) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| { el.recv_message(msg)})) {
                eprintln!("Problem encountered in the {} element's recv_message method. Aborting", #prefix);
                std::process::abort();
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #post_configuration_messages_fn(obj: *mut std::ffi::c_void) {
            if obj.is_null() {return };
            let el: &mut #struct_name = unsafe { &mut *(obj as *mut #struct_name) };
            if let Err(_) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| { el.post_configuration_messages();})) {
                eprintln!("Problem encountered in the {} element's post_configuration_messages method. Aborting", #prefix);
                std::process::abort();
            }
        }
    };
    g.into()
}

#[proc_macro_attribute]
pub fn render_element(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);

    let mut prefix: Option<LitStr> = None;
    let mut blurb: Option<LitStr> = None;

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("name") {
            prefix = meta.value()?.parse()?;
            Ok(())
        } else if meta.path.is_ident("blurb") {
            blurb = meta.value()?.parse()?;
            Ok(())
        } else {
            Err(meta.error("unsupported property"))
        }
    });

    parse_macro_input!(attr with parser);

    if prefix.is_none() {
        panic!("Must specify a name")
    }
    if blurb.is_none() {
        panic!("Must specify a blurb")
    }

    let el_name = prefix.clone().unwrap();
    let blurb = blurb.unwrap();

    let prefix = prefix.unwrap().value();

    let name = &ast.ident;
    let create_element = format_ident!("{}_create_element", prefix);
    let register_fn = format_ident!("{}_register", prefix);

    let g = quote! {
        #ast

        #[unsafe(no_mangle)]
        fn #create_element(properties: HashMap<String, Value>) -> Box<dyn ::physim_core::plugin::render::RenderElement> {
            #name::create_element(properties)
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn #register_fn() -> ::physim_core::plugin::ElementMeta {
            ::physim_core::plugin::ElementMeta::new(
                ::physim_core::plugin::ElementKind::Render,
                #el_name,
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_LICENSE"),
                env!("CARGO_PKG_AUTHORS"),
                #blurb,
                env!("CARGO_PKG_REPOSITORY")
            )
        }
    };
    g.into()
}

#[proc_macro_attribute]
pub fn initialise_state_element(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);

    let mut prefix: Option<LitStr> = None;
    let mut blurb: Option<LitStr> = None;

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("name") {
            prefix = meta.value()?.parse()?;
            Ok(())
        } else if meta.path.is_ident("blurb") {
            blurb = meta.value()?.parse()?;
            Ok(())
        } else {
            Err(meta.error("unsupported property"))
        }
    });

    parse_macro_input!(attr with parser);

    if prefix.is_none() {
        panic!("Must specify a name")
    }
    if blurb.is_none() {
        panic!("Must specify a blurb")
    }

    let el_name = prefix.clone().unwrap();
    let blurb = blurb.unwrap();

    let prefix = prefix.unwrap().value();

    let name = &ast.ident;
    let create_element = format_ident!("{}_create_element", prefix);
    let register_fn = format_ident!("{}_register", prefix);

    let g = quote! {
        #ast

        #[unsafe(no_mangle)]
        fn #create_element(properties: std::collections::HashMap<String, serde_json::Value>) -> Box<dyn ::physim_core::plugin::generator::GeneratorElement> {
            #name::create_element(properties)
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn #register_fn() -> ::physim_core::plugin::ElementMeta {
            ::physim_core::plugin::ElementMeta::new(
                ::physim_core::plugin::ElementKind::Initialiser,
                #el_name,
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_LICENSE"),
                env!("CARGO_PKG_AUTHORS"),
                #blurb,
                env!("CARGO_PKG_REPOSITORY")
            )
        }
    };
    g.into()
}

#[proc_macro_attribute]
pub fn synth_element(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);

    let mut prefix: Option<LitStr> = None;
    let mut blurb: Option<LitStr> = None;

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("name") {
            prefix = meta.value()?.parse()?;
            Ok(())
        } else if meta.path.is_ident("blurb") {
            blurb = meta.value()?.parse()?;
            Ok(())
        } else {
            Err(meta.error("unsupported property"))
        }
    });

    parse_macro_input!(attr with parser);

    if prefix.is_none() {
        panic!("Must specify a name")
    }
    if blurb.is_none() {
        panic!("Must specify a blurb")
    }

    let el_name = prefix.clone().unwrap();
    let blurb = blurb.unwrap();

    let prefix = prefix.unwrap().value();

    let name = &ast.ident;
    let create_element = format_ident!("{}_create_element", prefix);
    let register_fn = format_ident!("{}_register", prefix);

    let g = quote! {
        #ast

        #[unsafe(no_mangle)]
        fn #create_element(properties: std::collections::HashMap<String, serde_json::Value>) -> Box<dyn ::physim_core::plugin::generator::GeneratorElement> {
            #name::create_element(properties)
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn #register_fn() -> ::physim_core::plugin::ElementMeta {
            ::physim_core::plugin::ElementMeta::new(
                ::physim_core::plugin::ElementKind::Synth,
                #el_name,
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_LICENSE"),
                env!("CARGO_PKG_AUTHORS"),
                #blurb,
                env!("CARGO_PKG_REPOSITORY")
            )
        }
    };
    g.into()
}

#[proc_macro_attribute]
pub fn transmute_element(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);

    let mut prefix: Option<LitStr> = None;
    let mut blurb: Option<LitStr> = None;

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("name") {
            prefix = meta.value()?.parse()?;
            Ok(())
        } else if meta.path.is_ident("blurb") {
            blurb = meta.value()?.parse()?;
            Ok(())
        } else {
            Err(meta.error("unsupported property"))
        }
    });

    parse_macro_input!(attr with parser);

    if prefix.is_none() {
        panic!("Must specify a name")
    }
    if blurb.is_none() {
        panic!("Must specify a blurb")
    }

    let el_name = prefix.clone().unwrap();
    let blurb = blurb.unwrap();

    let prefix = prefix.unwrap().value();

    let name = &ast.ident;
    let create_element = format_ident!("{}_create_element", prefix);
    let register_fn = format_ident!("{}_register", prefix);

    let g = quote! {
        #ast

        #[unsafe(no_mangle)]
        fn #create_element(properties: std::collections::HashMap<String, serde_json::Value>) -> Box<dyn ::physim_core::plugin::transmute::TransmuteElement> {
            #name::create_element(properties)
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn #register_fn() -> ::physim_core::plugin::ElementMeta {
            ::physim_core::plugin::ElementMeta::new(
                ::physim_core::plugin::ElementKind::Transmute,
                #el_name,
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_LICENSE"),
                env!("CARGO_PKG_AUTHORS"),
                #blurb,
                env!("CARGO_PKG_REPOSITORY")
            )
        }
    };
    g.into()
}

#[proc_macro_attribute]
pub fn integrator_element(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);

    let mut prefix: Option<LitStr> = None;
    let mut blurb: Option<LitStr> = None;

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("name") {
            prefix = meta.value()?.parse()?;
            Ok(())
        } else if meta.path.is_ident("blurb") {
            blurb = meta.value()?.parse()?;
            Ok(())
        } else {
            Err(meta.error("unsupported property"))
        }
    });

    parse_macro_input!(attr with parser);

    if prefix.is_none() {
        panic!("Must specify a name")
    }
    if blurb.is_none() {
        panic!("Must specify a blurb")
    }

    let el_name = prefix.clone().unwrap();
    let blurb = blurb.unwrap();

    let prefix = prefix.unwrap().value();

    let name = &ast.ident;
    let create_element = format_ident!("{}_create_element", prefix);
    let register_fn = format_ident!("{}_register", prefix);

    let g = quote! {
        #ast

        #[unsafe(no_mangle)]
        fn #create_element(properties: std::collections::HashMap<String, serde_json::Value>) -> Box<dyn ::physim_core::plugin::integrator::IntegratorElement> {
            #name::create_element(properties)
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn #register_fn() -> ::physim_core::plugin::ElementMeta {
            ::physim_core::plugin::ElementMeta::new(
                ::physim_core::plugin::ElementKind::Integrator,
                #el_name,
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_LICENSE"),
                env!("CARGO_PKG_AUTHORS"),
                #blurb,
                env!("CARGO_PKG_REPOSITORY")
            )
        }
    };
    g.into()
}
