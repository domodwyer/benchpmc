#[macro_use]
extern crate clap;
extern crate ansi_term;
extern crate nix;
#[cfg(target_os = "freebsd")]
extern crate pmc;
extern crate separator;

mod error;
mod event;
mod runner;

#[cfg(all(debug_assertions, not(target_os = "freebsd")))]
use event::MockEvent;
#[cfg(target_os = "freebsd")]
use event::{PmcEvent, RSDPrinter, RelativePrinter};

use ansi_term::Colour::Yellow;
use clap::{App, AppSettings, Arg};
use runner::Counter;
use std::fmt::Display;
use std::process;
use std::time::Instant;

/// `DisplayCounter` composes the traits required to both run, and display a
/// counter
trait DisplayCounter: Counter + Display {}
impl<T: Counter + Display> DisplayCounter for T {}

fn main() {
    let matchers = App::new("benchpmc")
        .setting(AppSettings::AllowLeadingHyphen)
        .author(crate_authors!())
        .version(crate_version!())
        .template("{bin} {version} - {author}\n{about}\n\n{usage}\n{unified}\n\n{after-help}")
        .about("Benchmark targets using Intel/AMD CPU performance counters.")
        .version_short("v")
        .arg(
            Arg::with_name("target")
                .help("Executable to profile")
                .index(1)
                .required(true),
        )
        .arg(
            Arg::with_name("event-spec")
                .short("e")
                .long("event")
                .takes_value(true)
                .multiple(true)
                .help("One or more events to measure"),
        )
        .arg(
            Arg::with_name("count")
                .short("n")
                .long("count")
                .takes_value(true)
                .multiple(false)
                .default_value("10")
                .help("Number of times to measure target"),
        )
        // TODO: write samples to an outdir for further processing / graphing
        //
        // .arg(
        //     Arg::with_name("outdir")
        //         .short("o")
        //         .long("output")
        //         .takes_value(true)
        //         .multiple(false)
        //         .help("Output directory to write raw measurement values"),
        // )
        .arg(Arg::with_name("args").takes_value(true).multiple(true))
        .after_help(
            "\
Uses the libpmc userland interface for hpwmc to measure performance counters on 
supported CPUs - see hwpmc(4) for supported devices. Your kernel must have been 
compiled with hwpmc support, or the module loaded at runtime.

Event specifiers are passed through to libpmc unchanged, therefore any valid event 
specifier can be used (such as UOPS_RETIRED.ALL), including event qualifiers (such 
as setting the cmask, or filtering events by CPU privilege level).

See the pmc manpage for your CPU type for more information (i.e. pmc.haswell(3) 
for the Intel Haswell microarchitecture) - try running 'apropos pmc.'

If count is > 1, the average value is printed along with the relative standard 
deviation for observed counter values. Only per-process events are supported.",
        )
        .get_matches();

    let run_count = matchers
        .value_of("count")
        .expect("failed to get --count")
        .parse::<isize>()
        .unwrap_or_else(|_| {
            println!("Failed to parse --count, using default value");
            10
        });

    let mut args = vec![];
    if let Some(target_args) = matchers.values_of("args") {
        for arg in target_args {
            args.push(arg);
        }
    }
    let args = args; // drop mutability

    let target = matchers.value_of("target").unwrap();

    let counters = get_counters(&matchers);
    if let Err(err) = counters {
        println!("there was a problem with {}", err);
        process::exit(-1);
    }
    let mut counters = counters.unwrap();

    let prompt = Yellow.bold().paint("==> ");
    println!(
        "{} running {} '{}' with args {:?} ",
        prompt, run_count, target, args
    );

    for i in 0..run_count {
        let mut runner = runner::Runner::new(target).args(&args);

        let start = Instant::now();
        if let Some(err) = runner.run(&mut counters).err() {
            println!("failed to run benchmark: {}", err);
            process::exit(-1);
        }

        let diff = start.elapsed();
        let ms = (diff.as_secs() * 1000) + u64::from(diff.subsec_nanos() / 1_000_000);

        let progress = Yellow.paint(format!("[{}/{}]", i + 1, run_count));
        println!("{}{}\truntime: {}ms", prompt, progress, ms);
    }

    println!("\n");
    for c in counters {
        println!("{}", c);
    }
}

#[cfg(not(target_os = "freebsd"))]
fn get_counters<'a>(
    _matchers: &'a clap::ArgMatches<'a>,
) -> Result<Vec<Box<dyn DisplayCounter + 'a>>, String> {
    Ok(vec![Box::new(MockEvent::new("mock", 42))])
}

#[cfg(target_os = "freebsd")]
fn get_counters<'a>(
    matchers: &'a clap::ArgMatches<'a>,
) -> Result<Vec<Box<DisplayCounter + 'a>>, String> {
    let mut counters: Vec<Box<DisplayCounter>> = vec![];

    // Allocate user specified events
    if matchers.is_present("event-spec") {
        for event in matchers.values_of("event-spec").unwrap() {
            counters.push(Box::new(RSDPrinter::new(
                PmcEvent::new(event).map_err(|e| format!("{}: {}", event, e))?,
            )));
        }

        return Ok(counters);
    }

    let instructions =
        PmcEvent::new("instructions").map_err(|e| format!("initialising counter: {}", e))?;

    // Otherwise use the defaults
    let defaults = [
        ("RESOURCE_STALLS.ANY", "resource-stalls"),
        ("BR_INST_RETIRED.ALL_BRANCHES", "speculated-good"),
        ("BR_MISP_RETIRED.ALL_BRANCHES", "speculated-bad"),
        ("PAGE_FAULT.READ", "page-fault-read"),
        ("PAGE_FAULT.WRITE", "page-fault-write"),
    ];

    let mut comparators = vec![];
    for &(event, alias) in &defaults {
        if let Ok(counter) = PmcEvent::new(event)
            .map_err(|e| println!("{}: {}", event, e))
            .map(|c| c.alias(alias))
        {
            comparators.push(Box::new(RSDPrinter::new(counter)));
        }
    }

    // Push the instructions counter, along with all the default comparators
    // (which are expressed as a relative of instructions)
    counters.push(Box::new(RelativePrinter::new(
        RSDPrinter::new(instructions),
        comparators,
    )));

    // Attempt to allocate and push the cache counters
    if let Ok(refs) = PmcEvent::new("LONGEST_LAT_CACHE.REFERENCE")
        .map_err(|e| println!("LONGEST_LAT_CACHE.REFERENCE: {}", e))
        .map(|c| c.alias("cache-references"))
    {
        // Wrap the cache references in a RSDPrinter
        let refs = RSDPrinter::new(refs);

        // Attempt to build a relative pair
        let counter: Box<DisplayCounter> = match PmcEvent::new("LONGEST_LAT_CACHE.MISS") {
            Ok(misses) => Box::new(RelativePrinter::new(
                refs,
                vec![Box::new(RSDPrinter::new(misses.alias("cache-misses")))],
            )),
            Err(e) => {
                // Push the successful refs counter only
                println!("LONGEST_LAT_CACHE.MISS: {}", e);
                Box::new(refs)
            }
        };

        counters.push(counter);
    }

    Ok(counters)
}
