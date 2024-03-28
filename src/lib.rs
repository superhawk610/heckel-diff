#![allow(non_snake_case)]
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hasher};
use std::io::{BufRead, BufReader, Read};
use std::rc::Rc;

/// The number of times a line occurs in the old or new file. We only care
/// whether it:
///
/// - doesn't occur
/// - occurs exactly once
/// - occurs more than once
#[derive(Debug, PartialEq, Eq)]
enum Occurrences {
    Zero,
    One,
    Many,
}

impl Occurrences {
    fn increment(&mut self) {
        *self = match &self {
            Self::Zero => Self::One,
            _ => Self::Many,
        };
    }
}

/// An entry in the symbol table.
#[derive(Debug)]
struct SymbolEntry {
    /// The number of occurrences in the old file.
    OC: Occurrences,

    /// The number of occurrences in the new file.
    NC: Occurrences,

    /// The line number of the distinct occurrence in the old file (only
    /// set when the entry occurs exactly once in the old file).
    OLNO: Option<usize>,

    /// The line contents.
    line: String,
}

/// A symbol from either the old file (OA) or new file (NA). This will either
/// point to an entry in the symbol table, or the corresponding line in the
/// other file.
#[derive(Debug)]
enum Symbol {
    /// Reference to an entry in the symbol table.
    Entry(Rc<RefCell<SymbolEntry>>),

    /// Reference to the corresponding line in the other file.
    Reference(usize),
}

impl Symbol {
    fn as_entry_mut(&mut self) -> &mut Rc<RefCell<SymbolEntry>> {
        match self {
            Self::Entry(ref mut entry) => entry,
            _ => panic!("tried to access non-entry as entry"),
        }
    }
}

pub fn heckel_diff<O: Read, N: Read>(O: O, N: N) -> eyre::Result<()> {
    let O = BufReader::new(O);
    let N = BufReader::new(N);

    // Symbol table, representing distinct lines in the old and new file
    // and the number of occurrences in each.
    let mut symbols: HashMap<u64, Rc<RefCell<SymbolEntry>>> = HashMap::new();

    // Symbols contained in the old file.
    let mut OA: Vec<Symbol> = Vec::new();

    // Symbols contained in the new file.
    let mut NA: Vec<Symbol> = Vec::new();

    // first pass
    //
    // a) each line i of file N is read in sequence
    // b) a symbol table entry for each line i is created if it does not already exist
    // c) NC for the line's symbol table entry is incremented
    // d) NA[i] is set to point to the symbol table entry of line i
    for line in N.lines() {
        let line = line?;
        let hash = hash_str(&line);
        let sym = symbols
            .entry(hash)
            .and_modify(|sym| sym.borrow_mut().NC.increment())
            .or_insert_with(|| {
                Rc::new(RefCell::new(SymbolEntry {
                    OC: Occurrences::Zero,
                    NC: Occurrences::One,
                    OLNO: None,
                    line,
                }))
            });
        NA.push(Symbol::Entry(Rc::clone(sym)));
    }

    eprintln!("first pass ===\nsymbols\n{symbols:?}\n\nNA\n{NA:#?}\n");

    // second pass
    //
    // identical to the first pass, except we now act on O, OA, OC, and set OLNO
    for (line_num, line) in O.lines().enumerate() {
        let line = line?;
        let hash = hash_str(&line);
        let sym = symbols
            .entry(hash)
            .and_modify(|sym| {
                let mut sym = sym.borrow_mut();
                sym.OC.increment();
                sym.OLNO = Some(line_num);
            })
            .or_insert_with(|| {
                Rc::new(RefCell::new(SymbolEntry {
                    OC: Occurrences::One,
                    NC: Occurrences::Zero,
                    OLNO: Some(line_num),
                    line,
                }))
            });
        OA.push(Symbol::Entry(Rc::clone(sym)));
    }

    eprintln!("second pass ===\nsymbols\n{symbols:?}\n\nOA\n{OA:#?}\n");

    // third pass
    //
    // use observation 1 and process lines where NC = OC = 1; since each represents
    // (we assume) the same unmodified line, replace the symbol table pointers with
    // a reference to the line in the other file
    for (line_num, sym) in NA.iter_mut().enumerate() {
        let entry = sym.as_entry_mut().borrow();
        if entry.OC == Occurrences::One && entry.NC == Occurrences::One {
            let OLNO = entry.OLNO.unwrap();
            drop(entry);
            *sym = Symbol::Reference(OLNO);
            OA[OLNO] = Symbol::Reference(line_num);
        }
    }

    // add BEGIN lines
    OA.insert(0, Symbol::Reference(0));
    NA.insert(0, Symbol::Reference(0));

    // add END lines
    OA.push(Symbol::Reference(NA.len()));
    NA.push(Symbol::Reference(OA.len() - 1));

    eprintln!("third pass ===\nOA\n{OA:#?}\nNA\n{NA:#?}\n");

    // TODO: fourth pass
    // TODO: fifth pass
    // TODO: sixth pass

    Ok(())
}

fn hash_str(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(s.as_bytes());
    hasher.finish()
}
