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
    /// You must call CMesssage::to_message() to prevent memory leaks
    pub fn to_c_message(&self) -> CMessage {
        let topic_c = CString::new(self.topic.clone()).unwrap();
        let message_c = CString::new(self.message.clone()).unwrap();

        CMessage {
            priority: self.priority,
            topic: topic_c.into_raw(),
            message: message_c.into_raw(),
            sender_id: self.sender_id,
        }
    }

    /// this creates a clone of the underlying data in the CMessage without
    /// without consuming it. A Message is returned, and its memory is
    /// managed independently of the original CMessage
    pub unsafe fn from_c_ptr(c_msg: *const CMessage) -> Message {
        let c_msg = &*c_msg;
        let topic = CStr::from_ptr(c_msg.topic).to_string_lossy().into_owned();
        let message = CStr::from_ptr(c_msg.message).to_string_lossy().into_owned();

        Message {
            priority: c_msg.priority,
            topic,
            message,
            sender_id: c_msg.sender_id,
        }
    }
}

/// Constructs a `Message` for the plugin messaging system with a topic, message body, priority, and sender ID.
///
/// This macro is designed to be called from **within any function defined on a plugin element**. The `sender_id` is
/// automatically generated using the memory address of the element instance (`self`). This allows the messaging
/// system to identify the origin of the message.
///
/// # Arguments
///
/// * `$self` - The element instance. Typically `self`, used to derive a unique `sender_id`.
/// * `$topic` - A topic string (`&str` or `String`) representing the category or subject of the message.
/// * `$message` - The content of the message. Must be convertible to a `String`.
/// * `$priority` - The priority level of the message `MessagePriority`.
///
/// # Context
///
/// - This macro should be called from **functions that are part of an element** within a plugin.
/// - Elements are defined in the plugin (a dynamic library), and represent stateful units with related logic.
/// - The resulting `Message` is typically sent via a macro like `post_bus_msg!` to a global bus.
///
/// # Example
///
/// ```ignore
/// impl TransformElement for DebugTransform {
///     fn transform(&mut self, state: &[Entity], new_state: &mut [Entity], _dt: f32) {
///         for (i, e) in state.iter().enumerate() {
///             new_state[i] = *e
///         }
///         let msg1 = physim_core::msg!(
///             self,
///             "debugplugin",
///             "this is a message from debug transform",
///             MessagePriority::Low
///         );
///         post_bus_msg!(msg1);
///     }
/// }
/// ```
///
/// # Safety
///
/// Internally performs raw pointer casting to derive a `sender_id` from `self`. This is safe as long as `self`
/// is a valid reference to a plugin element instance.
#[macro_export]
macro_rules! msg {
    ($self:expr, $topic:expr, $message:expr, $priority:expr) => {
        $crate::messages::Message {
            topic: $topic.to_owned(),
            message: $message.to_string(),
            priority: $priority,
            sender_id: $self as *const Self as *const () as usize,
        }
    };
}

impl CMessage {
    pub fn to_message(self) -> Message {
        unsafe {
            let topic = CString::from_raw(self.topic as *mut i8)
                .to_str()
                .unwrap()
                .to_string();
            let message = CString::from_raw(self.message as *mut i8)
                .to_str()
                .unwrap()
                .to_string();

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

mod private {
    pub trait Sealed {
        fn recv_message_filtered(&self, message: super::Message);
    }

    impl<T: super::MessageClient> Sealed for T {
        fn recv_message_filtered(&self, message: super::Message) {
            let sender_id = self as *const Self as *const () as usize;
            if message.sender_id != sender_id {
                self.recv_message(&message)
            }
        }
    }
}
pub trait MessageClient: Send + Sync + private::Sealed {
    #[allow(unused_variables)]
    fn recv_message(&self, message: &Message) {}
    fn post_configuration_messages(&self) {}
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
                observer.recv_message_filtered(msg.clone());
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
        fn recv_message(&self, message: &super::Message) {
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
