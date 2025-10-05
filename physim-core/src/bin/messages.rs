use std::{
    collections::HashMap,
    ffi::c_void,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use physim_core::{
    messages::{post_bus_callback, Message, MessageBus, MessageClient, MessagePriority},
    plugin::{set_bus, transform::TransformElementHandler},
};

struct TestClient {}
impl MessageClient for TestClient {
    fn recv_message(&self, message: &Message) {
        println!("{:?}", message)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bus = Arc::new(Mutex::new(MessageBus::new()));
    bus.lock().unwrap().add_client(Arc::new(TestClient {}));

    let elements_db = physim_core::plugin::element_db();

    let element_meta = elements_db.get("debug").expect("plugins not loaded");
    unsafe {
        let _ = set_bus(element_meta, bus.clone());
    }

    let element =
        TransformElementHandler::load(element_meta.get_lib_path(), "debug", HashMap::default())
            .unwrap();

    let b1 = bus.clone();
    let t1 = thread::spawn(move || {
        let c1_id = 1;
        let bus_raw_ptr = Arc::into_raw(b1) as *mut c_void;
        let msg1 = Message {
            topic: "c1".to_owned(),
            message: format!("1 - sent by {c1_id}"),
            priority: MessagePriority::Low,
            sender_id: c1_id,
        };
        let msg2 = Message {
            topic: "c1".to_owned(),
            message: format!("2 - sent by {c1_id}"),
            priority: MessagePriority::Normal,
            sender_id: c1_id,
        };
        let msg3 = Message {
            topic: "c1".to_owned(),
            message: format!("3 - sent by {c1_id}"),
            priority: MessagePriority::RealTime,
            sender_id: c1_id,
        };
        let msg4 = Message {
            topic: "c1".to_owned(),
            message: format!("4 - sent by {c1_id}"),
            priority: MessagePriority::Critical,
            sender_id: c1_id,
        };
        post_bus_callback(bus_raw_ptr, msg1.to_c_message());
        post_bus_callback(bus_raw_ptr, msg2.to_c_message());
        post_bus_callback(bus_raw_ptr, msg3.to_c_message());
        post_bus_callback(bus_raw_ptr, msg4.to_c_message());
        unsafe { Arc::from_raw(bus_raw_ptr as *mut usize) };
    });
    let b2 = bus.clone();

    let t2 = thread::spawn(move || {
        let bus_raw_ptr = Arc::into_raw(b2) as *mut c_void;
        let c2_id = 2;
        let msg1 = Message {
            topic: "c2".to_owned(),
            message: format!("1 - sent by {c2_id}"),
            priority: MessagePriority::Low,
            sender_id: c2_id,
        };
        let msg2 = Message {
            topic: "c2".to_owned(),
            message: format!("2 - sent by {c2_id}"),
            priority: MessagePriority::Normal,
            sender_id: c2_id,
        };
        let msg3 = Message {
            topic: "c2".to_owned(),
            message: format!("3 - sent by {c2_id}"),
            priority: MessagePriority::RealTime,
            sender_id: c2_id,
        };
        let msg4 = Message {
            topic: "c2".to_owned(),
            message: format!("4 - sent by {c2_id}"),
            priority: MessagePriority::Critical,
            sender_id: c2_id,
        };
        post_bus_callback(bus_raw_ptr, msg1.to_c_message());
        post_bus_callback(bus_raw_ptr, msg2.to_c_message());
        post_bus_callback(bus_raw_ptr, msg3.to_c_message());
        post_bus_callback(bus_raw_ptr, msg4.to_c_message());
        unsafe { Arc::from_raw(bus_raw_ptr as *mut usize) };
    });

    let t3 = thread::spawn(move || {
        for _ in 0..100 {
            let mut lock = bus.lock().unwrap();
            lock.pop_messages();
            drop(lock);
            thread::sleep(Duration::from_millis(8)); // don't want to spend literally all our computation on this?
        }
    });
    {
        element.transform(&[], &mut []);
        element.transform(&[], &mut []);
        element.transform(&[], &mut []);
        element.transform(&[], &mut []);
    }
    let _ = t1.join();
    let _ = t2.join();
    let _ = t3.join();
    Ok(())
}
