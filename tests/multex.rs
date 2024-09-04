use multex::*;
use std::result;

type Result = result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn cannot_lock_the_same_index_twice() {
    if let Err(error) = Key::<usize, _>::new([0, 1, 0]) {
        assert_eq!(error.index, 0);
    } else {
        panic!();
    }
}

#[test]
fn can_lock_lots_of_indices() -> Result {
    let mut items = (0..1000).collect::<Vec<_>>();
    let multex = MultexV::new(&mut items);
    let indices = (0..1000).collect::<Vec<usize>>();
    let mut key = Key::new(indices.as_slice())?;
    let guard = multex.lock_with(&mut key, false);
    for (mut i, item) in guard.iter().enumerate() {
        assert_eq!(item, &Some(&mut i));
    }
    Ok(())
}

#[test]
fn locks_different_indices() -> Result {
    let multex = Multex8::new([1u8, 2u8, 3u8, 4u8]);
    let mut key1 = Key::new([0])?;
    let mut key2 = Key::new((1,))?;
    let mut guard1 = multex.lock_with(&mut key1, false);
    let mut guard2 = multex.lock_with(&mut key2, false);
    let [Some(value1)] = guard1.as_mut() else {
        panic!()
    };
    let (Some(value2),) = guard2.as_mut() else {
        panic!()
    };
    assert_eq!(**value1, 1u8);
    assert_eq!(**value2, 2u8);
    Ok(())
}

#[test]
fn locks_all_without_panic() {
    Multex32::new(Vec::new()).lock().push(1);
}

#[test]
fn locks_with_unit() -> Result {
    let multex = Multex8::new(());
    let mut key1 = Key::new([0, 3, 5, 7])?;
    let mut key2 = Key::new([2, 4, 5, 6])?;
    let guard1 = multex.try_lock_with(&mut key1, false);
    let guard2 = multex.try_lock_with(&mut key2, false);
    assert!(guard1.is_some());
    assert!(guard2.is_none());
    Ok(())
}

// #[test]
// fn boba() {
//     let multex = Multex8::new((1u8, 2u16));
//     let mut key1 = Key::new(At::<1>);
//     let mut key2 = Key::new((At::<1>, At::<0>));
//     let mut key3 = Key::new([0, 1]);
//     let mut key4 = Key::new((0, 1));
//     let mut guard1 = multex.lock_with(&mut key1, false);
//     let mut guard2 = multex.lock_with(&mut key2, false);
//     let mut guard3 = multex.lock_with(&mut key3, false);
//     let mut guard4 = multex.lock_with(&mut key4, false);
//     **guard1.as_mut().as_mut().unwrap() += 1;
//     **guard2.1.as_mut().unwrap() += 1;
//     match guard3[0].as_mut().unwrap() {
//         Or2::T0(_1) => **_1 += 1,
//         Or2::T1(_2) => **_2 += 2,
//     }
//     match guard4.0.as_mut().unwrap() {
//         Or2::T0(_1) => **_1 += 1,
//         Or2::T1(_2) => **_2 += 2,
//     }
// }

// fn fett() {
//     struct Boba(usize, String, Vec<usize>);
//     let mut boba1 = Boba(0, "".into(), vec![1, 2]);
//     let mut vector1 = boba1.2.iter_mut();
//     let multex1 = Multex64::new((
//         &mut boba1.0,
//         &mut boba1.1,
//         vector1.next().unwrap(),
//         vector1.next().unwrap(),
//     ));
//     let mut key1 = Key::new((1, 2));
//     let mut guard1 = multex1.lock_with(&mut key1, false);
//     if let (Some(Or4::T1(a)), Some(Or4::T2(b))) = &mut *guard1 {
//         a.push('a');
//         ***b += 1;
//     }

//     let mut boba2 = Boba(0, "".into(), vec![1, 2]);
//     let mut vector2 = boba2.2.iter_mut();
//     let multex2 = Multex8::new([
//         Or4::T0(&mut boba2.0),
//         Or4::T1(&mut boba2.1),
//         Or4::T2(vector2.next().unwrap()),
//         Or4::T3(vector2.next().unwrap()),
//     ]);
//     let mut key2 = Key::new((1, 2));
//     let mut guard2 = multex2.lock_with(&mut key2, false);
//     if let (Some(Or4::T1(a)), Some(Or4::T2(b))) = &mut *guard2 {
//         a.push('a');
//         **b += 1;
//     }
// }
