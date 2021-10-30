// Copyright 2017 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

//! Summarize the structure of a CASA table.

use clap::{App, Arg};
use rubbl_casatables::{Table, TableOpenMode};
use rubbl_core::{ctry, notify::ClapNotificationArgsExt, Error};
use std::{cmp::max, path::Path, process};
use aoflagger_sys::cxx_aoflagger_new;
use std::os::raw::c_short;

fn main() {
    let matches = App::new("tableinfo")
        .version("0.1.0")
        .rubbl_notify_args()
        .arg(
            Arg::with_name("IN-TABLE")
                .help("The path of the input data set")
                .required(true)
                .index(1),
        )
        .get_matches();

    let mut major: c_short = -1;
    let mut minor: c_short = -1;
    let mut sub_minor: c_short = -1;

    unsafe {
        let aoflagger = cxx_aoflagger_new();
        aoflagger.GetVersion(&mut major, &mut minor, &mut sub_minor);
    }

    process::exit(rubbl_core::notify::run_with_notifications(
        matches,
        |matches, _nbe| -> Result<i32, Error> {
            let inpath = Path::new(matches.value_of_os("IN-TABLE").unwrap()).to_owned();

            let mut t = ctry!(Table::open(&inpath, TableOpenMode::Read);
                          "failed to open input table \"{}\"", inpath.display());

            println!("Table \"{}\":", inpath.display());
            println!("Number of rows: {}", t.n_rows());
            println!("Number of columns: {}", t.n_columns());
            println!("aoflagger version: {}.{}.{}", major, minor, sub_minor);

            Ok(0)
        },
    ));
}
