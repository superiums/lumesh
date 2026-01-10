# Lumesh

a lighting shell with modern expierence.

## install

there're serial ways to complete the install process.

1. just run `bash -c install.sh`

2. manually:
- copy main binary file: `lume` to your path `/usr/bin`
- create link: `ln -s /usr/bin/lume /usr/bin/lumesh`
- copy completion files: `cp completions /usr/share/lumesh/`

3. from source
- from hub: `git clone 'https://github.com/superiums/lumesh'`
  `cargo build -r`
- from cargo: `cargo install lumesh`

## about completions

all these completions was converted from fish.

you could use the scrip in `mod` to convert by yourself.

there're two folders after convert:

- `completions`: ready to use.
- `unprocessed`: need manual edit to become usable.

*note there're some files named `*_1_n.csv`, as there're sevral editon of the same cmd, so I renamed it, and left it there for your choose.*

