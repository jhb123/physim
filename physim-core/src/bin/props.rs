use physim_core::plugin::generator::ElementConfigurationHandler;

fn main() {
    let elements_db = physim_core::plugin::discover_map();

    let element_meta = elements_db.get("debug").expect("plugins not loaded");
    let path = &element_meta.lib_path;
    // physim_core::discover()
    let properties = serde_json::json!({ "prop": 0, "a": 2}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    let el =
        physim_core::plugin::transform::TransformElementHandler::load(path, "debug", properties)
            .unwrap();

    let properties = serde_json::json!({ "a": 3}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();

    el.set_properties(properties);

    let properties = serde_json::json!({ "a": 31}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    el.set_properties(properties);

    let p = el.get_property("b");
    println!("{:?}", p);

    println!("{:?}", el.get_property_descriptions());
    //
    let element_meta = elements_db.get("astro").expect("plugins not loaded");
    let path = &element_meta.lib_path;

    let properties = serde_json::json!({ "prop": 0, "a": 2}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    let el =
        physim_core::plugin::transform::TransformElementHandler::load(path, "astro", properties)
            .unwrap();
    let properties = serde_json::json!({ "a": 3}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();

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
    let element =
        physim_core::plugin::generator::GeneratorElementHandler::load(path, "cube", properties)
            .unwrap();
    let properties = serde_json::json!({ "n": 3}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();

    let el = element;
    el.set_properties(properties);

    let properties = serde_json::json!({ "n": 31.2}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    el.set_properties(properties);

    let p = el.get_property("n");
    println!("{:?}", p);

    println!("{:?}", el.get_property_descriptions());

    // check render elements
    let element_meta = elements_db.get("glrender").expect("plugins not loaded");
    let path = &element_meta.lib_path;

    let properties = serde_json::json!({ "prop": 0, "a": 2}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    let element =
        physim_core::plugin::render::RenderElementHandler::load(path, "glrender", properties)
            .unwrap();
    let properties = serde_json::json!({ "a": 3}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();

    let el = element;
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

    let el = element;
    el.set_properties(properties);

    let properties = serde_json::json!({ "resolution": "4k"}).to_string();
    let properties = serde_json::from_str(&properties).unwrap();
    el.set_properties(properties);

    let p = el.get_property("resolution");
    println!("{:?}", p);

    println!("{:?}", el.get_property_descriptions());
}
