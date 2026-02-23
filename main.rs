use std::{
    collections::HashMap, fs::File, io::{self, Write}, sync::{
        atomic::{AtomicBool, Ordering}, Arc, Mutex
    }, thread, time::Duration
};

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};

type SSIZE = primitive_types::U512;
const SNUM: usize = 512;

fn main() {
    // 0 is equal to [...], n
    // 0,2 is equal to [...], n, n+2
    // 0,4,6 is equal to [...], n, n+4, n+6
    let cal_size = SNUM;
    let a = [9,11,13].into_iter().fold(SSIZE::zero(), |p,v| p | SSIZE::one() << v);
    // Cache here not per k-basis
    let mut cache = HashMap::new(); 
    // Pipes resulting nim-values and aditionals to result.nim_values
    let graph_path = "result.nim_values";
    let creating_graph = true;
    println!("{}", get_left_numbers(a).expect("Provided list should not throw").d());

    let cancel = Arc::new(AtomicBool::new(false));
    let additionals_mode = Arc::new(AtomicBool::new(false));
    let additionals_vec = Arc::new(Mutex::new(vec![0]));
    let history_vec = Arc::new(Mutex::new(vec![2]));

    println!("{:?}", get_nim_wert(a, &mut HashMap::new(), &cancel));

    {
        let cancel = Arc::clone(&cancel);
        let additionals_mode = Arc::clone(&additionals_mode);
        let additionals_vec = Arc::clone(&additionals_vec);
        let history_vec = Arc::clone(&history_vec);
        thread::spawn(move || {
        // enable raw mode
        enable_raw_mode().expect("Failed to enable raw mode");
        loop {
            // poll for key events
            let cancel_clone = Arc::clone(&cancel);
            if event::poll(std::time::Duration::from_millis(100)).unwrap() {
                if let Event::Key(key_event) = event::read().unwrap() {
                    match key_event.code {
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            let new_state = !additionals_mode.load(Ordering::Relaxed);
                            additionals_mode.store(new_state, Ordering::Relaxed);
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') => { // Skip
                            cancel_clone.store(true, Ordering::Relaxed);
                        }
                        KeyCode::Left => {
                            if additionals_mode.load(Ordering::Relaxed) {
                                let mut add = additionals_vec.lock().unwrap();
                                if add.len() > 1 { add.pop(); }
                            } else {
                                let mut hist = history_vec.lock().unwrap();
                                if hist.len() > 1 { hist.pop(); }
                            }
                            cancel_clone.store(true, Ordering::Relaxed);
                        }
                        KeyCode::Up => {
                            if additionals_mode.load(Ordering::Relaxed) {
                                let mut add = additionals_vec.lock().unwrap();
                                if let Some(last) = add.last_mut() {
                                    if *last > 0 { *last -= 1; }
                                }
                            } else {
                                let mut hist = history_vec.lock().unwrap();
                                if let Some(last) = hist.last_mut() {
                                    if *last > 2 { *last -= 1; }
                                }
                            }
                            cancel_clone.store(true, Ordering::Relaxed);
                        }
                        KeyCode::Down => {
                            if additionals_mode.load(Ordering::Relaxed) {
                                let mut add = additionals_vec.lock().unwrap();
                                if let Some(last) = add.last_mut() { *last += 1; }
                            } else {
                                let mut hist = history_vec.lock().unwrap();
                                if let Some(last) = hist.last_mut() { *last += 1; }
                            }
                            cancel_clone.store(true, Ordering::Relaxed);
                        }
                        KeyCode::Right => {
                            if additionals_mode.load(Ordering::Relaxed) {
                                let mut add = additionals_vec.lock().unwrap();
                                add.push(0);
                            } else {
                                let mut hist = history_vec.lock().unwrap();
                                let new_val = *hist.last().unwrap_or(&2);
                                hist.push(new_val);
                            }
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

    loop {
        cancel.store(false, Ordering::Relaxed);
        let mut output_file = if creating_graph { Some(File::create(graph_path).expect(&format!("Should be able to write to {}", graph_path))) } else { None };
        let history = {
            history_vec.lock().unwrap().clone()
        };
        let mix_ggt = history.iter().cloned().reduce(|p, v| ggt(p, v)).unwrap_or(history[0]);
        let mut said = SSIZE::zero();
        for i in history.iter() {
            said |= SSIZE::one() << *i;
        }
        let additionals = {
            let add_vec = additionals_vec.lock().unwrap();
            add_vec.iter()
                .fold(SSIZE::zero(), |p, &v| p | SSIZE::one() << v)
        };
        for k in 2..cal_size {
            // Cache here per k-basis
            // let mut cache = HashMap::new(); 
            let add =  additionals << k;
            if history.iter().any(|x| k % x == 0) {print!("{}, {:?}, -\n\r", history.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", "), add.d()); continue;}
            // We try to add a multiple of an existing number

            if ggt(k, mix_ggt) != 1 {print!("{}, {:?}, -\n\r", history.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", "), add.d()); continue;} 
            // Not coprime

            let left = get_left_numbers_raw(said).0;
            if add & left != add {
                // The picking the numbers is not an option
                print!("{}, {:?}, --\n\r", history.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", "), add.d());
                continue;
            }
            let said = said | add;
            let nim = get_nim_wert(said, &mut cache, &cancel);
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let form_nim = nim.map(|x| x.to_string()).unwrap_or(String::from("-1"));
            print!("{}, {:?} ({:?}), {}\n\r", history.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", "), add.d(), additionals.d(), form_nim);
            if let Some(file) = &mut output_file && let Some(left_num) = get_left_numbers(said) {
                let _ = writeln!(
                    file,
                    "{};{};{{{}}}",
                    said.d(),
                    form_nim,
                    left_num.iter_bits()
                        .map(|x| {
                            let new_said_numbers = said | SSIZE::one() << x;
                            let nim_wert = get_nim_wert(new_said_numbers, &mut cache, &cancel)?;
                            let new_left_num = get_left_numbers(new_said_numbers)?;
                            Some(format!("\"{}\": [{},{}]", x, nim_wert, new_left_num.d()))
                        })
                        .flatten()
                        .collect::<Vec<String>>()
                        .join(", ")
                );
            }
            io::stdout().flush().unwrap();
        }
        while !cancel.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(100));
        }
        cancel.store(false, Ordering::Relaxed);
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
fn get_left_numbers_raw(already_picked: SSIZE)->(SSIZE, bool){
    let modulo = already_picked.trailing_zeros() as usize;
    let mut konstruierbare_zahlen = SSIZE::zero();
    let mut mod_tracker = [SNUM + 1; SNUM];
    let mut highest = 0usize;
    let mut filled_mods = 0;

    'outer: for i in 0..SNUM {
        if i != 0 && !konstruierbare_zahlen.bit(i) {continue};
        if filled_mods == modulo && i > highest {
            break 'outer;
        }
        // i ist 0 oder bereits konstruierbar
        for num in already_picked.iter_bits() {
            let sum = num + i;
            let sum_mod = sum % modulo;
            if sum > SNUM - 1 || konstruierbare_zahlen.bit(sum) { continue };

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
        return (!konstruierbare_zahlen, false);
    }
    let mask = (SSIZE::one() << highest)-1;
    return ((mask ^ SSIZE::from(3)) & !konstruierbare_zahlen, true);
}
fn get_left_numbers(already_picked: SSIZE)->Option<SSIZE>{
    let res = get_left_numbers_raw(already_picked);
    return if res.1 {Some(res.0)} else {None};
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
