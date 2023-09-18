mod key;
mod lock;
mod multex;
mod system;

pub use key::{
    Index, Key, One1, One10, One11, One12, One13, One14, One15, One16, One2, One3, One4, One5,
    One6, One7, One8, One9,
};
pub use lock::IndexError;
pub use multex::{
    Guard, Multex, Multex16, Multex16N, Multex32, Multex32N, Multex64, Multex64N, Multex8,
    Multex8N, MultexN,
};

/*
    TODO:
    - Use references for 'mask: Self'
*/

#[test]
fn locks_different_indices() -> Result<(), Box<dyn std::error::Error>> {
    let multex = Multex8::new([1u8, 2u8, 3u8, 4u8]);
    let key1 = Key::new([0])?;
    let key2 = Key::new([1])?;
    let mut guard1 = multex.lock_with(&key1, false);
    let mut guard2 = multex.lock_with(&key2, false);
    let Some(value1) = guard1[0].as_mut() else {
        panic!()
    };
    let Some(value2) = guard2[0].as_mut() else {
        panic!()
    };
    assert_eq!(**value1, 1u8);
    assert_eq!(**value2, 2u8);
    Ok(())
}

// #[test]
// fn does_not_contend_on_out_of_bounds_indices() -> Result<(), Box<dyn Error>> {
//     let multex = Multex16::new([1u8, 2u8, 3u8, 4u8]);
//     let key1 = Indices::new([0, 4])?;
//     let key2 = Indices::new([1, 4])?;
//     let _guard1 = multex.lock_with(&key1, false);
//     let _guard2 = multex.lock_with(&key2, false);
//     Ok(())
// }

#[test]
fn locks_all_without_panic() {
    Multex32::new(Vec::new()).lock().push(1);
}

#[test]
fn boba() {
    let multex = Multex8::new((1u8, 2u16));
    let key1 = Key::new(Index::<1>).unwrap();
    let key2 = Key::new((Index::<1>, Index::<0>)).unwrap();
    let key3 = Key::new([0, 1]).unwrap();
    let key4 = Key::new((0, 1)).unwrap();
    let mut guard1 = multex.lock_with(&key1, false);
    let mut guard2 = multex.lock_with(&key2, false);
    let mut guard3 = multex.lock_with(&key3, false);
    let mut guard4 = multex.lock_with(&key4, false);
    **guard1.as_mut().unwrap() += 1;
    **guard2.1.as_mut().unwrap() += 1;
    match guard3[0].as_mut().unwrap() {
        One2::T0(_1) => **_1 += 1,
        One2::T1(_2) => **_2 += 2,
    }
    match guard4.0.as_mut().unwrap() {
        One2::T0(_1) => **_1 += 1,
        One2::T1(_2) => **_2 += 2,
    }
}

fn fett() {
    struct Boba(usize, String, Vec<usize>);
    let mut boba1 = Boba(0, "".into(), vec![1, 2]);
    let mut vector1 = boba1.2.iter_mut();
    let multex1 = Multex64::new((
        &mut boba1.0,
        &mut boba1.1,
        vector1.next().unwrap(),
        vector1.next().unwrap(),
    ));
    let key1 = Key::new((1, 2)).unwrap();
    let mut guard1 = multex1.lock_with(&key1, false);
    if let (Some(One4::T1(a)), Some(One4::T2(b))) = &mut *guard1 {
        a.push('a');
        ***b += 1;
    }

    let mut boba2 = Boba(0, "".into(), vec![1, 2]);
    let mut vector2 = boba2.2.iter_mut();
    let multex2 = Multex8::new([
        One4::T0(&mut boba2.0),
        One4::T1(&mut boba2.1),
        One4::T2(vector2.next().unwrap()),
        One4::T3(vector2.next().unwrap()),
    ]);
    let key2 = Key::new((1, 2)).unwrap();
    let mut guard2 = multex2.lock_with(&key2, false);
    if let (Some(One4::T1(a)), Some(One4::T2(b))) = &mut *guard2 {
        a.push('a');
        **b += 1;
    }
}
