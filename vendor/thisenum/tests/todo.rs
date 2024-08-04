use thisenum::{
    Const,
    ConstEach,
};

#[derive(Debug, Const)]
#[armtype(u8)]
enum TestU8 {
    #[value = 0x7f]
    Arm1,
    #[value = 0x3B]
    Arm2,
}

#[derive(Debug, Const)]
#[armtype(&str)]
enum TestStr {
    #[value = "this"]
    Arm1,
    #[value = "that"]
    Arm2,
}

#[derive(Debug, Const)]
#[armtype(&[u8])]
enum TestU8Slice4 {
    #[value = b"\x7F\x7F\x7F\x7F\x67"]
    Arm1,
    #[value = b"\x3B\x3B\x3B\x3B"]
    Arm2,
}

// #[derive(Const)]
// #[arm_type(Vec<usize>)]
// enum TestVecu8 {
//     #[value = vec![1, 2, 3]]
//     Arm1,
//     #[value = vec![4, 5, 6]]
//     Arm2,
// }

#[derive(Debug, ConstEach)]
enum TestStrAny {
    #[armtype(u8)]
    #[value = 0xAA]
    Arm1,
    #[value = "test3"]
    Arm2,
}

fn main() {

    // main2();
    main3();

    // // Const example
    // assert_eq!(TestU8::Arm1.value(), &0x7F);
    // assert_eq!(TestU8::Arm1, 0x7F as u8);
    // assert_eq!(TestU8::Arm2.value(), &0x3B);

    // // Const example 2
    // assert_eq!(TestStr::Arm1.value(), "this");
    // assert_eq!(TestStr::Arm1, "this");
    // assert_eq!(TestStr::Arm2.value(), "that");

    // // Const example 3
    // assert_eq!(TestU8Slice4::Arm1.value(), b"\x7F\x7F\x7F\x7F\x67");
    // assert_eq!(TestU8Slice4::Arm2.value(), b"\x3B\x3B\x3B\x3B");
    // assert_eq!(TestU8Slice4::Arm1, b"\x7F\x7F\x7F\x7F\x67" as &[u8]);

    // // ConstEach example
    // assert!(TestStrAny::Arm1.value::<u8>().is_some());
    // let val = TestStrAny::Arm1.value::<u8>().unwrap();
    // println!("TestStrAny::Arm1.value() = {:?}", val);
    // println!("TestStrAny::Arm1.value() = {:?}", TestStrAny::Arm1.value::<u8>().unwrap());
    // let value = TestStrAny::Arm2.value::<Vec<f32>>();
    // println!("TestStrAny::Arm2.value() = {:?}", value);
    // assert!(TestStrAny::Arm2.value::<Vec<f32>>().is_none());
    // assert!(TestStrAny::Arm2.value::<&str>().is_some());
}

// #[derive(ConstEach, Debug)]
// enum MyEnum {
//     #[armtype(u8)]
//     #[value = 0xAA]
//     A,
//     #[value = "test3"]
//     B,
// }

// #[derive(ConstEach, Debug)]
// enum Tags {
//     #[value = b"\x00\x01"]
//     Key,
//     #[armtype(u16)]
//     #[value = 24250]
//     Length,
//     #[armtype(&[u8])]
//     #[value = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f"]
//     Data,
// }

// fn main2() {
//     // [`ConstEach`] examples
//     assert!(MyEnum::A.value::<u8>().is_some());
//     assert!(MyEnum::A.value::<Vec<f32>>().is_none());
//     assert!(MyEnum::B.value::<u8>().is_none());
//     assert!(MyEnum::B.value::<&str>().is_some());

//     // An infered type. This will be as strict as possible,
//     // therefore [`&[u8]`] will fail but [`&[u8; 2]`] will succeed
//     assert!(Tags::Key.value::<&[u8; 2]>().is_some());
//     assert!(Tags::Key.value::<&[u8; 5]>().is_none());
//     assert!(Tags::Key.value::<&[u8]>().is_none());
//     assert!(u16::from_le_bytes(**Tags::Key.value::<&[u8; 2]>().unwrap()) == 0x0100);

//     // casting as anything other than the defined / inferred type will
//     // fail, since this uses [`downcast_ref`] from [`std::any::Any`]
//     assert!(Tags::Length.value::<u16>().is_some());
//     assert!(Tags::Length.value::<u32>().is_none());
//     assert!(Tags::Length.value::<u64>().is_none());

//     // however, can always convert to a different type
//     // after value is successfully acquired
//     assert!(*Tags::Length.value::<u16>().unwrap() as u32 == 24250);
// }

// use enum_const::Const;

#[derive(Const, Debug)]
#[armtype(i32)]
enum MyEnum {
    #[value = 0]
    A,
    #[value = 1]
    B,
}

#[derive(Const, Debug)]
#[armtype(&[u8])]
enum Tags {
    #[value = b"\x00\x01\x7f"]
    Key,
    #[value = b"\xba\x5e"]
    Length,
    #[value = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f"]
    Data,
}

// use enum_const::Const;

#[derive(Const, Debug)]
#[armtype(&[u8])]
/// https://exiftool.org/TagNames/EXIF.html
enum ExifTag {
    // ...
    #[value = b"\x01\x00"]
    ImageWidth,
    #[value = b"\x01\x01"]
    ImageHeight,
    #[value = b"\x01\x02"]
    BitsPerSamole,
    #[value = b"\x01\x03"]
    Compression,
    #[value = b"\x01\x06"]
    PhotometricInterpretation,
    // ...
}

#[derive(ConstEach, Debug)]
enum CustomEnum {
    #[armtype(&[u8])]
    #[value = b"\x01\x00"]
    A,
    #[value = "foo"]
    B,
    #[armtype(f32)]
    #[value = 3.14]
    C,
}

fn main3() {
    // // it's prefered to use the function call to `value` 
    // // to get a [`&'static T`] reference to the value
    // assert_eq!(MyEnum::A.value(), &0);
    // assert_eq!(MyEnum::B.value(), &1);
    // assert_eq!(Tags::Key.value(), b"\x00\x01\x7f");
    // assert_eq!(Tags::Length.value(), b"\xba\x5e");
    // assert_eq!(Tags::Data.value(), b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f");

    // // can also check equality without the function call. This must compare the input 
    // // type defined in `#[armtype = ...]`
    // assert_eq!(Tags::Length, b"\xba\x5e");

    assert_eq!(ExifTag::ImageWidth.value(), b"\x01\x00");
    assert_eq!(ExifTag::ImageWidth, b"\x01\x00");
    println!("ExifTag::ImageWidth.value() = {:?}", ExifTag::ImageWidth.value());
    
    assert_eq!(CustomEnum::A.value::<&[u8]>().unwrap(), b"\x01\x00");
    assert!(CustomEnum::B.value::<&str>().is_some());
    assert_eq!(CustomEnum::B.value::<&str>().unwrap(), &"foo");
    assert_eq!(CustomEnum::B.value::<&str>(), Some("foo").as_ref());
    assert_eq!(CustomEnum::C.value::<f32>(), Some(3.14).as_ref());
    // or on failure
    assert!(CustomEnum::C.value::<i32>().is_none());
}