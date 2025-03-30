use physim_core::plugin::initialiser::ElementConfigurationHandler;

fn main() {
    let path = "/Users/josephbriggs/repos/physim/target/release/libastro.dylib";
    // physim_core::discover()
    let properties = serde_json::json!({ "prop": 0, "a": 2}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    let element =
        physim_core::plugin::transform::TransformElementHandler::load(path, "debug", properties)
            .unwrap();

    let properties = serde_json::json!({ "a": 3}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();

    let mut el = element.lock().unwrap();
    el.set_properties(properties);

    let properties = serde_json::json!({ "a": 31}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    el.set_properties(properties);

    let p = el.get_property("b");
    println!("{:?}", p);

    println!("{:?}", el.get_property_descriptions());
    //
    let properties = serde_json::json!({ "prop": 0, "a": 2}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    let element =
        physim_core::plugin::transform::TransformElementHandler::load(path, "astro", properties)
            .unwrap();
    let properties = serde_json::json!({ "a": 3}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();

    let mut el = element.lock().unwrap();
    el.set_properties(properties);

    let properties = serde_json::json!({ "theta": 31}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    el.set_properties(properties);

    let p = el.get_property("theta");
    println!("{:?}", p);

    println!("{:?}", el.get_property_descriptions());

    //
    let properties = serde_json::json!({ "prop": 0, "a": 2}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    let element = physim_core::plugin::initialiser::InitialStateElementHandler::load(
        path, "cube", properties,
    )
    .unwrap();
    let properties = serde_json::json!({ "n": 3}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();

    let mut el = element;
    el.set_properties(properties);

    let properties = serde_json::json!({ "n": 31.2}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    el.set_properties(properties);

    let p = el.get_property("n");
    println!("{:?}", p);

    println!("{:?}", el.get_property_descriptions());

    // check render elements
    let path = "/Users/josephbriggs/repos/physim/target/release/libglrender.dylib";
    let properties = serde_json::json!({ "prop": 0, "a": 2}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    let element =
        physim_core::plugin::render::RenderElementHandler::load(path, "glrender", properties)
            .unwrap();
    let properties = serde_json::json!({ "a": 3}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();

    let mut el = element;
    el.set_properties(properties);

    let properties = serde_json::json!({ "resolution": "4k"}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    el.set_properties(properties);

    let p = el.get_property("resolution");
    println!("{:?}", p);

    println!("{:?}", el.get_property_descriptions());

    let properties = serde_json::json!({ "prop": 0, "a": 2}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    let element =
        physim_core::plugin::render::RenderElementHandler::load(path, "stdout", properties)
            .unwrap();
    let properties = serde_json::json!({ "a": 3}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();

    let mut el = element;
    el.set_properties(properties);

    let properties = serde_json::json!({ "resolution": "4k"}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    el.set_properties(properties);

    let p = el.get_property("resolution");
    println!("{:?}", p);

    println!("{:?}", el.get_property_descriptions());
}
