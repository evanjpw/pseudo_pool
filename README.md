# pseudo_pool

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](
https://github.com/evanjpw/pseudo_pool)
[![Cargo](https://img.shields.io/crates/v/pseudo_pool.svg)](
https://crates.io/crates/pseudo_pool)
[![Documentation](https://docs.rs/pseudo_pool/badge.svg)](
https://docs.rs/pseudo_pool)
![Build](
https://github.com/evanjpw/pseudo_pool/actions/workflows/rust.yml/badge.svg)

_A pool-like collection that automatically returns objects to the pool & blocks when the pool is empty_

## Motivation

I needed something that worked like a pool, that allowed me to checkout
objects in the pool & then return them, but it needed to have the following
characteristics:
* It needed to automatically return things to the pool when dropped
* It needed to have a non-blocking way to do checkout that returned an object or None if all objects are in use
* **IT NEEDED TO ALSO HAVE A BLOCKING CHECKOUT THAT BLOCKS UNTIL THERE IS A USABLE OBJECT**

The first two were covered by existing crates:
* [lockfree-object-pool](https://crates.io/crates/lockfree-object-pool)
* [object-pool](https://crates.io/crates/object-pool)
* [opool](https://crates.io/crates/opool)
* [lifeguard](https://github.com/zslayton/lifeguard?tab=readme-ov-file)
* [pool](https://crates.io/crates/pool)

These are _nice_ crates made by _responsible_ people, & not _absurd hacks_
like this crate.

However, none of them had characteristic three.

So I wrote this thing.

## Usage

```
[dependencies]
pseudo_pool = "0.1.0" # A version number that inspires confidence
```

## Examples

TBD

## Benchmarks

No.

## License

[MIT](https://opensource.org/license/mit)
