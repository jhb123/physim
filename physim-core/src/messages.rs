use std::{
    collections::BinaryHeap, ffi::{c_void, CStr, CString}, sync::{mpsc::{sync_channel, Receiver, SyncSender}, Arc, Mutex}, thread
};

// https://doc.rust-lang.org/nomicon/ffi.html#targeting-callbacks-to-rust-objects

#[repr(C)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub enum MessagePriority {
    Background,
    Low,
    Normal,
    High,
    RealTime,
    Critical,
}

#[derive(Eq, PartialEq, Clone)]
pub struct Message {
    pub priority: MessagePriority,
    pub topic: String,
    pub message: String,
    pub sender_id: usize,
}

#[repr(C)]
#[derive(Clone)]
pub struct CMessage {
    pub priority: MessagePriority,
    pub topic: *const std::ffi::c_char,
    pub message: *const std::ffi::c_char,
    pub sender_id: usize,
}

impl Message {
    pub fn to_c_message(self) -> (CMessage,CString,CString) {
        let topic_c = CString::new(self.topic.clone()).unwrap();
        let message_c = CString::new(self.message.clone()).unwrap();
        let c_msg = CMessage {
            priority: self.priority,
            topic: topic_c.as_ptr(),
            message: message_c.as_ptr(),
            sender_id: self.sender_id
        };
        (c_msg,topic_c,message_c)
    }
}

impl CMessage {
    pub unsafe fn to_message(self) -> Message {
        let topic = CStr::from_ptr(self.topic).to_str().unwrap().to_string();
        let message = CStr::from_ptr(self.message).to_str().unwrap().to_string();
        Message { priority: self.priority, topic, message, sender_id: self.sender_id }
    }
}


impl Ord for Message {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl PartialOrd for Message {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.priority.partial_cmp(&other.priority)
    }
}

pub trait MessageClient: Send + Sync {
    fn recv_message(&self, message: Message) {
        let sender_id = self as *const Self as *const () as usize;
        if message.sender_id !=  sender_id {
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
            let message = message.to_message();
            (*obj).post_message(message);
        }
        Arc::into_raw(arc);
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

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        thread,
        time::Duration,
    };

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
}
