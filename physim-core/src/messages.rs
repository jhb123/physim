use std::{
    collections::BinaryHeap,
    ffi::{c_void, CStr, CString},
    sync::{Arc, Mutex},
};

// https://doc.rust-lang.org/nomicon/ffi.html#targeting-callbacks-to-rust-objects

#[repr(C)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug, Copy)]
pub enum MessagePriority {
    Background,
    Low,
    Normal,
    High,
    RealTime,
    Critical,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Message {
    pub priority: MessagePriority,
    pub topic: String,
    pub message: String,
    pub sender_id: usize,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct CMessage {
    pub priority: MessagePriority,
    pub topic: *const std::ffi::c_char,
    pub message: *const std::ffi::c_char,
    pub sender_id: usize,
}

impl Message {
    pub fn to_c_message(self) -> CMessage {
        let topic_c = CString::new(self.topic.clone()).unwrap();
        let message_c = CString::new(self.message.clone()).unwrap();

        CMessage {
            priority: self.priority,
            topic: topic_c.into_raw(),
            message: message_c.into_raw(),
            sender_id: self.sender_id,
        }
    }
}

impl Drop for CMessage {
    fn drop(&mut self) {
        unsafe {
            if !self.topic.is_null() {
                drop(CString::from_raw(self.topic as *mut i8));
            }
            if !self.message.is_null() {
                drop(CString::from_raw(self.message as *mut i8));
            }
        }
    }
}

impl CMessage {
    pub fn to_message(self) -> Message {
        unsafe {
            let topic = CStr::from_ptr(self.topic).to_str().unwrap().to_string();
            let message = CStr::from_ptr(self.message).to_str().unwrap().to_string();

            Message {
                priority: self.priority,
                topic,
                message,
                sender_id: self.sender_id,
            }
        }
    }
}

impl Ord for Message {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl PartialOrd for Message {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub trait MessageClient: Send + Sync {
    fn recv_message(&self, message: Message) {
        let sender_id = self as *const Self as *const () as usize;
        if message.sender_id != sender_id {
            println!(
                "Priority: {:?} - topic {} - message: {} - sender: {sender_id}",
                message.priority, message.topic, message.message
            )
        }
    }
}

pub extern "C" fn callback(target: *mut c_void, message: CMessage) {
    unsafe {
        let arc = Arc::from_raw(target as *const Mutex<MessageBus>);
        {
            let mut obj = arc.lock().unwrap();
            if message.message.is_null() || message.topic.is_null() {
                eprintln!("ERROR, message contents is null");
            } else {
                let message = message.to_message();
                (*obj).post_message(message);
            }
        }
        // Just above, we create the Arc. To prevent dropping the message bus,
        // turn it back into raw pointer.
        let _ = Arc::into_raw(arc);
    }
}

pub struct MessageBus {
    queue: Mutex<BinaryHeap<Message>>,
    clients: Vec<Arc<dyn MessageClient>>,
}

impl MessageBus {
    pub fn post_message(&mut self, message: Message) {
        self.queue.lock().unwrap().push(message);
    }

    pub fn pop_messages(&mut self) {
        let mut queue = self.queue.lock().unwrap();
        while let Some(msg) = queue.pop() {
            for observer in self.clients.iter() {
                observer.recv_message(msg.clone());
            }
        }
    }

    pub fn add_client(&mut self, client: Arc<dyn MessageClient>) {
        // add self bus pointer to client so client can call post_message
        self.clients.push(client);
    }

    pub fn new() -> Self {
        Self {
            queue: Mutex::new(BinaryHeap::new()),
            clients: vec![],
        }
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    struct TestClient {}

    impl MessageClient for TestClient {
        fn recv_message(&self, message: super::Message) {
            println!(
                "Priority: {:?} - topic {} - message: {}",
                message.priority, message.topic, message.message
            )
        }
    }

    #[rustfmt::skip]
    #[test]
    fn test_message_bus() {
        
        assert!(true)
    }

    #[test]
    fn test_c_message() {
        let msg = Message {
            priority: MessagePriority::RealTime,
            topic: "topic".to_string(),
            message: "message".to_string(),
            sender_id: 0,
        };

        let c_msg = msg.clone().to_c_message();

        let msg2 = c_msg.to_message();
        assert_eq!(msg, msg2);
    }

    #[test]
    fn test_c_message_lifecycle() {
        let msg = Message {
            priority: MessagePriority::RealTime,
            topic: "topic".to_string(),
            message: "message".to_string(),
            sender_id: 0,
        };

        let c_msg = msg.to_c_message();
        drop(c_msg)
    }
}
