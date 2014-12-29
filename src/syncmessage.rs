use std::mem::size_of;
use std::intrinsics::TypeId;

use rawmessage::RawMessage;

/// A message that can not be cloned or copied, and can be shared with other threads.
///
/// This message can not be cloned or copied and can only be recieved
/// by a single endpoint on the local net. If you try to send it to
/// other nets it will fail, possibily with a panic. Therefore it should
/// be known that this message can not cross process boundaries.
pub struct SyncMessage {
    pub hash:           u64,
    pub payload:        RawMessage,
}

unsafe impl Send for SyncMessage { }

impl SyncMessage {
    /// Return the type contained in the sync message by consuming the
    /// sync message so that it can no longer be used. 

    /// _There is no contraint on `T` to be Send because the type is 
    /// checked using the hash._
    pub fn get_payload<T: 'static>(self) -> T {
        let rawmsg = self.payload;

        let tyid = TypeId::of::<T>();
        let hash = tyid.hash();

        if hash != self.hash {
            panic!("sync message was not correct type");
        }

        let t: T = unsafe { rawmsg.readstructunsafe(0) };
        t
    }

    /// Check if the type is contained. `is_type::<MyType>()`
    pub fn is_type<T: Send + 'static>(&self) -> bool {
        let tyid = TypeId::of::<T>();
        let hash = tyid.hash();

        if hash != self.hash {
            return false;
        }

        return true;
    }

    /// Create a new sync message by consuming the type passed making
    /// that type unique where it can not be cloned or duplicated.
    pub fn new<T: Send + 'static>(t: T) -> SyncMessage {
        let tyid = TypeId::of::<T>();
        let hash = tyid.hash();

        // Write the structure into a raw message, and
        // consume it in the process making it unsable.
        let mut rmsg = RawMessage::new(size_of::<T>());
        rmsg.writestruct(0, t);

        SyncMessage {
            hash:       hash,
            payload:    rmsg
        }
    }
}
