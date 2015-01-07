#![feature(slicing_syntax)]

extern crate test;
extern crate water;

use std::sync::mpsc::channel;
use std::mem::transmute_copy;

use test::Bencher;

use water::get_time;
use water::Timespec;
use water::timespec::add;
use water::timespec::sub;
use water::timespec::NSINSEC;

use water::Net;
use std::thread::Thread;
use std::time::duration::Duration;

const BIGSIZE: uint = 1024;

pub struct BencherHack {
    iterations: u64,
    dur:        Duration,
    bytes:      u64,
}

#[bench]
fn pingpong_mpsc_water(b: &mut Bencher) {
    let h: &mut BencherHack = unsafe { transmute_copy(&b) };

    let start = get_time();
    pingpong_mpsc_water_run(4, 1000);
    let end = get_time();
    let dur = end - start;

    //println!("dur:{}", dur);

    h.iterations = 1000;
    h.dur = dur;
    h.bytes = 0;
}

#[bench]
fn pingpong_mpsc_native(b: &mut Bencher) {
    let h: &mut BencherHack = unsafe { transmute_copy(&b) };

    let start = get_time();
    pingpong_mpsc_native_run(4, 1000);
    let end = get_time();
    let dur = end - start;

    h.iterations = 1000;
    h.dur = dur;
    h.bytes = 0;
}

enum NativeFoo {
    Apple,
    Grape([u8;BIGSIZE]),
}

fn pingpong_mpsc_native_run(m: uint, n: uint) {
    // Create pairs of tasks that pingpong back and forth.
    fn run_pair(n: uint) {
        let (atx, arx) = channel::<NativeFoo>();
        let btx = atx.clone();

        let ta = Thread::spawn(move|| {
            for _ in range(0, n) {
                btx.send(NativeFoo::Apple);
            }
        });

        let tb = Thread::spawn(move|| {
            for _ in range(0, n) {
                atx.send(NativeFoo::Grape([0u8;BIGSIZE]));
            }
        });

       let tc = Thread::spawn(move|| {
            for _ in range(0, n * 2) {
                arx.recv();
            }
        });
    }

    for _ in range(0, m) {
        run_pair(n)
    }
}

struct FooApple;
struct FooGrape {
    field:  [u8;BIGSIZE],
}

fn pingpong_mpsc_water_run(m: uint, n: uint) {
    fn run_pair(n: uint) {
        let mut net = Net::new(100);
        let epa = net.new_endpoint();
        let epb = net.new_endpoint();
        let epa2 = epa.clone();
        let ta = Thread::spawn(move || {
            for _ in range(0, n) {
                epa.sendsynctype(FooApple);
            }
        });

        let tb = Thread::spawn(move || {
            for _ in range(0, n) {
                epa2.sendsynctype(FooGrape { field: [0u8;BIGSIZE] });
            }
        });

        let tc = Thread::spawn(move || {
            for x in range(0, n * 2) {
                epb.recvorblockforever();
            }
        });
    }

    for _ in range(0u, m) {
        run_pair(n);
    }
}