extern crate digest;
extern crate gdk;
extern crate gtk;
extern crate md5;
extern crate muff;
extern crate pango;
extern crate sha3;

mod ui;

use ui::{App, Connect};

fn main() { App::new().connect_events().then_execute() }
