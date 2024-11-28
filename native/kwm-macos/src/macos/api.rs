use bitflags::bitflags;



#[no_mangle]
pub extern "C" fn add_numbers(x: i32, y: i32, s: SomeStruct) -> i32 {
    return x + y
}


bitflags! {
    #[repr(transparent)]
    pub struct SomeStruct: u32 {
        const A = 1;
        const B = 2;
    }
}