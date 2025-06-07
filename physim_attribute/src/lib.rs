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
    let set_property_fn = format_ident!("{}_set_properties", prefix);
    let get_property_fn = format_ident!("{}_get_properties", prefix);
    let get_property_descriptions_fn = format_ident!("{}_get_property_descriptions", prefix);
    let recv_message_fn = format_ident!("{}_recv_message", prefix);

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

            let el = Box::new(#struct_name::new( properties ) );
            Box::into_raw(el) as *mut std::ffi::c_void
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #transform_fn(obj: *const std::ffi::c_void, state: *const Entity, state_len: usize, new_state: *mut Entity, new_state_len: usize, dt: f32) {
            let el: & #struct_name = unsafe { &*(obj as *const #struct_name) };
            let s =  unsafe { std::slice::from_raw_parts(state, state_len) };
            let n =  unsafe {  std::slice::from_raw_parts_mut(new_state, new_state_len) };
            el.transform(s, n,dt);
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #destroy_fn(obj: *mut std::ffi::c_void) {
            if obj.is_null() {return};
            drop(unsafe { Box::from_raw(obj as *mut #struct_name) });
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #api_fn() -> *const ::physim_core::plugin::transform::TransformElementAPI {
            Box::into_raw(Box::new(::physim_core::plugin::transform::TransformElementAPI {
                init: #init_fn,
                transform: #transform_fn,
                destroy: #destroy_fn,
                set_properties: #set_property_fn,
                get_property: #get_property_fn,
                get_property_descriptions: #get_property_descriptions_fn,
                recv_message: #recv_message_fn,
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
        pub unsafe extern "C" fn #set_property_fn(obj: *mut std::ffi::c_void, data: *mut std::ffi::c_char) {
            if obj.is_null() {return};
            let el: &mut #struct_name = unsafe { &mut *(obj as *mut #struct_name) };

            let new_props = unsafe { std::ffi::CString::from_raw(data) };

            let properties = match serde_json::from_str(new_props.to_str().unwrap()){
                Ok(properties) => el.set_properties( properties ),
                Err(_) => { panic!("handle this properly")},
            };

        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #get_property_fn(obj: *mut std::ffi::c_void, prop: *mut std::ffi::c_char) -> *mut std::ffi::c_char {
            if obj.is_null() {return std::ptr::null_mut()};
            let el: &mut #struct_name = unsafe { &mut *(obj as *mut #struct_name) };
            let prop = unsafe { std::ffi::CString::from_raw(prop) };
            match el.get_property( prop.to_str().unwrap() ) {
                Ok(value) => std::ffi::CString::new(value.to_string()).unwrap().into_raw(),
                Err(_) => std::ptr::null_mut()
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #get_property_descriptions_fn(obj: *mut std::ffi::c_void) -> *mut std::ffi::c_char {
            if obj.is_null() {return std::ptr::null_mut()};
            let el: &mut #struct_name = unsafe { &mut *(obj as *mut #struct_name) };
            let properties = el.get_property_descriptions();
            match serde_json::to_string(&properties) {
                Ok(s) => return std::ffi::CString::new(s).unwrap().into_raw(),
                Err(_) => return std::ptr::null_mut()
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
            el.recv_message(msg);
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
        // #[derive(::serde::Serialize)]
        #ast

        #[unsafe(no_mangle)]
        fn #create_element(properties: std::collections::HashMap<String, serde_json::Value>) -> Box<dyn ::physim_core::plugin::generator::GeneratorElement> {
            #name::create_element(properties)
        }

        // #[unsafe(no_mangle)]
        // fn #set_property_fn(properties: HashMap<String, Value>) {
        //     #name::set_properties(properties)
        // }

        // #[unsafe(no_mangle)]
        // fn #get_property_fn(prop:&str) -> Result<Value, Box<dyn std::error::Error>> {
        //     #name::get_property(prop)
        // }

        // #[unsafe(no_mangle)]
        // fn #get_property_descriptions_fn() -> HashMap<String, String> {
        //     #name::get_property_descriptions()
        // }

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
        #[derive(::serde::Serialize)]
        #ast

        #[unsafe(no_mangle)]
        fn #create_element(properties: std::collections::HashMap<String, serde_json::Value>) -> Box<dyn ::physim_core::plugin::generator::GeneratorElement> {
            #name::create_element(properties)
        }

        // #[unsafe(no_mangle)]
        // fn #set_property_fn(properties: HashMap<String, Value>) {
        //     #name::set_properties(properties)
        // }

        // #[unsafe(no_mangle)]
        // fn #get_property_fn(prop:&str) -> Result<Value, Box<dyn std::error::Error>> {
        //     #name::get_property(prop)
        // }

        // #[unsafe(no_mangle)]
        // fn #get_property_descriptions_fn() -> HashMap<String, String> {
        //     #name::get_property_descriptions()
        // }

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
