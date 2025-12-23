# mspacman
A simple tool to view dependencies of pacman installed packages in Arch linux and derivatives

## Requires
- pacman to be installed

## Install
cargo install mspacman

## Run
$ ms


## Features
- view dependencies of pacman installed packages
- view parent and child dependencies
- jump around dependencies and follow dependency chain
- view explicitly installed packages, all packages, and available updates
- sort by various fields, such as name, size, install date
- filter by name, explicitly installed, orphans, foreign installed
- view the files that is provided by a package
- run commands on selected packages: remove, update
- sync pacman database

![Screenshot](Screenshot.png)
