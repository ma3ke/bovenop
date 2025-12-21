# _bovenop_&mdash;listen and watch processes by name

Observe memory, cpu, and disk I/O for processes matching the provided name.

![bovenop showing a number of wilted processes followed by some active processes][image]

This program is helpful when running a number of processes you want to keep
track of as they run and end. It was designed to keep a basic overview of the
memory peak and evolution of the same program through a number of small tweaks
and changes.

When programs are finished, they become 'wilted': their entries are dimmed
and collapsed into condensed representations.

See also [my thread][thread] about this program.

## Installation

You know the drill, probably, but you can install it with cargo.

```console
cargo install bovenop
```

## Usage

Open _bovenop_ and listen for processes with some name.

```console
bovenop <program-name>
```

Within the interface, the following controls are currently available.

- To clear and reset all entries, press `r`. 
- Use `C` and `E` to collapse and expand all entries, respectively. 
- Exit with `^C` or `q`.

## Future work and contributions

There is a lot that could be improved, but it does the basic job I created it
for. 

- I think simple navigation and selection-based collapsing/expanding
  (lowercase `c` and `e`) would be pleasant.
- Scrolling through longer listes of processes.
- Selecting between chronological and reverse-chronological representations
  would be useful, especially if new processes slowly push old ones outside
  of the scroll view.
- An explicit marker for dead processes would be good. The current way of
  showing wilted processes is through dimming, which is not very visible under
  some circumstances.
- Rudimentary color selection support to provide better visibility for other
  users' needs.
- A marker showing the highest recorded peak memory, for example, could be
  useful.
- Regular expression matching on process names.
- The ability to provide a list of accepted process names.
- Flags for setting the width and presence or absence of some categories. For
  instance, you may want to just see memory and CPU information, with an extra
  wide memory chart.

It must be noted, however, that none of these features may become a reality.
This is just a tool I wrote for myself to do a particular thing, after all :)

### Contributions

I very much welcome input from people based actual needs in the form of issues,
emails, or fedi posts. As with many of my projects, I appreciate and welcome
code contributions, provided we have had some prior discussion about it where
we briefly discuss the design of any non-trivial changes. Additional contact
options can be found on my site.

## License

This project is licensed under GPL-3.0, see LICENSE.

[image]: bovenop.png
[thread]: https://hachyderm.io/@ma3ke/115757953511991333
