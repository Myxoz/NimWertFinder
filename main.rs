use std::{
    collections::HashMap, io::{self, Write}, sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering}, Arc
    }, thread, time::Duration
};

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};

type SSIZE = primitive_types::U512;
const SNUM: usize = 512;

fn main() {
    let additionals = [0].into_iter().fold(SSIZE::zero(), |p,v| p | SSIZE::one() << v);
    // 0 is equal to [...], n
    // 0,2 is equal to [...], n, n+2
    // 0,4,6 is equal to [...], n, n+4, n+6
    let cal_size = SNUM;
    let cancel = Arc::new(AtomicBool::new(false));
    let control = Arc::new(AtomicUsize::new(0));
    let a = [4,6,15,17].into_iter().fold(SSIZE::zero(), |p,v| p | SSIZE::one() << v);
    // Cache here not per k-basis
    let mut cache = HashMap::new(); 
    println!("{}", get_left_numbers(a).expect("Provided list should not throw").d());
    println!("{:?}", get_nim_wert(a, &mut HashMap::new(), &cancel));
    // panic!("Early return");
    // input listener
    {
        let cancel = Arc::clone(&cancel);
        let control = Arc::clone(&control);
        thread::spawn(move || {
        // enable raw mode
        enable_raw_mode().expect("Failed to enable raw mode");
        loop {
            // poll for key events
            let cancel_clone = Arc::clone(&cancel);
            if event::poll(std::time::Duration::from_millis(100)).unwrap() {
                if let Event::Key(key_event) = event::read().unwrap() {
                    match key_event.code {
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            cancel_clone.store(true, Ordering::Relaxed);
                        }
                        KeyCode::Left => {
                            control.store(1, Ordering::Relaxed);
                            cancel_clone.store(true, Ordering::Relaxed);
                        }
                        KeyCode::Down => {
                            control.store(4, Ordering::Relaxed);
                            cancel_clone.store(true, Ordering::Relaxed);
                        }
                        KeyCode::Up => {
                            control.store(2, Ordering::Relaxed);
                            cancel_clone.store(true, Ordering::Relaxed);
                        }
                        KeyCode::Right => {
                            control.store(3, Ordering::Relaxed);
                            cancel_clone.store(true, Ordering::Relaxed);
                        }
                        KeyCode::Esc => {
                            println!("Exiting listener");
                            break;
                        }
                        KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                            std::process::exit(1);
                        }
                        _ => {}
                    }
                }
            }
        }
        // disable raw mode before exiting
        disable_raw_mode().expect("Failed to disable raw mode");
        });
    }

    let mut history = vec![2];
    let mut depth = 0;
    loop {
        cancel.store(false, Ordering::Relaxed);
        let mix_ggt = history.iter().cloned().reduce(|p, v| ggt(p, v)).unwrap_or(history[0]);
        let mut said = SSIZE::zero();
        for i in history.iter() {
            said |= SSIZE::one() << *i;
        }
        for k in 2..cal_size {
            // Cache here per k-basis
            // let mut cache = HashMap::new(); 
            let add =  additionals << k;
            if history.iter().any(|x| k % x == 0) {print!("{}, {:?}, -\n\r", history.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", "), add.d()); continue;}
            // We try to add a multiple of an existing number

            if ggt(k, mix_ggt) != 1 {print!("{}, {:?}, -\n\r", history.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", "), add.d()); continue;} 
            // Not coprime

            let said = said | add;
            let nim = get_nim_wert(said, &mut cache, &cancel);
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            print!("{}, {:?}, {}\n\r", history.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", "), add.d(), nim.map(|x| x.to_string()).unwrap_or(String::from("-1")));
            io::stdout().flush().unwrap();
        }
        while !cancel.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(100));
        }
        // Now we are done and we wanna do an action
        match control.load(Ordering::Relaxed) {
            1 => {
                if depth > 0 {
                    depth -= 1;
                    history.pop();
                }
            }
            2 => {
                let i = history.len()-1;
                if history[i] > 2 {
                    history[i] -= 1;
                }
            }
            3 => {
                depth += 1;
                history.push(2);
            }
            4 => {
                let i = history.len()-1;
                history[i] += 1;
            }
            _ => {}
        }
    }
}
fn get_nim_wert(said_numbers: SSIZE, memo: &mut HashMap<SSIZE, u32>, cancel: &AtomicBool) -> Option<u32> {
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    let mut nim_werte = SSIZE::zero();
    let left_numbers = get_left_numbers(said_numbers)?;
    let reduced = /* reduce_said_numbers(said_numbers); */ left_numbers;
    if let Some(&v) = memo.get(&reduced) {
        return Some(v);
    }
    if left_numbers.is_zero() {
        memo.insert(reduced, 0);
        return Some(0);
    }
    for i in left_numbers.iter_bits() {
        nim_werte |= SSIZE::one() << (get_nim_wert(said_numbers | (SSIZE::one() << i), memo, &cancel)?);
    }
    let mex = (!nim_werte).trailing_zeros();
    if mex >= (SNUM - 1) as u32 {
        return None;
    }
    memo.insert(reduced, mex);
    Some(mex)
}

fn ggt(mut a: usize, mut b: usize) -> usize{
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}
fn get_left_numbers(already_picked: SSIZE)->Option<SSIZE>{
    let modulo = already_picked.trailing_zeros() as usize;
    let mut konstruierbare_zahlen = SSIZE::zero();
    let mut mod_tracker = [SNUM + 1; SNUM];
    let mut highest = 0usize;
    let mut filled_mods = 0;

    'outer: for i in 0..SNUM {
        if i != 0 && (konstruierbare_zahlen & (SSIZE::one() << i)).is_zero() {continue};
        if filled_mods == modulo && i > highest {
            break 'outer;
        }
        // i ist 0 oder bereits konstruierbar
        for num in already_picked.iter_bits() {
            let sum = num + i;
            let sum_mod = sum % modulo;
            if !(konstruierbare_zahlen & (SSIZE::one() << sum)).is_zero() { continue };

            // Noch nicht konstruierbar
            konstruierbare_zahlen |= SSIZE::one() << sum;
            let pre_change = mod_tracker[sum_mod];
            if pre_change <= sum { continue };

            // Smaller way to solve mod
            mod_tracker[sum_mod] = sum;
            highest = *mod_tracker[0..modulo].iter().max().unwrap();
            // Didnt have a way to solve for mod
            if pre_change == SNUM + 1 {
                // Vorher keine Lösung bekannt
                filled_mods += 1;
            }
        }
    }
    if highest >= SNUM - 1 {
        return None;
    }
    let mask = (SSIZE::one() << highest)-1;
    Some((mask ^ SSIZE::from(3)) & !konstruierbare_zahlen)
}
pub struct NUMBERBitIter {
    value: SSIZE,
}

impl Iterator for NUMBERBitIter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.value.is_zero() {
            None
        } else {
            let idx = self.value.trailing_zeros() as usize;
            // Clear the lowest set bit
            self.value = self.value & (self.value - SSIZE::one());
            Some(idx)
        }
    }
}
pub trait NUMBERExt {
    fn iter_bits(self) -> NUMBERBitIter;
    fn d(&self) -> BitDisplay;
}
impl NUMBERExt for SSIZE {
    fn iter_bits(self) -> NUMBERBitIter {
        NUMBERBitIter { value: self }
    }
    fn d(&self) -> BitDisplay {
        BitDisplay(self.iter_bits().collect())
    }
}

pub struct BitDisplay(Vec<usize>);

impl std::fmt::Display for BitDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl std::fmt::Debug for BitDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
