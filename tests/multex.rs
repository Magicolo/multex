use multex::*;

#[test]
fn cannot_lock_the_same_index_twice() {
    let multex = Multex8::new([1u8, 2u8, 3u8, 4u8]);
    let mut key = Key::new([0, 1, 0]);
    let guard = multex.lock_with(&mut key, false);
    assert_eq!(guard[0], Some(&mut 1u8));
    assert_eq!(guard[1], Some(&mut 2u8));
    assert_eq!(guard[2], None);
}

#[test]
fn can_lock_lots_of_indices() {
    let multex = MultexV::new((0..1000).collect::<Vec<_>>());
    let mut key = Key::new((0..1000).collect::<Vec<usize>>());
    let guard = multex.lock_with(&mut key, false);
    for (mut i, item) in guard.iter().enumerate() {
        assert_eq!(item, &Some(&mut i));
    }
}

#[test]
fn locks_different_indices() {
    let multex = Multex8::new([1u8, 2u8, 3u8, 4u8]);
    let mut key1 = Key::new([0]);
    let mut key2 = Key::new([1]);
    let mut guard1 = multex.lock_with(&mut key1, false);
    let mut guard2 = multex.lock_with(&mut key2, false);
    let Some(value1) = guard1[0].as_mut() else {
        panic!()
    };
    let Some(value2) = guard2[0].as_mut() else {
        panic!()
    };
    assert_eq!(**value1, 1u8);
    assert_eq!(**value2, 2u8);
}

#[test]
fn does_not_contend_on_out_of_bounds_indices() {
    let multex = Multex16::new([1u8, 2u8, 3u8, 4u8]);
    let mut key1 = Key::new([0, 4]);
    let mut key2 = Key::new([1, 4]);
    let _guard1 = multex.lock_with(&mut key1, false);
    let _guard2 = multex.lock_with(&mut key2, false);
}

#[test]
fn locks_all_without_panic() {
    Multex32::new(Vec::new()).lock().push(1);
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
