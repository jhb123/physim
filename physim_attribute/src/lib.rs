#![feature(vec_into_raw_parts)]
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{DeriveInput, Lit, LitStr, parse_macro_input};

#[proc_macro_attribute]
pub fn transform_element(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);

    let attr_lit = parse_macro_input!(attr as syn::Lit);

    let prefix = if let Lit::Str(lit_str) = attr_lit {
        lit_str.value()
    } else {
        panic!("Expected a string literal as the macro attribute");
    };

    let el_name = LitStr::new(&prefix, Span::call_site());
    let struct_name = &ast.ident;
    let register_fn = format_ident!("{}_register", prefix);
    let init_fn = format_ident!("{}_init", prefix);
    let transform_fn = format_ident!("{}_transform", prefix);
    let destroy_fn = format_ident!("{}_destroy", prefix);
    let api_fn = format_ident!("{}_get_api", prefix);

    let g = quote! {
        #ast

        #[unsafe(no_mangle)]
        pub extern "C" fn #init_fn(config: *const u8, len: usize) -> *mut std::ffi::c_void {
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
        pub extern "C" fn #transform_fn(obj: *mut std::ffi::c_void, state: *const Entity, state_len: usize, new_state: *mut Entity, new_state_len: usize, dt: f32) {
            let el: &mut #struct_name = unsafe { &mut *(obj as *mut #struct_name) };
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
        pub extern "C" fn #api_fn() -> *const TransformElementAPI {
            Box::into_raw(Box::new(TransformElementAPI {
                init: #init_fn,
                transform: #transform_fn,
                destroy: #destroy_fn,
            }))
        }


        #[unsafe(no_mangle)]
        unsafe extern "C" fn #register_fn() -> ElementInfo {
            ElementInfo::new(
                ElementKind::Transform,
                #el_name,
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_LICENSE"),
                env!("CARGO_PKG_AUTHORS"),
            )
        }
    };
    g.into()
}

#[proc_macro_attribute]
pub fn render_element(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);

    let attr_lit = parse_macro_input!(attr as syn::Lit);

    let prefix = if let Lit::Str(lit_str) = attr_lit {
        lit_str.value()
    } else {
        panic!("Expected a string literal as the macro attribute");
    };

    // Build the trait implementation
    let el_name = LitStr::new(&prefix, Span::call_site());
    let name = &ast.ident;
    let create_element = format_ident!("{}_create_element", prefix);
    let register_fn = format_ident!("{}_register", prefix);

    let g = quote! {
        #ast

        #[unsafe(no_mangle)]
        fn #create_element(properties: HashMap<String, Value>) -> Box<dyn RenderElement> {
            #name::create_element(properties)
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn #register_fn() -> ElementInfo {
            ElementInfo::new(
                ElementKind::Render,
                #el_name,
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_LICENSE"),
                env!("CARGO_PKG_AUTHORS"),
            )
        }
    };
    g.into()
}

#[proc_macro_attribute]
pub fn initialise_state_element(attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as DeriveInput);

    let attr_lit = parse_macro_input!(attr as syn::Lit);

    let prefix = if let Lit::Str(lit_str) = attr_lit {
        lit_str.value()
    } else {
        panic!("Expected a string literal as the macro attribute");
    };

    // Build the trait implementation

    let el_name = LitStr::new(&prefix, Span::call_site());
    let name = &ast.ident;
    let create_element = format_ident!("{}_create_element", prefix);
    let register_fn = format_ident!("{}_register", prefix);

    let g = quote! {
        #ast

        #[unsafe(no_mangle)]
        fn #create_element(properties: HashMap<String, Value>) -> Box<dyn InitialStateElement> {
            #name::create_element(properties)
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn #register_fn() -> ElementInfo {
            ElementInfo::new(
                ElementKind::Initialiser,
                #el_name,
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_LICENSE"),
                env!("CARGO_PKG_AUTHORS"),
            )
        }
    };
    g.into()
}
