use std::ffi::CStr;
use std::fs::remove_file;
use std::mem::ManuallyDrop;
use std::path::Path;
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

pub fn run(
    runtime: ManuallyDrop<Runtime>,
    mut stream: Framed<UnixStream, QemuEventCodec>,
    mut receiver: UnboundedReceiver<QemuEventExec>,
    batch_size: usize,
) {
    runtime.spawn(async move {
        let mut ctr = 0;
        loop {
            let r = receiver.recv().await.unwrap();
            // TODO: handle error
            stream.feed(r).await.unwrap();
            ctr += 1;

            if ctr == batch_size {
                ctr = 0;
                // TODO: handle error
                stream.flush().await.unwrap();
            }
        }
    });
}

pub struct Sender {
    sender: UnboundedSender<QemuEventExec>,
}

impl Sender {
    pub fn send(&self, msg: QemuEventExec) {
        match self.sender.send(msg) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error sending message: {}", e);
                exit(1);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn setup(batch_size: usize, socket: *const c_char) -> *mut Sender {
    let c_str = unsafe { CStr::from_ptr(socket) };
    let c_string = c_str.to_str().unwrap();

    if Path::new(c_string).exists() {
        // Delete the socket if it already exists
        remove_file(c_string).unwrap();
    }

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
pub extern "C" fn submit(client: *mut Sender, event: *mut QemuEventExec) {
    let sender = unsafe { &mut *client };
    let event = unsafe { &mut *event };

    sender.send(*event);
}

#[no_mangle]
pub extern "C" fn teardown(_client: *mut Sender) {
    // TODO: This should drop the runtime and the channel on QEMU exit if we want to be
    // nitpicky
}

#[no_mangle]
pub extern "C" fn dbg_print_evt(event: *mut QemuEventExec) {
    let event = unsafe { &mut *event };
    eprintln!("Event: {:?}", event);
}
