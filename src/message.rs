use std::mem::size_of;
use std::intrinsics::TypeId;

use rawmessage::RawMessage;
use syncmessage::SyncMessage;
use clonemessage::CloneMessage;

pub fn workaround_to_static_bug() {
    panic!("sync message was not correct type");
}

/// A structure for generic represention of a message.
///
/// This structure contains the actual message data. It exposes
/// the source and destination fields, but the actual message payload
/// must be accessed using the `payload` field.
pub struct Message {
    pub srcsid:         u64,             // source server id
    pub srceid:         u64,             // source endpoint id
    pub dstsid:         u64,             // destination server id
    pub dsteid:         u64,             // destination endpoint id
    pub canloop:        bool,            // can loop back into sender?
    pub payload:        MessagePayload,  // actual payload
}

/// Helps `Message` become generic allowing it to represent multiple types of messages.
pub enum MessagePayload {
    Raw(RawMessage),
    Sync(SyncMessage),
    Clone(CloneMessage),
}

unsafe impl Send for Message {}
unsafe impl Send for CloneMessage {}

impl Clone for Message {
    /// Will properly clone the message and respect the actual message type. This can fail
    /// with a panic if the actual message type does not support `clone()` therefore it is
    /// your responsibility to verify or ensure the message supports `clone()`.
    fn clone(&self) -> Message {
        match self.payload {
            MessagePayload::Raw(ref msg) => {
                Message {
                    canloop: false,
                    srcsid: self.srcsid, srceid: self.srceid,
                    dstsid: self.dstsid, dsteid: self.dsteid,
                    payload: MessagePayload::Raw((*msg).clone())
                }
            },
            MessagePayload::Clone(ref msg) => {
                Message {
                    canloop: false,
                    srcsid: self.srcsid, srceid: self.srceid,
                    dstsid: self.dstsid, dsteid: self.dsteid,
                    payload: MessagePayload::Clone((*msg).clone())
                }                
            }
            MessagePayload::Sync(ref msg) => {
                panic!("Tried to clone a SyncMessage which is unique!");
            }
        }


    }
}

impl Message {
    /// Returns the total capacity of the message. Also known as the total size of the 
    /// message buffer which will contain raw byte data.
    pub fn cap(&self) -> uint {
        match self.payload {
            MessagePayload::Raw(ref msg) => msg.cap(),
            MessagePayload::Sync(ref msg) => msg.payload.cap(),
            MessagePayload::Clone(ref msg) => msg.payload.cap(),
        }
    }

    /// Duplicates the message making a new buffer and returning that message. _At the
    /// moment only raw messages can be duplicated._
    pub fn dup(&self) -> Message {
        match self.payload {
            MessagePayload::Raw(ref msg) => {
                Message {
                    canloop: self.canloop,
                    dstsid: self.dstsid, dsteid: self.dsteid,
                    srcsid: self.srcsid, srceid: self.srceid,
                    payload: MessagePayload::Raw(msg.dup())
                }
            },
            _ => {
                panic!("message type can not be duplicated!");
            }
        }
    }

    pub fn dup_ifok(self) -> Message {
        match self.payload {
            MessagePayload::Raw(ref msg) => {
                Message {
                    canloop: self.canloop,
                    dstsid: self.dstsid, dsteid: self.dsteid,
                    srcsid: self.srcsid, srceid: self.srceid,
                    payload: MessagePayload::Raw(msg.dup())
                }
            },
            _ => {
                self
            }
        }   
    }

    pub fn get_raw(self) -> RawMessage {
        match self.payload {
            MessagePayload::Raw(msg) => {
                msg
            },
            _ => {
                panic!("message was not type raw! [consider checking type]")
            }
        }
    }

    pub fn get_rawmutref(&mut self) -> &mut RawMessage {
        match self.payload {
            MessagePayload::Raw(ref mut msg) => {
                msg
            },
            _ => {
                panic!("message was not type raw! [consider checking type]")
            }
        }
    }

