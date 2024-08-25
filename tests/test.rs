

#[test]
fn main() {
    // let value0 = BerLength::new(47 as u64);
    // let value1 = BerLength::new(201 as u64);
    let value2 = BerLength::new(&123891829038102_u64);
    
    // assert_eq!(value0.encode(), vec![47]);
    println!("{:?}", value2.encode());
    // println!("{:?}", value0.encode()); // Should return [201]
    // println!("{:?}", value1.encode());  // Encoded long form of the number

    // let value3 = BerOid::new(23298 as u64);
    let value4 = BerOid::encode(&23298_u64);
    let value5 = BerOid::<u64>::decode(&mut value4.as_slice()).unwrap();
    println!("{:?}", value4);
    println!("{:?}", value5);
    // let value5 = value5.unwrap();

    // [129, 182, 2]
    // println!("{:?}", value4);

    // let encoded = value3.encode();
    // println!("{:?}", value3.encode());
}