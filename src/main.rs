#![feature(generators, generator_trait)]

extern crate carrier;
extern crate env_logger;
extern crate prost;

use carrier::error::Error;
use std::env;
use carrier::osaka;
use carrier::mio;
use mio_extras::channel;


const VERSION:  &'static str = env!("CARGO_PKG_VERSION");
const NAME:     &'static str = env!("CARGO_PKG_NAME");

// import generated protobuf types
// you don't have to use protobuf, but its pretty recommended
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/myorg.mything.v1.rs"));
}

pub fn main() -> Result<(), Error> {
    // logger setup, because the default env_logger options are terrible
    if let Err(_) = env::var("RUST_LOG") {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    // setup osaka async reactors
    let poll            = osaka::Poll::new();
    let config          = carrier::config::load()?;
    let mut publisher   = carrier::publisher::new(config)
        // these are the standard services that you probably always want
        .route("/v0/shell",                         None, carrier::publisher::shell::main)
        .route("/v0/sft",                           None, carrier::publisher::sft::main)
        .route("/v2/carrier.sysinfo.v1/sysinfo",    None, carrier::publisher::sysinfo::sysinfo)

        // add your custom services. yey
        .route("/v2/myorg.mything.v1/whodis",      None, whoami)
        .route("/v2/myorg.mything.v1/cheese",      None, cheese)

        // discovery service is pretty useful. you will regret not adding it
        .with_disco(NAME.to_string(), VERSION.to_string())

        // axons allows extending your service with external binaries.
        // If you're unsure about your security policies, dont use it.
        .with_axons()
        .publish(poll);

    // wait for publisher osaka task to finish
    publisher.run()
}

pub fn whoami(
    _poll: osaka::Poll,
    _headers: carrier::headers::Headers,
    _identity: &carrier::identity::Identity,
    mut stream: carrier::endpoint::Stream,
) -> Option<osaka::Task<()>> {
    // send a 200 ok header first
    stream.send(carrier::headers::Headers::ok().encode());


    // send a message. messages can be as big as you configured your other peer to allow.
    // Carrier will to the packet reassembly for you.
    stream.message(proto::Helo{
        whodis: "peter".to_string()
    });

    // this is a simple sync example. we have nothing further to wait for
    None
}


pub fn cheese(
    poll: osaka::Poll,
    headers: carrier::headers::Headers,
    identity: &carrier::identity::Identity,
    stream: carrier::endpoint::Stream,
) -> Option<osaka::Task<()>> {

    // DO NOT BLOCK inside event handlers, since you'll be starving the whole pipeline.
    // unfortunately you cannot use tokio, because that doesnt work on embedded systems.
    // You can use osaka instead, which requires rust unstable for the !feature declaration at the
    // top of this file. If all your commands return instantly, you don't need osaka.
    return Some(pics_or_it_didnt_happen(poll,headers,identity,stream));
}

#[osaka::osaka]
pub fn pics_or_it_didnt_happen(
    poll: osaka::Poll,
    _headers: carrier::headers::Headers,
    _identity: &carrier::identity::Identity,
    mut stream: carrier::endpoint::Stream,
)
{
    // send a 200 ok header first
    stream.send(carrier::headers::Headers::ok().encode());


    let (tx, rx) = channel::channel();

    std::thread::spawn(move ||{
        println!("SAY SCHEEEZ");
        std::thread::sleep(std::time::Duration::from_secs(3));
        tx.send(proto::Cute{
            picture: "here be cats".into()
        }).unwrap();
    });

    let token = poll
        .register(&rx, mio::Ready::readable(), mio::PollOpt::level())
        .unwrap();

    // drive the main loop
    loop {
        match rx.try_recv() {
            Ok(v) => {
                // if we get something, feed it back to the stream
                stream.message(v);
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                // if we get nothing, wait until token is ready
                yield poll.again(token.clone(), None);
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                // if the thread exited, exit this task
                return;
            }
        }
    }
}