    pub fn get_clonemutref(&mut self) -> &mut CloneMessage {
        match self.payload {
            MessagePayload::Clone(ref mut msg) => {
                msg
            },
            _ => {
                panic!("message was not type clone! [consider checking type]")
            }
        }
    }

    pub fn get_syncmutref(&mut self) -> &mut SyncMessage {
        match self.payload {
            MessagePayload::Sync(ref mut msg) => {
                msg
            },
            _ => {
                panic!("message was not type sync! [consider checking type]")
            }
        }
    }

    pub fn get_rawref(&self) -> &RawMessage {
        match self.payload {
            MessagePayload::Raw(ref msg) => {
                msg
            },
            _ => {
                panic!("message was not type raw! [consider checking type]")
            }
        }
    }

    pub fn get_cloneref(&self) -> &CloneMessage {
        match self.payload {
            MessagePayload::Clone(ref msg) => {
                msg
            },
            _ => {
                panic!("message was not type clone! [consider checking type]")
            }
        }
    }

    pub fn get_syncref(&self) -> &SyncMessage {
        match self.payload {
            MessagePayload::Sync(ref msg) => {
                msg
            },
            _ => {
                panic!("message was not type sync! [consider checking type]")
            }
        }
    }

    pub fn get_clone(self) -> CloneMessage {
        match self.payload {
            MessagePayload::Clone(msg) => {
                msg
            },
            _ => {
                panic!("message was not type clone! [consider checking type]")
            }
        }
    }

    pub fn get_sync(self) -> SyncMessage {
        match self.payload {
            MessagePayload::Sync(msg) => {
                msg
            },
            _ => {
                panic!("message was not type sync! [consider checking type]")
            }
        }
    }

    pub fn is_clone(&self) -> bool {
        match self.payload {
            MessagePayload::Sync(_) => false,
            MessagePayload::Raw(_) => false,
            MessagePayload::Clone(_) => true,
        }
    }

    pub fn is_raw(&self) -> bool {
        match self.payload {
            MessagePayload::Sync(_) => false,
            MessagePayload::Raw(_) => true,
            MessagePayload::Clone(_) => false,
        }
    }

    pub fn is_sync(&self) -> bool {
        match self.payload {
            MessagePayload::Sync(_) => true,
            MessagePayload::Raw(_) => false,
            MessagePayload::Clone(_) => false,
        }
    }

    pub fn new_fromraw(rmsg: RawMessage) -> Message {
        Message {
            canloop: false,
            srcsid: 0, srceid: 0,
            dstsid: 0, dsteid: 0,
            payload: MessagePayload::Raw(rmsg),
        }
    }

    pub fn new_raw(cap: uint) -> Message {
        Message {
            canloop: false,
            srcsid: 0, srceid: 0,
            dstsid: 0, dsteid: 0,
            payload: MessagePayload::Raw(RawMessage::new(cap)),
        }
    }

    pub fn new_clone<T: Send + Clone + 'static>(t: T) -> Message {
        // Create a message payload of type Sync.
        let payload = MessagePayload::Clone(CloneMessage::new(t));

        Message {
            canloop: false,
            srcsid: 0, srceid: 0, dstsid: 0, dsteid: 0,
            payload: payload,
        }
    }

    pub fn new_sync<T: Send + 'static>(t: T) -> Message {
        // Create a message payload of type Sync.
        let payload = MessagePayload::Sync(SyncMessage::new(t));

        Message {
            canloop: false,
            srcsid: 0, srceid: 0, dstsid: 0, dsteid: 0,
            payload: payload,
        }
    }

    pub fn is_type<T: Send + 'static>(&self) -> bool {
        if self.is_sync() && self.get_syncref().is_type::<T>() {
            return true;
        }

        if self.is_clone() && self.get_cloneref().is_type::<T>() {
            return true;
        }

        false
    }
}