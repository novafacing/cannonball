//! This client is used by the QEMU plugin to communicate with the consumer connected to the
//! UNIX socket. It is very simple and essentially provides two functions: `setup` and `submit`
//! to create the pipes and socket and start a thread to listen for events, and to submit events
//! to the socket, respectively.
use std::ffi::CStr;
use std::mem::ManuallyDrop;
use std::process::exit;
use std::thread::sleep;
use std::time::Duration;

use futures::SinkExt;
use libc::c_char;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::{
    net::UnixStream,
    runtime::{Builder, Runtime},
};
use tokio_util::codec::Framed;

use crate::qemu_event::{QemuEventCodec, QemuEventExec};

pub enum ClientEvent {
    Event(QemuEventExec),
    Shutdown,
}

/// Run the client's listener thread on the Tokio event loop. This will receive events off of
/// the receive end of the channel and send them to the UNIX socket. It will batch events for
/// efficiency.
pub fn run(
    runtime: ManuallyDrop<Runtime>,
    mut stream: Framed<UnixStream, QemuEventCodec>,
    mut receiver: UnboundedReceiver<ClientEvent>,
    batch_size: usize,
) {
    runtime.spawn(async move {
        let mut ctr = 0;
        loop {
            let r = receiver.recv().await.unwrap();
            match r {
                ClientEvent::Event(evt) => {
                    // TODO: handle error
                    stream.feed(evt).await.unwrap();
                    ctr += 1;

                    if ctr == batch_size {
                        ctr = 0;
                        // TODO: handle error
                        stream.flush().await.unwrap();
                    }
                }
                ClientEvent::Shutdown => {
                    stream.flush().await.unwrap();
                }
            }
        }
    });
}

/// A handle to the client sender object. This is used to submit events to the thread that pulls
/// then off of the channel and sends them to the UNIX socket. This struct is opaque to the QEMU
/// plugin.
pub struct Sender {
    /// The sender side of the channel that the client dispatcher thread is pulling events from
    sender: UnboundedSender<ClientEvent>,
}

impl Sender {
    /// Submit an event to the client dispatcher thread over the send side of the channel
    pub fn send(&self, msg: QemuEventExec) {
        match self.sender.send(ClientEvent::Event(msg)) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error sending message: {}", e);
                exit(1);
            }
        }
    }

    pub fn shutdown(&self) {
        match self.sender.send(ClientEvent::Shutdown) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error sending message: {}", e);
                exit(1);
            }
        }
    }
}

#[no_mangle]
/// Setup the UNIX socket and start the client dispatcher thread. This function is called by the
/// QEMU plugin to initialize the client via FFI
pub extern "C" fn setup(batch_size: usize, socket: *const c_char) -> *mut Sender {
    let c_str = unsafe { CStr::from_ptr(socket) };
    let c_string = c_str.to_str().unwrap();

    // This breaks new mode of operation!
    // if Path::new(c_string).exists() {
    //     // Delete the socket if it already exists
    //     remove_file(c_string).unwrap();
    // }

    // TODO: Don't let the runtime go out of scope (which cancels the receive, which breaks the channel) but also...lets not do this.
    let runtime = ManuallyDrop::new(Builder::new_multi_thread().enable_all().build().unwrap());
    let mut ustream: Option<UnixStream> = None;

    // Try to connect to the socket until it is available
    while ustream.is_none() {
        match runtime.block_on(UnixStream::connect(c_string)) {
            Ok(s) => ustream = Some(s),
            Err(_) => {
                sleep(Duration::from_millis(333));
            }
        }
    }

    let ustream = ustream.unwrap();

    let stream = Framed::new(ustream, QemuEventCodec {});
    let (sender, receiver) = unbounded_channel();

    let sender = sender;
    let receiver = receiver;

    run(runtime, stream, receiver, batch_size);

    Box::into_raw(Box::new(Sender {
        sender: sender.clone(),
    }))
}

#[no_mangle]
/// Submit an event to the client dispatcher thread. This function is called by the QEMU plugin
/// to submit events via FFI
pub extern "C" fn submit(client: *mut Sender, event: *mut QemuEventExec) {
    let sender = unsafe { &mut *client };
    let event = unsafe { &mut *event };

    sender.send(*event);
}

#[no_mangle]
/// Destroy the client sender object and stop the Tokio runtime. This function is called by the
/// QEMU plugin to destroy the client sender object via FFI
pub extern "C" fn teardown(client: *mut Sender) {
    // TODO: This should drop the runtime and the channel on QEMU exit if we want to be
    // nitpicky
    let sender = unsafe { &mut *client };
    sender.shutdown();
}

#[no_mangle]
/// Debug function to print out a qemu event struct
pub extern "C" fn dbg_print_evt(event: *mut QemuEventExec) {
    let event = unsafe { &mut *event };
    eprintln!("Event: {:?}", event);
}
